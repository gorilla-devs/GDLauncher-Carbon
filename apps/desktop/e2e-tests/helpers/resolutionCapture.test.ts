import { afterEach, expect, it, vi } from "vitest"
import { EventEmitter } from "node:events"
import type { Page, Response } from "playwright"
import {
  captureModrinthVersions,
  captureCurseforgeVersions
} from "./resolutionCapture.js"
function driver() {
  const events = new EventEmitter()
  let now = 0
  vi.spyOn(Date, "now").mockImplementation(() => now)
  const page = {
    on: events.on.bind(events),
    off: events.off.bind(events),
    waitForTimeout: async (ms: number) => {
      now += ms
    }
  } as unknown as Page
  const emit = (
    route: string,
    input: unknown,
    id: string,
    error = false,
    truncated = false
  ) =>
    events.emit("response", {
      url: () =>
        `http://synthetic.invalid/rspc/${route}?input=${encodeURIComponent(JSON.stringify(input))}`,
      json: async () => ({
        result: error
          ? { type: "error", data: "bad" }
          : {
              type: "response",
              data: route.includes("modrinth")
                ? [
                    {
                      id,
                      date_published: "2026-01-01",
                      version_type: "release",
                      game_versions: ["1.20.1"],
                      loaders: ["fabric"]
                    }
                  ]
                : {
                    data: [
                      {
                        id: Number(id),
                        fileDate: "2026-01-01",
                        releaseType: "stable",
                        gameVersions: ["Fabric", "1.20.1"]
                      }
                    ],
                    pagination: {
                      index: 0,
                      pageSize: 20,
                      resultCount: 1,
                      totalCount: truncated ? 2 : 1
                    }
                  }
            }
      })
    } as unknown as Response)
  return { page, events, emit }
}
afterEach(() => vi.restoreAllMocks())
const mr = "modplatforms.modrinth.getProjectVersions",
  cf = "modplatforms.curseforge.getModFiles"
const right = {
  project_id: "target",
  game_versions: ["1.20.1"],
  loaders: ["fabric"]
}
const cfRight = {
  modId: 7,
  query: { gameVersion: "1.20.1", modLoaderType: "Fabric" }
}
const scope = { gameVersion: "1.20.1", loader: "fabric" }
const fetchImpl = (async (url: string | URL | Request) => {
  // eslint-disable-next-line @typescript-eslint/no-base-to-string -- called with a URL string
  expect(String(url)).toBe("https://api.modrinth.com/v2/project/target/version")
  return {
    ok: true,
    json: async () => [
      {
        id: "right",
        date_published: "2026-01-01",
        version_type: "release",
        game_versions: ["1.20.1"],
        loaders: ["fabric"]
      }
    ]
  } as globalThis.Response
}) as typeof fetch
it.each([
  { ...right, project_id: "foreign" },
  { ...right, game_versions: ["1.12.2"] },
  { ...right, loaders: ["forge"] },
  { ...right, loaders: ["fabric", "forge"] },
  { ...right, game_versions: ["1.20.1", "1.12.2"] },
  { ...right, loaders: ["neofabric"] }
])(
  "Modrinth excludes foreign exact-scope input %j after the target",
  async (bad) => {
    const d = driver()
    const lists = await captureModrinthVersions(
      d.page,
      "target",
      async () => {
        d.emit(mr, right, "right")
        d.emit(mr, bad, "wrong")
      },
      scope,
      { fetchImpl }
    )
    expect(lists.scoped.map((v) => v.id)).toEqual(["right"])
    expect(lists.unfiltered.map((v) => v.id)).toEqual(["right"])
    expect(d.events.listenerCount("response")).toBe(0)
  }
)
it("Modrinth accepts later exact repeats and removes listeners on run error and timeout", async () => {
  const d = driver()
  const lists = await captureModrinthVersions(
    d.page,
    "target",
    async () => {
      d.emit(mr, right, "earlier")
      d.emit(mr, right, "right")
    },
    scope,
    { fetchImpl }
  )
  expect(lists.scoped[0].id).toBe("right")
  const error = new Error("run")
  await expect(
    captureModrinthVersions(
      d.page,
      "target",
      async () => {
        throw error
      },
      scope,
      { fetchImpl }
    )
  ).rejects.toBe(error)
  await expect(
    captureModrinthVersions(d.page, "target", async () => {}, scope, {
      fetchImpl
    })
  ).rejects.toThrow(/no .*request/)
  expect(d.events.listenerCount("response")).toBe(0)
})
it.each([
  { ...cfRight, modId: 8 },
  { modId: 7, query: { gameVersion: "1.12.2", modLoaderType: "Fabric" } },
  { modId: 7, query: { gameVersion: "1.20.1", modLoaderType: "Forge" } },
  { modId: 7, query: { gameVersion: null, modLoaderType: "Fabric" } }
])("CurseForge excludes foreign scoped input %j", async (bad) => {
  const d = driver()
  const lists = await captureCurseforgeVersions(
    d.page,
    async () => {
      d.emit(cf, cfRight, "1")
      d.emit(cf, bad, "2")
    },
    { projectId: 7, ...scope },
    {
      ready: async () => {},
      driveUnfiltered: async () => {
        d.emit(
          cf,
          { modId: 7, query: { gameVersion: "1.20.1", modLoaderType: null } },
          "1"
        )
        d.emit(
          cf,
          { modId: 8, query: { gameVersion: "1.12.2", modLoaderType: null } },
          "2"
        )
      }
    }
  )
  expect(lists.scoped.map((v) => v.id)).toEqual(["1"])
  expect(lists.unfiltered.map((v) => v.id)).toEqual(["1"])
  expect(d.events.listenerCount("response")).toBe(0)
})
it.each(["error", "truncated"])(
  "CurseForge preserves %s rejection",
  async (mode) => {
    const d = driver()
    await expect(
      captureCurseforgeVersions(
        d.page,
        async () => {
          d.emit(cf, cfRight, "1", mode === "error", mode === "truncated")
        },
        { projectId: 7, ...scope },
        {
          ready: async () => {},
          driveUnfiltered: async () => {
            d.emit(
              cf,
              {
                modId: 7,
                query: { gameVersion: "1.20.1", modLoaderType: null }
              },
              "1"
            )
          }
        }
      )
    ).rejects.toThrow(mode === "error" ? /rspc error/ : /truncated/)
    expect(d.events.listenerCount("response")).toBe(0)
  }
)
