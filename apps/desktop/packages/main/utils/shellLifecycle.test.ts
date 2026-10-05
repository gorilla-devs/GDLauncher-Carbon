import { describe, it, expect } from "vitest"
import fs from "node:fs/promises"
import path from "node:path"
import os from "node:os"
import {
  createCoreOutput,
  MAX_CORE_RECORD,
  MAX_STARTUP_CAPTURE,
  type CoreStartup,
  type CoreLog
} from "./coreOutput"
import {
  LauncherLifecycle,
  mainWindowClosed,
  userWindows
} from "./windowLifecycle"
import { ProtocolQueue, type ProtocolDelivery } from "./protocolQueue"
import { subscribeProtocol } from "../../preload/protocolBridge"
import { subscribeCloseWarning } from "../../preload/closeWarningBridge"
import { acceptProtocolUrl } from "../../mainWindow/src/utils/protocolAction"
import {
  restoreDatabaseSnapshot,
  assertDatabaseRestoreReady,
  assertDatabaseRestoreReadySync
} from "./databaseRestore"
import { restoreSnapshotClick } from "../../preload/restoreButton"

function output() {
  const logs: CoreLog[] = [],
    results: CoreStartup[] = [],
    records: string[] = []
  const core = createCoreOutput({
    log: (row) => logs.push(row),
    settle: (value) => results.push(value),
    record: (row) => records.push(row),
    progress: () => {}
  })
  return { core, logs, results, records }
}
describe("Task23 complete core records", () => {
  it("preserves readiness and structured identity across every UTF8 byte split", () => {
    const ready = "_STATUS_:READY|1234|fixture-token\r\n"
    const identity =
      '_GDL_ACCOUNT_EMAIL_:{"session":"fixture","revision":1,"selected":"fixture","email":"café@example.invalid"}\r\n'
    for (const record of [ready, identity]) {
      const bytes = Buffer.from(record)
      for (let split = 0; split <= bytes.length; split++) {
        const value = output()
        value.core.push(bytes.subarray(0, split))
        value.core.push(bytes.subarray(split))
        if (record === ready) {
          expect(value.results).toHaveLength(1)
          expect(value.results[0]).toMatchObject({
            type: "success",
            result: { port: 1234, apiToken: "fixture-token" }
          })
          expect(JSON.stringify(value.results[0].logs)).not.toContain(
            "fixture-token"
          )
        } else expect(value.records).toEqual([identity.trimEnd()])
        expect(JSON.stringify(value.logs)).not.toContain("fixture-token")
        expect(JSON.stringify(value.logs)).not.toContain("café@example.invalid")
      }
    }
  })
  it("accepts CR/LF/CRLF once, byte chunks and empty identity payload; retains Windows snapshot colons", () => {
    const value = output()
    const text =
      '_GDL_ACCOUNT_EMAIL_:{"email":null}\r_STATUS_:READY|1234|fixture-token\nplain café\r\n'
    for (const byte of Buffer.from(text)) value.core.push(Buffer.from([byte]))
    expect(value.records).toEqual([
      '_GDL_ACCOUNT_EMAIL_:{"email":null}',
      "plain café"
    ])
    expect(value.results).toHaveLength(1)
    const fatal = output()
    fatal.core.push(
      Buffer.from("_STATUS_:DB_DOWNGRADE_FAILED|C:\\fixture\\snapshot.db\n")
    )
    expect(fatal.results[0]).toMatchObject({
      type: "error",
      snapshotPath: "C:\\fixture\\snapshot.db"
    })
  })
  it("EOF and overflowing records never dispatch sensitive fragments or readiness", () => {
    const value = output()
    value.core.push(Buffer.from("_STATUS_:READY|1234|fixture-token"))
    value.core.end()
    expect(value.results).toHaveLength(0)
    expect(JSON.stringify(value.logs)).not.toContain("fixture-token")
    const overflow = output()
    overflow.core.push(
      Buffer.from("_GDL_ACCOUNT_EMAIL_:" + "x".repeat(MAX_CORE_RECORD + 10))
    )
    overflow.core.push(
      Buffer.from("sensitive-tail\n_STATUS_:READY|1234|safe-token\n")
    )
    expect(overflow.records).toHaveLength(0)
    expect(overflow.results).toHaveLength(1)
    expect(JSON.stringify(overflow.logs)).not.toContain("sensitive-tail")
    expect(overflow.logs[0].message).toContain("discarded")
  })
  it("bounds startup capture and settles once on timeout/fatal while later identity still dispatches", () => {
    const value = output()
    for (let i = 0; i < 30; i++)
      value.core.push(Buffer.from("x".repeat(60000) + "\n"))
    value.core.fail("timeout")
    const frozen = JSON.stringify(value.results)
    value.core.push(
      Buffer.from(
        '_STATUS_:READY|1234|fixture-token\n_GDL_ACCOUNT_EMAIL_:{"email":null}\n'
      )
    )
    value.core.stderr("late")
    value.core.fail("late failure")
    expect(value.results).toHaveLength(1)
    expect(JSON.stringify(value.results)).toBe(frozen)
    expect(
      value.results[0].logs.reduce((n, l) => n + l.message.length, 0)
    ).toBeLessThan(MAX_STARTUP_CAPTURE + 100)
    expect(value.records.at(-1)).toContain("_GDL_ACCOUNT_EMAIL_")
  })
})
/** Behaves like an Electron 39 BrowserWindow: a window is marked destroyed
 *  before 'closed' fires, after which its getters (`webContents` included)
 *  and methods throw "Object has been destroyed" and `destroy()` does
 *  nothing. With `deferClosed`, an accepted close only destroys the window
 *  when the test calls `finishClose()`, as Electron does a turn later. */
class FakeWindow {
  destroyed = false
  warnings = 0
  hidden = false
  shown = 0
  focused = 0
  deferClosed = false
  private closeAccepted = false
  closed: () => void = () => {}
  closing: (e: { preventDefault(): void }) => void = () => {}
  readonly contents = {
    send: () => {
      this.warnings++
    }
  }
  private live() {
    if (this.destroyed) throw new TypeError("Object has been destroyed")
  }
  get webContents() {
    this.live()
    return this.contents
  }
  isDestroyed() {
    return this.destroyed
  }
  close() {
    this.live()
    let cancel = false
    this.closing({
      preventDefault: () => {
        cancel = true
      },
      get defaultPrevented() {
        return cancel
      }
    } as any)
    if (cancel) return
    if (this.deferClosed) this.closeAccepted = true
    else this.destroy()
  }
  finishClose() {
    if (!this.closeAccepted) throw Error("no accepted close is pending")
    this.closeAccepted = false
    this.destroy()
  }
  destroy() {
    if (this.destroyed) return
    this.destroyed = true
    this.closed()
  }
  hide() {
    this.live()
    this.hidden = true
  }
  minimize() {
    this.live()
    this.hidden = true
  }
  show() {
    this.live()
    this.shown++
  }
  focus() {
    this.live()
    this.focused++
  }
}
/** Windows wired the way main/index.ts wires them: the 'closed' listener is
 *  the real `mainWindowClosed` with the WebContents captured at creation,
 *  and the protocol queue is bound to each new window. By default each
 *  window's App has mounted and acknowledged the close warning, and the
 *  warning is enabled. */
function windows(
  options: { app?: boolean; deferClosed?: boolean; warning?: boolean } = {}
) {
  let current: FakeWindow | null = null,
    creates = 0,
    quits = 0,
    released = 0
  const deliveries: { sender: object; url: string }[] = []
  const queue = new ProtocolQueue<object>((sender, delivery) =>
    deliveries.push({ sender, url: delivery.url })
  )
  const lifecycle = new LauncherLifecycle<FakeWindow>({
    current: () => current,
    clear: (w) => {
      if (current === w) current = null
    },
    create: () => make(),
    quit: () => {
      quits++
    }
  })
  function make() {
    creates++
    const w = new FakeWindow()
    w.deferClosed = options.deferClosed ?? false
    current = w
    const contents = w.webContents
    queue.bind(contents)
    if (options.app ?? true) lifecycle.warningReceiver(contents, true)
    w.closing = (e) => lifecycle.close(w, e, options.warning ?? true)
    w.closed = () =>
      mainWindowClosed(w, contents, {
        current: () => current,
        release: () => {
          released++
        },
        lifecycle,
        protocol: queue
      })
    return w
  }
  return {
    lifecycle,
    queue,
    deliveries,
    make,
    get current() {
      return current
    },
    get creates() {
      return creates
    },
    get quits() {
      return quits
    },
    get released() {
      return released
    }
  }
}
describe("Task23 window and keyed game ownership", () => {
  it("launch-close bypasses only its warning and remains alive until matching owned exit", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    expect(w.destroyed).toBe(true)
    expect(w.warnings).toBe(0)
    expect(value.current).toBeNull()
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
    value.lifecycle.event("GAME_CLOSED|99|9|closeWindow")
    expect(value.creates).toBe(1)
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
  })
  it("ordinary cancellation retains handle; accepted close clears after closed; old callbacks retain replacement", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    w.close()
    expect(value.current).toBe(w)
    expect(w.destroyed).toBe(false)
    expect(w.warnings).toBe(1)
    const replacement = value.make()
    w.destroy()
    expect(value.current).toBe(replacement)
    value.lifecycle.close(
      replacement,
      {
        preventDefault() {
          throw Error("warning false must accept")
        }
      },
      false
    )
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    replacement.close()
    expect(value.current).toBeNull()
  })
  it("two owned games, duplicates, stale revision, adopted release and initial empty maintain aggregate", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":0,"instances":[]}')
    expect(value.lifecycle.gameRunning).toBe(false)
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|1|none")
    value.lifecycle.snapshot('{"revision":2,"instances":[1,2]}')
    value.lifecycle.event("GAME_LAUNCHED|2|2|none")
    value.lifecycle.snapshot('{"revision":3,"instances":[2]}')
    value.lifecycle.event("GAME_CLOSED|1|1|none")
    value.lifecycle.event("GAME_CLOSED|9|9|closeWindow")
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    w.close()
    expect(w.warnings).toBe(1)
    expect(value.lifecycle.gameRunning).toBe(true)
    value.lifecycle.snapshot('{"revision":4,"instances":[]}')
    expect(value.lifecycle.gameRunning).toBe(false)
    value.lifecycle.beginCore()
    value.lifecycle.snapshot('{"revision":1,"instances":[88]}')
    expect(value.lifecycle.gameRunning).toBe(true)
    expect(value.creates).toBe(1)
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    expect(value.lifecycle.gameRunning).toBe(false)
  })
  it("confirmed quit ignores game warning and suppresses reopening", () => {
    const value = windows()
    const w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|1|closeWindow")
    value.lifecycle.confirmQuit()
    value.lifecycle.event("GAME_CLOSED|1|1|closeWindow")
    expect(value.quits).toBe(1)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    expect(value.creates).toBe(1)
    expect(w.destroyed).toBe(true)
  })
})
describe("Main window closed listener on destroyed windows", () => {
  it("an X close does not throw and clears the handle", () => {
    const value = windows(),
      w = value.make()
    expect(() => w.close()).not.toThrow()
    expect(w.destroyed).toBe(true)
    // The fake really behaves like Electron inside 'closed'.
    expect(() => w.webContents).toThrow("Object has been destroyed")
    expect(value.current).toBeNull()
    expect(value.released).toBe(1)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    expect(value.creates).toBe(1)
  })
  it("a Close Window launch, then 'closed', then GAME_CLOSED reopens exactly one window", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    expect(() =>
      value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    ).not.toThrow()
    expect(w.destroyed).toBe(true)
    expect(w.warnings).toBe(0)
    expect(value.current).toBeNull()
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
    const reopened = value.current!
    expect(reopened).not.toBe(w)
    expect(reopened.isDestroyed()).toBe(false)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
    expect(value.current).toBe(reopened)
  })
  it("GAME_CLOSED arriving before 'closed' still reopens exactly once", () => {
    const value = windows({ deferClosed: true }),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    expect(w.destroyed).toBe(false)
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(1)
    expect(() => w.finishClose()).not.toThrow()
    expect(value.creates).toBe(2)
    expect(value.current).not.toBe(w)
    expect(value.current!.isDestroyed()).toBe(false)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
  })
  it("a deep link queued while no window exists is delivered to the reopened window", () => {
    const value = windows(),
      w = value.make(),
      first = w.webContents
    value.queue.ready(first, "1", value.queue.session(first)!)
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    expect(value.current).toBeNull()
    value.queue.enqueue("gdlauncher://share/WHILECLOSED")
    expect(value.deliveries).toHaveLength(0)
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    const reopened = value.current!.webContents
    // The closed window's renderer can no longer claim the request.
    expect(value.queue.session(first)).toBeNull()
    value.queue.ready(first, "1", 1)
    expect(value.deliveries).toHaveLength(0)
    value.queue.ready(reopened, "1", value.queue.session(reopened)!)
    expect(value.deliveries).toEqual([
      { sender: reopened, url: "gdlauncher://share/WHILECLOSED" }
    ])
  })
  it("closing the window a relaunch showed during a Close Window game quits instead of reopening later", () => {
    for (const deferClosed of [false, true]) {
      // The close warning is turned off ('never show'), or App has not
      // mounted yet, so X closes the window straight away.
      const value = windows({ warning: false, deferClosed }),
        w = value.make()
      value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
      value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
      if (deferClosed) w.finishClose()
      expect(w.destroyed).toBe(true)
      expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
      // GDLauncher is started again mid-game ('second-instance').
      const shown = value.make()
      shown.close()
      if (deferClosed) {
        // Until 'closed' arrives the window still counts as open.
        expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
        shown.finishClose()
      }
      expect(shown.destroyed).toBe(true)
      expect(value.current).toBeNull()
      // Nothing keeps the launcher alive, so 'window-all-closed' quits it,
      expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
      // and the game exiting later opens nothing.
      value.lifecycle.snapshot('{"revision":2,"instances":[]}')
      value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
      expect(value.creates).toBe(2)
      expect(value.current).toBeNull()
    }
  })
  it("a Close Window launch from a relaunched window keeps the earlier game's reopen", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    expect(w.destroyed).toBe(true)
    const shown = value.make()
    value.lifecycle.snapshot('{"revision":2,"instances":[1,2]}')
    value.lifecycle.event("GAME_LAUNCHED|2|8|closeWindow")
    expect(shown.destroyed).toBe(true)
    expect(shown.warnings).toBe(0)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
    value.lifecycle.snapshot('{"revision":3,"instances":[2]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(3)
    const reopened = value.current!
    expect(reopened.isDestroyed()).toBe(false)
    // The second game exiting reuses the window the first one reopened.
    value.lifecycle.snapshot('{"revision":4,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|2|8|closeWindow")
    expect(value.creates).toBe(3)
    expect(value.current).toBe(reopened)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
  })
  it("a replacement window that already exists keeps its own state when the old one closes", () => {
    const value = windows({ deferClosed: true }),
      w = value.make()
    w.close()
    const replacement = value.make()
    w.finishClose()
    expect(value.released).toBe(0)
    expect(value.current).toBe(replacement)
    // The queue stays bound to the replacement.
    const contents = replacement.webContents
    expect(value.queue.session(contents)).not.toBeNull()
  })
})
describe("Close warning needs a mounted App (adopted games)", () => {
  function app(value: ReturnType<typeof windows>, w: FakeWindow) {
    const listeners = new Set<() => void>(),
      contents = w.webContents
    let opened = 0
    const ipc = {
      on: (_: string, f: () => void) => {
        listeners.add(f)
      },
      removeListener: (_: string, f: () => void) => {
        listeners.delete(f)
      },
      send: (channel: string, ready: unknown) => {
        if (channel === "close-warning-receiver")
          value.lifecycle.warningReceiver(contents, ready === true)
      }
    }
    return {
      mount: () =>
        subscribeCloseWarning(ipc, () => {
          opened++
        }),
      listeners,
      get opened() {
        return opened
      }
    }
  }
  it("X during startup with an adopted game closes instead of silently doing nothing", () => {
    const value = windows({ app: false }),
      w = value.make()
    // The startup scan adopted a game from an earlier session before READY.
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
    expect(value.lifecycle.gameRunning).toBe(true)
    w.close()
    expect(w.warnings).toBe(0)
    expect(w.destroyed).toBe(true)
    expect(value.current).toBeNull()
  })
  it("a mounted App holds the close for its warning", () => {
    const value = windows({ app: false }),
      w = value.make(),
      renderer = app(value, w)
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
    renderer.mount()
    w.close()
    expect(w.warnings).toBe(1)
    expect(w.destroyed).toBe(false)
    expect(value.current).toBe(w)
  })
  it("once App unmounts the close goes ahead", () => {
    const value = windows({ app: false }),
      w = value.make(),
      renderer = app(value, w)
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
    renderer.mount()()
    expect(renderer.listeners.size).toBe(0)
    w.close()
    expect(w.warnings).toBe(0)
    expect(w.destroyed).toBe(true)
    expect(value.current).toBeNull()
  })
  it("a reload withdraws the old App's hold until the new App mounts", () => {
    const value = windows({ app: false }),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
    app(value, w).mount()
    // did-navigate: the document whose App acknowledged is gone. The new
    // document has its own ipcRenderer and acknowledges on the same
    // WebContents once its App mounts.
    value.lifecycle.warningReceiver(w.webContents, false)
    const reloaded = app(value, w)
    const stop = reloaded.mount()
    w.close()
    expect(w.warnings).toBe(1)
    expect(w.destroyed).toBe(false)
    stop()
    w.close()
    expect(w.warnings).toBe(1)
    expect(w.destroyed).toBe(true)
  })
  it("a reload before the new App mounts lets the close go ahead", () => {
    const value = windows({ app: false }),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}')
    app(value, w).mount()
    value.lifecycle.warningReceiver(w.webContents, false)
    w.close()
    expect(w.warnings).toBe(0)
    expect(w.destroyed).toBe(true)
    expect(value.current).toBeNull()
  })
  it("the bridge acknowledges once per document and only withdraws after the last listener", () => {
    const sent: unknown[] = [],
      listeners = new Set<() => void>()
    const ipc = {
      on: (_: string, f: () => void) => {
        listeners.add(f)
      },
      removeListener: (_: string, f: () => void) => {
        listeners.delete(f)
      },
      send: (channel: string, value: unknown) => sent.push([channel, value])
    }
    const calls: unknown[][] = []
    const first = subscribeCloseWarning(ipc, (...args: unknown[]) => {
      calls.push(args)
    })
    const second = subscribeCloseWarning(ipc, () => {})
    expect(sent).toEqual([["close-warning-receiver", true]])
    for (const listener of listeners) (listener as any)({ sender: "event" })
    // The IPC event object never reaches App.
    expect(calls).toEqual([[]])
    first()
    first()
    expect(sent).toHaveLength(1)
    second()
    expect(sent).toEqual([
      ["close-warning-receiver", true],
      ["close-warning-receiver", false]
    ])
    expect(listeners.size).toBe(0)
  })
  it("a core exit asks for a quit only when its Close Window games kept the launcher alive", () => {
    const value = windows(),
      w = value.make()
    // Startup, with no game or only an adopted one.
    value.lifecycle.beginCore("startup")
    expect(value.lifecycle.endCore("startup")).toBe(false)
    value.lifecycle.beginCore("adopted")
    value.lifecycle.snapshot('{"revision":1,"instances":[4]}', "adopted")
    expect(value.lifecycle.endCore("adopted")).toBe(false)
    // Hide and minimize launches never keep a windowless launcher alive.
    value.lifecycle.beginCore("hidden")
    value.lifecycle.event("GAME_LAUNCHED|1|1|hideWindow", "hidden")
    expect(w.hidden).toBe(true)
    expect(value.lifecycle.endCore("hidden")).toBe(false)
    // A quit already under way finishes on its own.
    value.lifecycle.beginCore("quitting")
    value.lifecycle.event("GAME_LAUNCHED|2|2|closeWindow", "quitting")
    expect(w.destroyed).toBe(true)
    value.lifecycle.confirmQuit()
    expect(value.lifecycle.endCore("quitting")).toBe(false)
  })
  it("a core exit clears the adopted guard and the Close Window keep-alive and ignores its late records", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.beginCore("first")
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}', "first")
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow", "first")
    expect(w.destroyed).toBe(true)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
    expect(value.lifecycle.endCore("stale")).toBe(false)
    expect(value.lifecycle.gameRunning).toBe(true)
    // Its game was keeping the windowless launcher alive: the caller has to
    // make the quit decision 'window-all-closed' deferred.
    expect(value.lifecycle.endCore("first")).toBe(true)
    expect(value.lifecycle.gameRunning).toBe(false)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    expect(value.lifecycle.endCore("first")).toBe(false)
    // Output the dead core printed before exiting, read afterwards.
    value.lifecycle.snapshot('{"revision":9,"instances":[1]}', "first")
    expect(value.lifecycle.gameRunning).toBe(false)
    const reopened = value.make()
    value.lifecycle.event("GAME_LAUNCHED|2|8|closeWindow", "first")
    expect(reopened.destroyed).toBe(false)
    reopened.close()
    expect(reopened.warnings).toBe(0)
    expect(reopened.destroyed).toBe(true)
    value.lifecycle.beginCore("second")
    value.lifecycle.snapshot('{"revision":1,"instances":[3]}', "second")
    expect(value.lifecycle.gameRunning).toBe(true)
  })
})
function protocol() {
  const sender = {},
    other = {},
    received: ProtocolDelivery[] = []
  const listeners = new Set<(event: unknown, value: ProtocolDelivery) => void>()
  const queue = new ProtocolQueue<object>((owner, delivery) => {
    received.push(delivery)
    for (const listener of listeners) listener(null, delivery)
  })
  queue.bind(sender)
  const ipc = {
    invoke: async () => queue.session(sender),
    on: (_: string, f: (e: unknown, v: ProtocolDelivery) => void) => {
      listeners.add(f)
    },
    removeListener: (
      _: string,
      f: (e: unknown, v: ProtocolDelivery) => void
    ) => {
      listeners.delete(f)
    },
    send: (channel: string, value: any) => {
      if (channel === "protocol-consumer-ready")
        queue.ready(sender, value.subscription, value.generation)
      else if (channel === "protocol-consumer-ack") queue.ack(sender, value)
      else queue.unsubscribe(sender, value)
    }
  }
  return { sender, other, queue, ipc, received, listeners }
}
describe("Task23 App callback readiness", () => {
  it("late subscription consumes FIFO share/search only after callback and ACK", async () => {
    const value = protocol(),
      actions: string[] = []
    value.queue.enqueue("gdlauncher://share/ABC123")
    value.queue.enqueue("curseforge://install?addonId=123&fileId=456")
    expect(value.received).toHaveLength(0)
    const cleanup = subscribeProtocol(value.ipc, (url) =>
      acceptProtocolUrl(url, {
        share: (c) => actions.push(c),
        navigate: (r) => actions.push(r)
      })
    )
    await Promise.resolve()
    expect(value.received).toHaveLength(2)
    expect(actions).toHaveLength(2)
    expect(actions[0]).toBe("ABC123")
    cleanup()
    value.queue.enqueue("gdlauncher://share/NEXT")
    expect(value.received).toHaveLength(2)
    subscribeProtocol(value.ipc, (url) => actions.push(url))
    await Promise.resolve()
    expect(actions).toHaveLength(3)
  })
  it("early subscription and postsubscribe ingress accept once, duplicate live delivery only ACKs", async () => {
    const value = protocol()
    let count = 0
    subscribeProtocol(value.ipc, () => {
      count++
    })
    await Promise.resolve()
    value.queue.enqueue("gdlauncher://share/CONTROL")
    for (const listener of value.listeners) listener(null, value.received[0])
    expect(count).toBe(1)
    value.queue.enqueue("gdlauncher://share/SECOND")
    expect(count).toBe(2)
  })
  it("stale sender/generation/ID cannot consume; reload and recreated windows revoke", () => {
    const value = protocol()
    value.queue.enqueue("gdlauncher://share/ONE")
    value.queue.ready(value.other, "bad", 0)
    expect(value.received).toHaveLength(0)
    value.queue.ready(value.sender, "1", value.queue.session(value.sender)!)
    const first = value.received[0]
    value.queue.ack(value.other, first)
    value.queue.ack(value.sender, { ...first, id: 999 })
    value.queue.revoke(value.sender)
    value.queue.ack(value.sender, first)
    value.queue.ready(value.sender, "2", value.queue.session(value.sender)!)
    expect(value.received[1].id).toBe(first.id)
    value.queue.close(value.sender)
    value.queue.bind(value.other)
    value.queue.ready(value.other, "3", value.queue.session(value.other)!)
    expect(value.received[2].id).toBe(first.id)
    expect(value.received[2].generation).not.toBe(first.generation)
  })
  it("callback failure retains entry and reports safely without ACK", async () => {
    const value = protocol()
    let reported = ""
    value.queue.enqueue("gdlauncher://share/FAIL")
    const cleanup = subscribeProtocol(
      value.ipc,
      () => {
        throw Error("synthetic")
      },
      (s) => {
        reported = s
      }
    )
    await Promise.resolve()
    expect(reported).toContain("pending")
    cleanup()
    let calls = 0
    subscribeProtocol(value.ipc, () => {
      calls++
    })
    await Promise.resolve()
    expect(calls).toBe(1)
  })
})
async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "task23-db-"))
  for (const [name, bytes] of [
    ["gdl_conf.db", "original"],
    ["gdl_conf.db-wal", "wal"],
    ["gdl_conf.db-shm", "shm"]
  ])
    await fs.writeFile(path.join(root, name), bytes)
  const snapshot = path.join(root, "gdl_conf.pre-downgrade.db")
  let stopped = 0,
    restarted = 0
  return {
    root,
    snapshot,
    stopCore: async () => {
      stopped++
    },
    restart: () => {
      restarted++
    },
    get stopped() {
      return stopped
    },
    get restarted() {
      return restarted
    }
  }
}
async function unchanged(root: string) {
  for (const [name, bytes] of [
    ["gdl_conf.db", "original"],
    ["gdl_conf.db-wal", "wal"],
    ["gdl_conf.db-shm", "shm"]
  ])
    expect(await fs.readFile(path.join(root, name), "utf8")).toBe(bytes)
}
describe("Task23 data preserving restore and recovery retry", () => {
  it("missing allowed snapshot preserves all bytes without shutdown and retry through actual button succeeds", async () => {
    const value = await fixture(),
      button = {
        disabled: false,
        textContent: "Restore",
        style: { opacity: "1" }
      },
      error = { textContent: "" }
    const click = restoreSnapshotClick(button as any, error, () =>
      restoreDatabaseSnapshot(value.root, value.snapshot, value)
    )
    await click()
    await unchanged(value.root)
    expect(button.disabled).toBe(false)
    expect(error.textContent).toContain("missing")
    expect(value.stopped).toBe(0)
    await fs.writeFile(value.snapshot, "snapshot")
    await click()
    expect(button.disabled).toBe(true)
    expect(value.stopped).toBe(1)
    expect(value.restarted).toBe(1)
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("snapshot")
    for (const suffix of ["-wal", "-shm"])
      await expect(
        fs.stat(path.join(value.root, "gdl_conf.db" + suffix))
      ).rejects.toMatchObject({ code: "ENOENT" })
    const bundle = (await fs.readdir(value.root)).find((n) =>
      n.startsWith("gdl_conf.restore-originals-")
    )!
    await unchanged(path.join(value.root, bundle))
    await assertDatabaseRestoreReady(value.root)
  })
  it("wrong sibling and failed observed shutdown do not mutate/relaunch", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    expect(
      (
        await restoreDatabaseSnapshot(
          value.root,
          path.join(value.root, "foreign.db"),
          value
        )
      ).ok
    ).toBe(false)
    expect(value.stopped).toBe(0)
    const outcome = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      stopCore: async () => {
        throw Error("unconfirmed core exit")
      }
    })
    expect(outcome.ok).toBe(false)
    await unchanged(value.root)
    expect(value.restarted).toBe(0)
  })
  it("required sidecar move failure rolls back captured DB, retains recovery bytes and never relaunches", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      rename: async (from: string, to: string) => {
        if (from.endsWith("-wal")) throw Error("synthetic access refusal")
        return fs.rename(from, to)
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    expect(result.ok).toBe(false)
    await unchanged(value.root)
    expect(value.restarted).toBe(0)
    await assertDatabaseRestoreReady(value.root)
    expect(result.recoveryPath).toBeTruthy()
    expect(
      await fs.readFile(path.join(result.recoveryPath!, "gdl_conf.db"), "utf8")
    ).toBe("original")
  })
  it("publication failure rolls back; copy failure leaves DB and sidecars untouched", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      link: async (from: string, to: string) => {
        if (from.endsWith("snapshot-staged.db")) throw Error("publish failed")
        return fs.link(from, to)
      }
    }
    expect(
      (
        await restoreDatabaseSnapshot(value.root, value.snapshot, {
          ...value,
          files: files as typeof fs
        })
      ).ok
    ).toBe(false)
    await unchanged(value.root)
    const copyFiles = {
      ...fs,
      copyFile: async () => {
        throw Error("copy failed")
      }
    }
    expect(
      (
        await restoreDatabaseSnapshot(value.root, value.snapshot, {
          ...value,
          files: copyFiles as typeof fs
        })
      ).ok
    ).toBe(false)
    await unchanged(value.root)
    expect(value.restarted).toBe(0)
  })
  it("link-unavailable rollback exclusively copies and verifies the original database", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      rename: async (from: string, to: string) => {
        if (from.endsWith("-wal")) throw Error("sidecar preserve failed")
        return fs.rename(from, to)
      },
      link: async () => {
        throw Object.assign(Error("links unsupported"), { code: "ENOTSUP" })
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    expect(result.ok).toBe(false)
    expect(value.restarted).toBe(0)
    await unchanged(value.root)
    await assertDatabaseRestoreReady(value.root)
  })
  it("intent retirement failure after publication keeps guard and never claims success", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      unlink: async (file: string) => {
        if (file.endsWith(".gdl-db-restore.json")) throw Error("intent busy")
        return fs.unlink(file)
      }
    }
    const outcome = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    expect(outcome.ok).toBe(false)
    expect(value.restarted).toBe(0)
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("snapshot")
    await unchanged(outcome.recoveryPath!)
    await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
      "needs recovery"
    )
  })
  it("the synchronous startup guard refuses exactly what the async one refuses", async () => {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), "restore-sync-"))
    try {
      expect(() => assertDatabaseRestoreReadySync(root)).not.toThrow()
      await fs.writeFile(path.join(root, ".gdl-db-restore.json"), "{}")
      expect(() => assertDatabaseRestoreReadySync(root)).toThrow(
        "needs recovery"
      )
      await expect(assertDatabaseRestoreReady(root)).rejects.toThrow(
        "needs recovery"
      )
    } finally {
      await fs.rm(root, { recursive: true, force: true })
    }
  })
  it("foreign occupant makes rollback ambiguous; startup guard refuses partial originals", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      rename: async (from: string, to: string) => {
        if (from.endsWith("-wal")) {
          await fs.writeFile(path.join(value.root, "gdl_conf.db"), "foreign")
          throw Error("interrupted")
        }
        return fs.rename(from, to)
      }
    }
    const outcome = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    expect(outcome.ok).toBe(false)
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("foreign")
    expect(
      await fs.readFile(path.join(outcome.recoveryPath!, "gdl_conf.db"), "utf8")
    ).toBe("original")
    await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
      "needs recovery"
    )
    expect(value.restarted).toBe(0)
  })
  it("thrown IPC and unsafe failure text are handled via textContent and Retry", async () => {
    const button = {
        disabled: false,
        textContent: "Restore",
        style: { opacity: "1" }
      },
      error = { textContent: "" }
    await restoreSnapshotClick(button as any, error, async () => {
      throw Error("ipc")
    })()
    expect(button.disabled).toBe(false)
    expect(button.textContent).toBe("Retry Restore")
    await restoreSnapshotClick(button as any, error, async () => ({
      ok: false,
      message: "<img src=x>"
    }))()
    expect(error.textContent).toBe("<img src=x>")
  })
})
it("Task23 reload delayed prior readiness cannot suppress current subscription redelivery", () => {
  const sender = {},
    deliveries: ProtocolDelivery[] = []
  const queue = new ProtocolQueue<object>((_, delivery) =>
    deliveries.push(delivery)
  )
  queue.bind(sender)
  queue.enqueue("gdlauncher://share/PENDING")
  const oldGeneration = queue.session(sender)!
  queue.revoke(sender)
  queue.ready(sender, "1", oldGeneration) // delayed pre-navigation readiness is rejected
  expect(deliveries).toHaveLength(0)
  queue.ready(sender, "1", queue.session(sender)!) // new renderer revision can restart at 1
  expect(deliveries).toHaveLength(1)
})
it("Task23 supported link-unsupported filesystem still restores a known snapshot safely", async () => {
  const value = await fixture()
  await fs.writeFile(value.snapshot, "snapshot")
  const files = {
    ...fs,
    link: async () => {
      throw Object.assign(Error("unsupported links"), { code: "ENOTSUP" })
    }
  }
  const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
    ...value,
    files: files as typeof fs
  })
  expect(result.ok).toBe(true)
  expect(await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")).toBe(
    "snapshot"
  )
})

it("Task23 exclusive publication refuses occupied destination without overwriting or unsafe unlink", async () => {
  const value = await fixture()
  await fs.writeFile(value.snapshot, "snapshot")
  let exclusiveCalls = 0
  const files = {
    ...fs,
    link: async (from: string, to: string) => {
      if (from.endsWith("snapshot-staged.db")) {
        await fs.writeFile(to, "foreign")
        throw Object.assign(Error("occupied"), { code: "EEXIST" })
      }
      return fs.link(from, to)
    },
    copyFile: async (from: string, to: string, flags?: number) => {
      if (from.endsWith("snapshot-staged.db")) exclusiveCalls++
      return fs.copyFile(from, to, flags)
    }
  }
  const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
    ...value,
    files: files as typeof fs
  })
  expect(result.ok).toBe(false)
  expect(exclusiveCalls).toBe(0)
  expect(value.restarted).toBe(0)
  expect(await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")).toBe(
    "foreign"
  )
  await unchanged(result.recoveryPath!)
  await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
    "needs recovery"
  )
})
it("Task23 partial exclusive copy retains original bundle and guards startup", async () => {
  const value = await fixture()
  await fs.writeFile(value.snapshot, "snapshot")
  const files = {
    ...fs,
    link: async () => {
      throw Object.assign(Error("unsupported"), { code: "ENOTSUP" })
    },
    copyFile: async (from: string, to: string, flags?: number) => {
      if (from.endsWith("snapshot-staged.db")) {
        await fs.writeFile(to, "partial", { flag: "wx" })
        throw Error("partial IO")
      }
      return fs.copyFile(from, to, flags)
    }
  }
  const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
    ...value,
    files: files as typeof fs
  })
  expect(result.ok).toBe(false)
  expect(value.restarted).toBe(0)
  expect(await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")).toBe(
    "partial"
  )
  await unchanged(result.recoveryPath!)
  await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
    "needs recovery"
  )
})
it("Task23 permission/link IO errors never use fallback; failed rollback preserves guarded originals", async () => {
  const value = await fixture()
  await fs.writeFile(value.snapshot, "snapshot")
  let fallback = 0
  const files = {
    ...fs,
    link: async () => {
      throw Object.assign(Error("permission"), { code: "EACCES" })
    },
    copyFile: async (from: string, to: string, flags?: number) => {
      if (from.endsWith("snapshot-staged.db")) fallback++
      return fs.copyFile(from, to, flags)
    }
  }
  const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
    ...value,
    files: files as typeof fs
  })
  expect(result.ok).toBe(false)
  expect(fallback).toBe(0)
  await unchanged(result.recoveryPath!)
  await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
    "needs recovery"
  )
})

it("Task23 asynchronous actual closed event retains launch purpose; canceled unload retains handle and user warning", () => {
  const value = windows(),
    w = value.make()
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  w.close = () => w.closing({ preventDefault() {} })
  value.lifecycle.event("GAME_LAUNCHED|1|1|closeWindow")
  expect(value.current).toBe(w)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
  w.destroy()
  expect(value.current).toBeNull()
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
  const other = windows(),
    canceled = other.make()
  other.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  canceled.close = () => canceled.closing({ preventDefault() {} })
  other.lifecycle.event("GAME_LAUNCHED|1|1|closeWindow")
  other.lifecycle.cancelLaunchClose(canceled)
  expect(other.current).toBe(canceled)
  expect(other.lifecycle.keepAliveWithoutWindows()).toBe(false)
  other.lifecycle.close(canceled, { preventDefault() {} }, true)
  expect(canceled.warnings).toBe(1)
})

it("Task23 a new core session rejects old snapshots and owned actions", () => {
  const value = windows()
  const w = value.make()
  value.lifecycle.beginCore("old")
  value.lifecycle.snapshot('{"revision":10,"instances":[1]}', "old")
  value.lifecycle.beginCore("new")
  value.lifecycle.snapshot('{"revision":1,"instances":[2]}', "new")
  value.lifecycle.snapshot('{"revision":99,"instances":[]}', "old")
  value.lifecycle.event("GAME_LAUNCHED|1|1|closeWindow", "old")
  expect(value.lifecycle.gameRunning).toBe(true)
  expect(w.destroyed).toBe(false)
})

// Round01 controls execute the actual owners with inert windows and runner-local bytes.
describe("Task23 round01 restore namespace and close settlement", () => {
  async function absent() {
    const value = await fixture()
    for (const name of ["gdl_conf.db", "gdl_conf.db-wal", "gdl_conf.db-shm"])
      await fs.unlink(path.join(value.root, name))
    await fs.writeFile(value.snapshot, "complete snapshot")
    return value
  }
  it("round01 absent-original partial-copy keeps intent and refuses actual startup guard", async () => {
    const value = await absent()
    const files = {
      ...fs,
      link: async () => {
        throw Object.assign(Error("unsupported link"), { code: "ENOTSUP" })
      },
      copyFile: async (from: string, to: string, flags?: number) => {
        if (from.endsWith("snapshot-staged.db")) {
          await fs.writeFile(to, "partial destination", { flag: "wx" })
          throw Object.assign(Error("entered copy failure"), { code: "EIO" })
        }
        return fs.copyFile(from, to, flags)
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
      "needs recovery"
    )
    expect(result.ok).toBe(false)
    expect(value.restarted).toBe(0)
    expect(result.ok ? "" : result.message).toContain("partial or foreign")
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("partial destination")
    expect(
      await fs.readFile(
        path.join(result.recoveryPath!, "snapshot-staged.db"),
        "utf8"
      )
    ).toBe("complete snapshot")
    expect(await fs.readFile(value.snapshot, "utf8")).toBe("complete snapshot")
  })
  it("round01 healthy absent originals support linked and exclusive-copy restoration", async () => {
    for (const fallback of [false, true]) {
      const value = await absent()
      const files = fallback
        ? {
            ...fs,
            link: async () => {
              throw Object.assign(Error("unsupported"), { code: "ENOTSUP" })
            }
          }
        : fs
      const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
        ...value,
        files: files
      })
      expect(result.ok).toBe(true)
      expect(value.restarted).toBe(1)
      await assertDatabaseRestoreReady(value.root)
      expect(
        await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
      ).toBe("complete snapshot")
      for (const suffix of ["-wal", "-shm"])
        await expect(
          fs.lstat(path.join(value.root, "gdl_conf.db" + suffix))
        ).rejects.toMatchObject({ code: "ENOENT" })
    }
  })
  it("round01 clean prepublication failure reconciles captured absence without blocking retry", async () => {
    const value = await absent()
    const files = {
      ...fs,
      link: async () => {
        throw Object.assign(Error("publish failed before entry"), {
          code: "EIO"
        })
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    expect(result.ok).toBe(false)
    expect(value.restarted).toBe(0)
    await assertDatabaseRestoreReady(value.root)
    for (const name of ["gdl_conf.db", "gdl_conf.db-wal", "gdl_conf.db-shm"])
      await expect(fs.lstat(path.join(value.root, name))).rejects.toMatchObject(
        { code: "ENOENT" }
      )
    expect(
      await fs.readFile(
        path.join(result.recoveryPath!, "snapshot-staged.db"),
        "utf8"
      )
    ).toBe("complete snapshot")
  })
  it("round01 newly occupied absent sidecar preserves rollback bytes and startup intent", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    await fs.unlink(path.join(value.root, "gdl_conf.db-wal"))
    const files = {
      ...fs,
      link: async (from: string, to: string) => {
        if (from.endsWith("snapshot-staged.db")) {
          await fs.writeFile(
            path.join(value.root, "gdl_conf.db-wal"),
            "foreign sidecar",
            { flag: "wx" }
          )
          throw Error("publication failed")
        }
        return fs.link(from, to)
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
      "needs recovery"
    )
    expect(result.ok).toBe(false)
    expect(value.restarted).toBe(0)
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("original")
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db-wal"), "utf8")
    ).toBe("foreign sidecar")
    expect(
      await fs.readFile(path.join(result.recoveryPath!, "gdl_conf.db"), "utf8")
    ).toBe("original")
  })
  it("round01 changed unmoved original cannot retire intent after another file rolls back", async () => {
    const value = await fixture()
    await fs.writeFile(value.snapshot, "snapshot")
    const files = {
      ...fs,
      rename: async (from: string, to: string) => {
        if (from.endsWith("-wal")) {
          await fs.writeFile(from, "changed unmoved WAL")
          throw Error("move refused")
        }
        return fs.rename(from, to)
      }
    }
    const result = await restoreDatabaseSnapshot(value.root, value.snapshot, {
      ...value,
      files: files as typeof fs
    })
    await expect(assertDatabaseRestoreReady(value.root)).rejects.toThrow(
      "needs recovery"
    )
    expect(result.ok).toBe(false)
    expect(value.restarted).toBe(0)
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db"), "utf8")
    ).toBe("original")
    expect(
      await fs.readFile(path.join(value.root, "gdl_conf.db-wal"), "utf8")
    ).toBe("changed unmoved WAL")
  })
  it("round01 matching exit before delayed actual closed reopens once and retires purpose", () => {
    const value = windows(),
      w = value.make()
    w.close = () => w.closing({ preventDefault() {} })
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    w.destroy()
    expect(value.creates).toBe(2)
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    const replacement = value.current
    w.destroy()
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.creates).toBe(2)
    expect(value.current).toBe(replacement)
  })
  it("round01 current replacement suppresses reopen and retires the completed close purpose", () => {
    const value = windows(),
      w = value.make()
    value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
    value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
    const replacement = value.make()
    value.lifecycle.snapshot('{"revision":2,"instances":[]}')
    value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
    expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
    expect(value.creates).toBe(2)
    w.destroy()
    expect(value.current).toBe(replacement)
  })
  it("round01 delayed closure preserves replacement and quit/cancellation ownership", () => {
    for (const mode of ["replacement", "quit", "cancel"] as const) {
      const value = windows(),
        w = value.make()
      w.close = () => w.closing({ preventDefault() {} })
      value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
      value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
      value.lifecycle.snapshot('{"revision":2,"instances":[]}')
      value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
      const replacement = mode === "replacement" ? value.make() : null
      if (mode === "quit") value.lifecycle.confirmQuit()
      if (mode === "cancel") value.lifecycle.cancelLaunchClose(w)
      if (mode !== "cancel") w.destroy()
      expect(value.creates).toBe(mode === "replacement" ? 2 : 1)
      expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
      expect(value.current).toBe(mode === "cancel" ? w : replacement)
      expect(value.quits).toBe(mode === "quit" ? 1 : 0)
    }
  })
})

it("Task23 round01 delayed actual closed before exit reopens once and retires purpose", () => {
  const value = windows(),
    w = value.make()
  w.close = () => w.closing({ preventDefault() {} })
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  w.destroy()
  expect(value.current).toBeNull()
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
  value.lifecycle.snapshot('{"revision":2,"instances":[]}')
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(value.creates).toBe(2)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
  const replacement = value.current
  w.destroy()
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(value.creates).toBe(2)
  expect(value.current).toBe(replacement)
})
it("Task23 round01 shared window close purposes preserve another owned game until its exit", () => {
  const value = windows(),
    w = value.make()
  w.close = () => w.closing({ preventDefault() {} })
  value.lifecycle.snapshot('{"revision":1,"instances":[1,2]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  value.lifecycle.event("GAME_LAUNCHED|2|8|closeWindow")
  value.lifecycle.snapshot('{"revision":2,"instances":[2]}')
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  w.destroy()
  expect(value.creates).toBe(2)
  expect(value.lifecycle.gameRunning).toBe(true)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(true)
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  w.destroy()
  expect(value.creates).toBe(2)
  value.lifecycle.snapshot('{"revision":3,"instances":[]}')
  value.lifecycle.event("GAME_CLOSED|2|8|closeWindow")
  expect(value.creates).toBe(2)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
})
it("Task23 round01 canceled close before exit retains warnings and never claims reopen purpose", () => {
  const value = windows(),
    w = value.make()
  w.close = () => w.closing({ preventDefault() {} })
  value.lifecycle.snapshot('{"revision":1,"instances":[1]}')
  value.lifecycle.event("GAME_LAUNCHED|1|7|closeWindow")
  value.lifecycle.cancelLaunchClose(w)
  w.close()
  expect(w.warnings).toBe(1)
  expect(value.current).toBe(w)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
  value.lifecycle.snapshot('{"revision":2,"instances":[]}')
  value.lifecycle.event("GAME_CLOSED|1|7|closeWindow")
  expect(value.creates).toBe(1)
  expect(value.current).toBe(w)
  expect(value.lifecycle.keepAliveWithoutWindows()).toBe(false)
})
