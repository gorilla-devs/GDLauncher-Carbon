import { expect, it } from "vitest"
import { createImportBatch } from "./importBatch"
import { scanRows } from "./importSelection"

function deferred() {
  let resolve!: (value?: unknown) => void
  let reject!: (error: unknown) => void
  const promise = new Promise((ok, fail) => {
    resolve = ok
    reject = fail
  })
  return { promise, resolve, reject }
}
function fixture() {
  const rows = scanRows(
    [0, 1].map((index) => ({
      Valid: {
        filename: `synthetic-${index}`,
        instance_name: `Synthetic ${index}`,
        warnings: []
      }
    })),
    "session-A"
  )
  const pending = [deferred(), deferred()]
  const state = {
    busy: false,
    session: "session-A",
    selected: rows.map((r) => r.selectionId),
    errors: [] as unknown[][],
    closed: 0,
    submitted: [] as number[]
  }
  const run = createImportBatch()
  const batch = () =>
    run({
      session: "session-A",
      selectionIds: [...state.selected],
      rows,
      submit: (request) => {
        state.submitted.push(request.index)
        return pending[request.index].promise
      },
      currentSession: () => state.session,
      setBusy: (value) => {
        state.busy = value
      },
      removeSelections: (ids) => {
        state.selected = state.selected.filter((id) => !ids.includes(id))
      },
      onUncertain: (errors) => {
        state.errors.push(errors)
      },
      onSuccess: () => {
        state.closed++
      }
    })
  return { state, rows, pending, batch }
}
it("retains busy until rejected and pending requests settle, then prevents replay", async () => {
  const f = fixture()
  const running = f.batch()
  f.pending[0].reject(new Error("ambiguous preparation failure"))
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(f.state.busy).toBe(true)
  await f.batch()
  expect(f.state.submitted).toEqual([0, 1])
  f.pending[1].resolve()
  await running
  expect(f.state.busy).toBe(false)
  expect(f.state.selected).toEqual([])
  f.state.selected = f.rows.map((r) => r.selectionId)
  await f.batch()
  expect(f.state.submitted).toEqual([0, 1])
  expect(f.state.errors.length).toBeGreaterThan(0)
})
it("settles all successes and closes once", async () => {
  const f = fixture()
  const running = f.batch()
  f.pending.forEach((p) => p.resolve())
  await running
  expect(f.state.closed).toBe(1)
  expect(f.state.selected).toEqual([])
  expect(f.state.errors).toEqual([])
})
it("treats all rejected mutations as uncertain and requires a fresh session", async () => {
  const f = fixture()
  const running = f.batch()
  f.pending.forEach((p) => p.reject(new Error("possibly published")))
  await running
  expect(f.state.closed).toBe(0)
  expect(f.state.errors[0]).toHaveLength(2)
  expect(f.state.selected).toEqual([])
})
it("does not clear a new scan selection or close its modal", async () => {
  const f = fixture()
  const running = f.batch()
  f.state.session = "session-B"
  f.state.selected = ["session-B:0"]
  f.pending.forEach((p) => p.resolve())
  await running
  expect(f.state.selected).toEqual(["session-B:0"])
  expect(f.state.closed).toBe(0)
})
it("keeps an early success consumed when its sibling later rejects", async () => {
  const f = fixture()
  const running = f.batch()
  f.pending[0].resolve()
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(f.state.busy).toBe(true)
  f.pending[1].reject(new Error("uncertain"))
  await running
  f.state.selected = [f.rows[0].selectionId]
  await f.batch()
  expect(f.state.submitted).toEqual([0, 1])
  expect(f.state.closed).toBe(0)
})
it("snapshots requests and does not remove newly selected rows", async () => {
  const f = fixture()
  const running = f.batch()
  f.state.selected = [...f.state.selected, "session-A:new"]
  f.pending[0].resolve()
  f.pending[1].reject(new Error("uncertain"))
  await running
  expect(f.state.selected).toEqual(["session-A:new"])
  expect(f.state.submitted).toEqual([0, 1])
})
it("allows explicit fresh-session recovery and suppresses old-session errors", async () => {
  const run = createImportBatch()
  const state = {
    session: "old",
    selected: ["old:0"],
    errors: [] as unknown[],
    accepted: 0
  }
  const old = deferred()
  const make = (session: string, submit: () => Promise<unknown>) => ({
    session,
    selectionIds: [`${session}:0`],
    rows: scanRows(
      [
        {
          Valid: {
            filename: "synthetic",
            instance_name: "Synthetic",
            warnings: []
          }
        }
      ],
      session
    ),
    submit,
    currentSession: () => state.session,
    setBusy: () => {},
    removeSelections: (ids: string[]) => {
      state.selected = state.selected.filter((id) => !ids.includes(id))
    },
    onUncertain: (errors: unknown[]) => {
      state.errors.push(errors)
    },
    onSuccess: () => {}
  })
  const running = run(make("old", () => old.promise))
  state.session = "fresh"
  state.selected = ["fresh:0"]
  old.reject(new Error("old uncertainty"))
  await running
  expect(state.errors).toEqual([])
  expect(state.selected).toEqual(["fresh:0"])
  await run(
    make("fresh", async () => {
      state.accepted++
    })
  )
  expect(state.accepted).toBe(1)
  expect(state.selected).toEqual([])
})
