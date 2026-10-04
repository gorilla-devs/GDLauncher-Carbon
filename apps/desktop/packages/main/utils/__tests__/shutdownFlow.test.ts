import { afterEach, expect, it, vi } from "vitest"
import {
  ShutdownBlockedError,
  type RuntimeBlocker,
  type ShutdownStatus
} from "../coreShutdown"
import {
  ShutdownFlow,
  ShutdownNotPossibleError,
  type ShutdownAction,
  type ShutdownChoice,
  type ShutdownView
} from "../shutdownFlow"

afterEach(() => vi.useRealTimers())

type Report = (status: ShutdownStatus) => void
/**
 * A flow over scripted attempts: each `stop` takes the next entry, which
 * either resolves (the core exited), rejects, or stays pending until the test
 * settles it; it may report statuses meanwhile. Prompts are answered from
 * `answers`.
 */
function flow(
  action: ShutdownAction,
  attempts: ((report: Report) => Promise<void>)[],
  answers: (ShutdownChoice | "never")[] = [],
  cancel: () => Promise<"released" | "finishing"> = async () => "released",
  force: () => Promise<void> = async () => {}
) {
  const log: string[] = []
  const views: ShutdownView[] = []
  const updates: ShutdownView[] = []
  const aborted: boolean[] = []
  const promptSignals: AbortSignal[] = []
  const io = {
    stop: vi.fn(async (signal: AbortSignal, report: Report) => {
      log.push("stop")
      const next = attempts.shift()
      if (!next) throw new Error("no more attempts")
      signal.addEventListener("abort", () => log.push("stop aborted"))
      return next(report)
    }),
    cancel: vi.fn(async () => {
      log.push("cancel")
      return cancel()
    }),
    force: vi.fn(async () => {
      log.push("force")
      return force()
    }),
    prompt: vi.fn(async (view: ShutdownView, signal: AbortSignal) => {
      log.push(`prompt ${view.stage}`)
      views.push(view)
      promptSignals.push(signal)
      const answer = answers.shift() ?? "keep"
      if (answer !== "never") return answer
      return new Promise<ShutdownChoice | null>((resolve) =>
        signal.addEventListener("abort", () => {
          aborted.push(true)
          resolve(null)
        })
      )
    }),
    update: vi.fn((view: ShutdownView, signal: AbortSignal) => {
      // Only the prompt on screen is updated.
      expect(signal).toBe(promptSignals.at(-1))
      expect(signal.aborted).toBe(false)
      updates.push(view)
    }),
    hide: vi.fn(() => {
      log.push("hide")
    })
  }
  // As in main/index.ts, only a quit hides the window.
  const flowIo = { ...io, hide: action === "quit" ? io.hide : undefined }
  return {
    flow: new ShutdownFlow(action, flowIo, 3000),
    io,
    log,
    views,
    updates,
    aborted
  }
}
const unresolved = (blockers: RuntimeBlocker[]) => () =>
  Promise.reject(
    new ShutdownBlockedError("unresolved", "unresolved", blockers, true)
  )
/** An attempt that settles when the test says so. */
function pending() {
  let settle!: (error?: Error) => void
  const promise = new Promise<void>((resolve, reject) => {
    settle = (error) => (error ? reject(error) : resolve())
  })
  return { attempt: () => promise, settle }
}
/** An attempt that ends unresolved after `ms`. */
const unresolvedAfter = (ms: number, blockers: RuntimeBlocker[]) => () =>
  new Promise<void>((_, reject) =>
    setTimeout(
      () =>
        reject(
          new ShutdownBlockedError("unresolved", "unresolved", blockers, true)
        ),
      ms
    )
  )
/** A draining status naming `blockers`. */
const draining = (blockers: RuntimeBlocker[]): ShutdownStatus => ({
  requestId: "r",
  purpose: "quit",
  phase: "draining",
  blockers,
  serversStopped: true
})

it("a stop that finishes within 3 s asks nothing", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const f = flow("quit", [slow.attempt])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(2500)
  slow.settle()
  expect(await run).toBe("stopped")
  expect(f.io.prompt).not.toHaveBeenCalled()
})

it("a stop still running after 3 s shows what it waits for and closes the prompt itself when it finishes", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const f = flow("quit", [slow.attempt], ["never"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(2999)
  expect(f.io.prompt).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(1)
  expect(f.views[0]).toMatchObject({
    action: "quit",
    stage: "waiting",
    reason: null,
    canWait: true
  })
  slow.settle()
  expect(await run).toBe("stopped")
  expect(f.aborted).toEqual([true])
})

it("Wait hides the window until a long deletion is done, however many attempts that takes", async () => {
  vi.useFakeTimers()
  const deleting: RuntimeBlocker[] = [
    { kind: "deletion", name: "delete instance 12" }
  ]
  const first = pending()
  const second = pending()
  const third = pending()
  const f = flow(
    "quit",
    [first.attempt, second.attempt, third.attempt],
    ["wait"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.log).toEqual(["stop", "prompt waiting", "hide"])
  // Each attempt ends at the core's 15 s budget while the batch still runs.
  await vi.advanceTimersByTimeAsync(12000)
  first.settle(new ShutdownBlockedError("x", "unresolved", deleting, true))
  await vi.advanceTimersByTimeAsync(1000)
  expect(f.log.at(-1)).toBe("stop")
  await vi.advanceTimersByTimeAsync(15000)
  second.settle(new ShutdownBlockedError("x", "unresolved", deleting, true))
  await vi.advanceTimersByTimeAsync(1000)
  // Done at about 40 s: it closes without having asked again.
  await vi.advanceTimersByTimeAsync(8000)
  third.settle()
  expect(await run).toBe("stopped")
  expect(f.log).toEqual(["stop", "prompt waiting", "hide", "stop", "stop"])
  expect(f.io.cancel).not.toHaveBeenCalled()
})

it("a second launch between two unseen attempts shows the prompt at once", async () => {
  vi.useFakeTimers()
  const first = pending()
  const second = pending()
  const f = flow("quit", [first.attempt, second.attempt], ["wait", "never"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  first.settle(
    new ShutdownBlockedError("x", "unresolved", [
      { kind: "deletion", name: "delete instance 12" }
    ])
  )
  await vi.advanceTimersByTimeAsync(500)
  f.flow.reveal()
  await vi.advanceTimersByTimeAsync(0)
  expect(f.log).toEqual([
    "stop",
    "prompt waiting",
    "hide",
    "stop",
    "prompt waiting"
  ])
  second.settle()
  expect(await run).toBe("stopped")
})

it("after Wait, the window comes back only for what waiting cannot fix", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const f = flow(
    "quit",
    [slow.attempt, unresolved([{ kind: "unconfirmed", name: "lost worker" }])],
    ["wait", "keep"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  slow.settle(
    new ShutdownBlockedError(
      "x",
      "unresolved",
      [{ kind: "task", name: "stuck copy" }],
      true
    )
  )
  await vi.advanceTimersByTimeAsync(1000)
  expect(await run).toBe("kept")
  expect(f.log).toEqual([
    "stop",
    "prompt waiting",
    "hide",
    "stop",
    "prompt blocked",
    "cancel"
  ])
  expect(f.views[1]).toMatchObject({
    stage: "blocked",
    reason: "unresolved",
    canWait: false,
    serversStopped: true
  })
})

it("Try again on a blocked stop retries it, Keep GDLauncher open gives it back", async () => {
  const f = flow(
    "quit",
    [
      unresolved([{ kind: "task", name: "stuck copy" }]),
      () => Promise.reject(new ShutdownBlockedError("refused", "busy"))
    ],
    ["wait", "keep"]
  )
  expect(await f.flow.run()).toBe("kept")
  expect(f.log).toEqual([
    "stop",
    "prompt blocked",
    "hide",
    "stop",
    "prompt blocked",
    "cancel"
  ])
})

it("Try again on a quit retries unseen while it is still deleting, then closes", async () => {
  vi.useFakeTimers()
  const blockers: RuntimeBlocker[] = [
    { kind: "deletion", name: "delete instance 12" }
  ]
  const retry = pending()
  const f = flow(
    "quit",
    [unresolved(blockers), retry.attempt, async () => {}],
    ["wait"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(30000)
  // Still deleting 30 s later: the window stays hidden, nothing is asked.
  expect(f.log).toEqual(["stop", "prompt blocked", "hide", "stop"])
  retry.settle(new ShutdownBlockedError("x", "unresolved", blockers))
  await vi.advanceTimersByTimeAsync(1000)
  expect(await run).toBe("stopped")
  expect(f.log).toEqual(["stop", "prompt blocked", "hide", "stop", "stop"])
})

it("an unanswered waiting prompt stays up and keeps waiting while something is still finishing", async () => {
  vi.useFakeTimers()
  const first = pending()
  const second = pending()
  const f = flow("restart", [first.attempt, second.attempt], ["never"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.views[0]).toMatchObject({
    action: "restart",
    stage: "waiting",
    // The prompt itself is the wait: Wait would only hide it.
    canWait: false
  })
  first.settle(
    new ShutdownBlockedError(
      "x",
      "slow",
      [{ kind: "server", name: "server 7 run" }],
      true
    )
  )
  await vi.advanceTimersByTimeAsync(1000)
  expect(f.log).toEqual(["stop", "prompt waiting", "stop"])
  expect(f.aborted).toEqual([])
  second.settle()
  expect(await run).toBe("stopped")
  expect(f.aborted).toEqual([true])
  expect(f.views).toHaveLength(1)
})

it("Try again on an update says what it waits for again after 3 s", async () => {
  vi.useFakeTimers()
  const retry = pending()
  const f = flow(
    "update",
    [unresolved([{ kind: "install", name: "java install" }]), retry.attempt],
    ["wait", "never"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(2999)
  expect(f.log).toEqual(["stop", "prompt blocked", "stop"])
  await vi.advanceTimersByTimeAsync(1)
  expect(f.log.at(-1)).toBe("prompt waiting")
  retry.settle()
  expect(await run).toBe("stopped")
  expect(f.io.hide).not.toHaveBeenCalled()
})

it("Quit now hides the window at once, then ends the core and reports it forced", async () => {
  const f = flow(
    "quit",
    [unresolved([{ kind: "server", name: "server 7 run" }])],
    ["force"]
  )
  expect(await f.flow.run()).toBe("forced")
  // Gone before the few seconds SIGTERM, then SIGKILL can take.
  expect(f.log).toEqual(["stop", "prompt blocked", "hide", "force"])
  expect(f.io.cancel).not.toHaveBeenCalled()
})

it("Install now while it still waits stops waiting, and says it is stopping until the core's exit is seen", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const exit = pending()
  const f = flow(
    "update",
    [slow.attempt],
    ["force", "never"],
    undefined,
    exit.attempt
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.log).toEqual([
    "stop",
    "prompt waiting",
    "stop aborted",
    "prompt stopping",
    "force"
  ])
  expect(f.views[1]).toMatchObject({
    action: "update",
    stage: "stopping",
    reason: null,
    blockers: [],
    canWait: false
  })
  // Still up while the core is being ended.
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.aborted).toEqual([])
  exit.settle()
  expect(await run).toBe("forced")
  expect(f.aborted).toEqual([true])
  expect(f.io.hide).not.toHaveBeenCalled()
})

it("the stopping prompt goes away also when the core's exit could not be seen", async () => {
  const f = flow(
    "restart",
    [unresolved([{ kind: "deletion", name: "delete instance 12" }])],
    ["force", "never"],
    undefined,
    async () => {
      throw new Error("Core exit was not confirmed after SIGKILL")
    }
  )
  await expect(f.flow.run()).rejects.toThrow("not confirmed after SIGKILL")
  expect(f.log).toEqual(["stop", "prompt blocked", "prompt stopping", "force"])
  expect(f.aborted).toEqual([true])
})

it("Keep GDLauncher open that the core can no longer grant lets the stop finish", async () => {
  const f = flow(
    "quit",
    [unresolved([{ kind: "task", name: "game 3 session" }]), async () => {}],
    ["keep"],
    async () => "finishing"
  )
  expect(await f.flow.run()).toBe("stopped")
  expect(f.log).toEqual(["stop", "prompt blocked", "cancel", "stop"])
})

it("Keep GDLauncher open that comes too late says so, and is not offered again", async () => {
  vi.useFakeTimers()
  const handover = pending()
  const f = flow(
    "reset",
    [unresolved([{ kind: "task", name: "stuck copy" }]), handover.attempt],
    ["keep", "never"],
    async () => "finishing"
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(0)
  expect(f.log).toEqual([
    "stop",
    "prompt blocked",
    "cancel",
    "stop",
    "prompt waiting"
  ])
  expect(f.views[0].finishing).toBe(false)
  expect(f.views[1]).toMatchObject({ stage: "waiting", finishing: true })
  handover.settle()
  expect(await run).toBe("stopped")
})

it("a request that ends unresolved after coming too late can be given back again", async () => {
  vi.useFakeTimers()
  const handover = pending()
  let answers = 0
  const f = flow(
    "quit",
    [unresolved([{ kind: "task", name: "stuck copy" }]), handover.attempt],
    ["keep", "never", "keep"],
    async () => (++answers === 1 ? "finishing" : "released")
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(0)
  expect(f.views[1].finishing).toBe(true)
  // The games' session workers did not settle in time.
  handover.settle(
    new ShutdownBlockedError("x", "unresolved", [
      { kind: "unconfirmed", name: "game 3 session" }
    ])
  )
  expect(await run).toBe("kept")
  expect(f.views[2]).toMatchObject({ stage: "blocked", finishing: false })
})

it("a lost answer to Keep GDLauncher open is asked once more, and two still give the launcher back", async () => {
  let calls = 0
  const f = flow(
    "quit",
    [unresolved([{ kind: "task", name: "stuck copy" }])],
    ["keep"],
    async () => {
      if (++calls === 1) throw new Error("timed out")
      return "released"
    }
  )
  expect(await f.flow.run()).toBe("kept")
  expect(f.log).toEqual(["stop", "prompt blocked", "cancel", "cancel"])

  const lost = flow(
    "quit",
    [unresolved([{ kind: "task", name: "stuck copy" }])],
    ["keep"],
    async () => {
      throw new Error("timed out")
    }
  )
  expect(await lost.flow.run()).toBe("kept")
  expect(lost.log).toEqual(["stop", "prompt blocked", "cancel", "cancel"])
})

it("a task only a restart clears hides Wait", async () => {
  const f = flow(
    "restart",
    [unresolved([{ kind: "unconfirmed", name: "lost worker" }])],
    ["keep"]
  )
  expect(await f.flow.run()).toBe("kept")
  expect(f.views[0]).toMatchObject({ action: "restart", canWait: false })
})

it("a refusal names nothing from an earlier attempt", async () => {
  const f = flow(
    "quit",
    [
      unresolved([{ kind: "task", name: "stuck copy" }]),
      () => Promise.reject(new ShutdownBlockedError("refused", "busy"))
    ],
    ["wait", "keep"]
  )
  await f.flow.run()
  expect(f.views.map((view) => [view.reason, view.blockers.length])).toEqual([
    ["unresolved", 1],
    ["busy", 0]
  ])
  // Servers stopped by the first attempt stay stopped.
  expect(f.views[1].serversStopped).toBe(true)
})

it("a stop that is not possible from this launcher is not offered as a choice", async () => {
  const f = flow("reset", [
    () => Promise.reject(new ShutdownNotPossibleError("launcher-owned"))
  ])
  await expect(f.flow.run()).rejects.toThrow("launcher-owned")
  expect(f.io.prompt).not.toHaveBeenCalled()
  expect(f.io.force).not.toHaveBeenCalled()
})

it("reveal shows the prompt at once, and a prompt that went away unanswered is asked again", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const f = flow("quit", [slow.attempt], ["wait", "never"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.log).toEqual(["stop", "prompt waiting", "hide"])
  // A second launch while it finishes unseen.
  f.flow.reveal()
  await vi.advanceTimersByTimeAsync(0)
  expect(f.log.at(-1)).toBe("prompt waiting")
  slow.settle()
  expect(await run).toBe("stopped")
})

it("Try again on a restart says what it waits for 3 s after the click, however short each attempt is", async () => {
  vi.useFakeTimers()
  const deleting: RuntimeBlocker[] = [
    { kind: "deletion", name: "delete instance 12" }
  ]
  // Each attempt ends within the 3 s, so waiting 3 s per attempt never
  // showed the prompt again.
  const f = flow(
    "restart",
    [
      unresolved(deleting),
      unresolvedAfter(2500, deleting),
      unresolvedAfter(2500, deleting),
      unresolvedAfter(2500, deleting)
    ],
    ["wait", "never"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(0)
  expect(f.log).toEqual(["stop", "prompt blocked", "stop"])
  // The first retry ends at 2.5 s; the second starts after the 1 s pause,
  // already past the 3 s, so the prompt is shown with it.
  await vi.advanceTimersByTimeAsync(3499)
  expect(f.log).toEqual(["stop", "prompt blocked", "stop"])
  await vi.advanceTimersByTimeAsync(10)
  expect(f.log).toEqual([
    "stop",
    "prompt blocked",
    "stop",
    "stop",
    "prompt waiting"
  ])
  expect(f.views[1]).toMatchObject({ stage: "waiting", canWait: false })
  void run
})

it("after Try again, attempts that keep failing at once bring the blocked prompt back instead of retrying for good", async () => {
  vi.useFakeTimers()
  const fails = unresolvedAfter(200, [{ kind: "server", name: "server 7 run" }])
  const f = flow("restart", [fails, fails, fails, fails], ["wait", "keep"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(10000)
  expect(await run).toBe("kept")
  // Three quick attempts after the click, then the user is asked again.
  expect(f.log).toEqual([
    "stop",
    "prompt blocked",
    "stop",
    "stop",
    "stop",
    "prompt blocked",
    "cancel"
  ])
  expect(f.views[1]).toMatchObject({ stage: "blocked", canWait: true })
})

it("after Wait, a quit whose attempts keep failing at once comes back instead of retrying unseen for good", async () => {
  vi.useFakeTimers()
  const first = pending()
  const fails = unresolvedAfter(200, [{ kind: "server", name: "server 7 run" }])
  const f = flow("quit", [first.attempt, fails, fails, fails], ["wait", "keep"])
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.log).toEqual(["stop", "prompt waiting", "hide"])
  // The first attempt took the core's whole budget: it was waiting on
  // something. The next ones fail at once.
  await vi.advanceTimersByTimeAsync(12000)
  first.settle(
    new ShutdownBlockedError(
      "x",
      "unresolved",
      [{ kind: "server", name: "server 7 run" }],
      true
    )
  )
  await vi.advanceTimersByTimeAsync(10000)
  expect(await run).toBe("kept")
  expect(f.log).toEqual([
    "stop",
    "prompt waiting",
    "hide",
    "stop",
    "stop",
    "stop",
    "prompt blocked",
    "cancel"
  ])
})

it("a quit's waiting prompt offers no Wait while something waiting cannot clear is listed", async () => {
  vi.useFakeTimers()
  const slow = pending()
  const f = flow(
    "quit",
    [
      (report) => {
        // A server still saving, and a hook's child that never reported.
        report(
          draining([
            { kind: "server", name: "server 7 run" },
            { kind: "unconfirmed", name: "post-exit hook" }
          ])
        )
        return slow.attempt()
      }
    ],
    ["never", "keep"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.views[0]).toMatchObject({ stage: "waiting", canWait: false })
  slow.settle(
    new ShutdownBlockedError(
      "x",
      "unresolved",
      [{ kind: "unconfirmed", name: "post-exit hook" }],
      true
    )
  )
  expect(await run).toBe("kept")
  expect(f.log).toEqual(["stop", "prompt waiting", "prompt blocked", "cancel"])
  expect(f.io.hide).not.toHaveBeenCalled()
  expect(f.views[1]).toMatchObject({ stage: "blocked", canWait: false })
})

it("a waiting prompt follows what the stop waits for, also across attempts, and only when what it shows changes", async () => {
  vi.useFakeTimers()
  const server: RuntimeBlocker = { kind: "server", name: "server 7 run" }
  const deletion: RuntimeBlocker = {
    kind: "deletion",
    name: "delete instance 12"
  }
  let report!: Report
  const first = pending()
  const second = pending()
  const f = flow(
    "restart",
    [
      (r) => {
        report = r
        r(draining([server, deletion]))
        return first.attempt()
      },
      (r) => {
        report = r
        r(draining([deletion]))
        return second.attempt()
      }
    ],
    ["never"]
  )
  const run = f.flow.run()
  await vi.advanceTimersByTimeAsync(3000)
  expect(f.views[0].blockers).toEqual([server, deletion])
  // Another server stopping is still "a server is stopping".
  report(draining([{ kind: "server", name: "server 8 run" }, deletion]))
  expect(f.updates).toEqual([])
  // The server is done; the deletion goes on.
  report(draining([deletion]))
  expect(f.updates.map((view) => view.blockers)).toEqual([[deletion]])
  expect(f.updates[0]).toMatchObject({ stage: "waiting", action: "restart" })
  // The attempt runs out of budget; the next one names the same.
  first.settle(new ShutdownBlockedError("x", "unresolved", [deletion], true))
  await vi.advanceTimersByTimeAsync(1000)
  expect(f.log).toEqual(["stop", "prompt waiting", "stop"])
  expect(f.updates).toHaveLength(1)
  report(draining([]))
  expect(f.updates.at(-1)!.blockers).toEqual([])
  second.settle()
  expect(await run).toBe("stopped")
  expect(f.views).toHaveLength(1)
})
