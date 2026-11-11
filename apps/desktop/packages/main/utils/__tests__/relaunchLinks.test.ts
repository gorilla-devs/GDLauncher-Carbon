import { describe, expect, it, vi } from "vitest"
import { ProtocolQueue, type ProtocolDelivery } from "../protocolQueue"
import { relaunchWithoutHandledLinks } from "../relaunchLinks"

function wrapped(argv: string[], pending: string[]) {
  const relaunch = vi.fn()
  const app = { relaunch }
  relaunchWithoutHandledLinks(
    app,
    () => argv,
    () => pending
  )
  return { app, relaunch }
}

describe("restart after a deep link started the launcher", () => {
  it("a restart leaves out a deep link the App has acted on and keeps every other argument", () => {
    const cases = [
      {
        argv: ["C:\\GDLauncher\\GDLauncher.exe", "gdlauncher://share/ABC"],
        args: []
      },
      {
        argv: [
          "/opt/GDLauncher/gdlauncher",
          "--runtime_path",
          "/data",
          "curseforge://install?addonId=1&fileId=2"
        ],
        args: ["--runtime_path", "/data"]
      },
      {
        argv: ["/electron", "/repo/apps/desktop", "modrinth://mod/sodium"],
        args: ["/repo/apps/desktop"]
      }
    ]
    for (const { argv, args } of cases) {
      const { app, relaunch } = wrapped(argv, [])
      app.relaunch()
      expect(relaunch).toHaveBeenCalledTimes(1)
      expect(relaunch.mock.calls[0]).toEqual([{ args }])
    }
  })

  it("a restart keeps a deep link the App has not acted on yet, so the startup error screen's Restart still opens it", () => {
    const link = "gdlauncher://share/ABC"
    const { app, relaunch } = wrapped(
      ["/opt/GDLauncher/gdlauncher", link],
      [link]
    )
    app.relaunch()
    expect(relaunch).toHaveBeenCalledTimes(1)
    expect(relaunch.mock.calls[0]).toEqual([])
  })

  it("a restart of a launcher started without a deep link keeps Electron's default", () => {
    const { app, relaunch } = wrapped(
      [
        "/opt/GDLauncher/gdlauncher",
        "--gdl_api_url=https://example.invalid",
        "--runtime_path",
        "/data"
      ],
      []
    )
    app.relaunch()
    expect(relaunch).toHaveBeenCalledTimes(1)
    expect(relaunch.mock.calls[0]).toEqual([])
  })

  it("a restart that passes its own options is passed on unchanged", () => {
    const { app, relaunch } = wrapped(
      ["/opt/GDLauncher/gdlauncher", "gdlauncher://share/ABC"],
      []
    )
    app.relaunch({ args: ["--x"] })
    app.relaunch({ execPath: "/y" })
    expect(relaunch.mock.calls).toEqual([
      [{ args: ["--x"] }],
      [{ execPath: "/y" }]
    ])
  })

  it("the protocol queue lists a link as pending until the App acknowledges it", () => {
    const deliveries: ProtocolDelivery[] = []
    const queue = new ProtocolQueue<object>((_, delivery) =>
      deliveries.push(delivery)
    )
    const sender = {}
    queue.enqueue("gdlauncher://share/A")
    queue.enqueue("modrinth://mod/B")
    queue.bind(sender)
    expect(queue.pending()).toEqual([
      "gdlauncher://share/A",
      "modrinth://mod/B"
    ])
    queue.ready(sender, "1", queue.session(sender)!)
    expect(deliveries).toHaveLength(1)
    expect(queue.pending()).toEqual([
      "gdlauncher://share/A",
      "modrinth://mod/B"
    ])
    queue.ack(sender, deliveries[0])
    expect(queue.pending()).toEqual(["modrinth://mod/B"])
  })
})
