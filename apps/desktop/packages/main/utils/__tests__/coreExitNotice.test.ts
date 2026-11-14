import { expect, it, vi } from "vitest"
import { CORE_EXIT_SETTLE_MS, createCoreExitNotice } from "../coreExitNotice"

function notice(expected = false) {
  const state = { expected }
  const sent: unknown[] = []
  const waits: [() => void, number][] = []
  const value = createCoreExitNotice({
    expected: () => state.expected,
    send: (exit) => sent.push(exit),
    wait: (run, ms) => waits.push([run, ms])
  })
  return {
    state,
    sent,
    waits,
    value,
    settle: () => waits.splice(0).forEach(([run]) => run())
  }
}
const crash = { code: 101, signal: null }

it("tells the window only after the wait, with how the core ended", () => {
  const f = notice()
  f.value.exited(true, crash)
  expect(f.sent).toEqual([])
  expect(f.value.current).toBeNull()
  expect(f.waits.map(([, ms]) => ms)).toEqual([CORE_EXIT_SETTLE_MS])
  f.settle()
  expect(f.sent).toEqual([crash])
  expect(f.value.current).toEqual(crash)
})

it("says nothing about a core that never became ready", () => {
  const f = notice()
  f.value.exited(false, crash)
  expect(f.waits).toEqual([])
  expect(f.value.current).toBeNull()
})

it("says nothing when the stop is expected as the core exits", () => {
  const f = notice(true)
  f.value.exited(true, crash)
  expect(f.waits).toEqual([])
  // Not even once what stopped it is over: that has its own way out.
  f.state.expected = false
  expect(f.sent).toEqual([])
  expect(f.value.current).toBeNull()
})

it("says nothing when a quit begins during the wait", () => {
  // Ctrl+C or a logout ends the core a moment before the launcher's quit.
  const f = notice()
  f.value.exited(true, crash)
  f.state.expected = true
  f.settle()
  expect(f.sent).toEqual([])
  f.state.expected = false
  expect(f.value.current).toBeNull()
})

it("a window loaded during a restart from the notice is not told again", () => {
  const f = notice()
  f.value.exited(true, crash)
  f.settle()
  f.state.expected = true
  expect(f.value.current).toBeNull()
  // Keep GDLauncher open: the core is still gone.
  f.state.expected = false
  expect(f.value.current).toEqual(crash)
})

it("waits on a real timer by default", () => {
  vi.useFakeTimers()
  try {
    const sent: unknown[] = []
    createCoreExitNotice({
      expected: () => false,
      send: (exit) => sent.push(exit)
    }).exited(true, crash)
    vi.advanceTimersByTime(CORE_EXIT_SETTLE_MS - 1)
    expect(sent).toEqual([])
    vi.advanceTimersByTime(1)
    expect(sent).toEqual([crash])
  } finally {
    vi.useRealTimers()
  }
})
