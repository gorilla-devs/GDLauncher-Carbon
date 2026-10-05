import { expect, it, vi } from "vitest"
import { AnimationControllerImpl } from "./AnimationController"

function deferred() {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
async function flush() {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
function fixture(reduced = false, ready = true) {
  const handles: Record<string, any[]> = { backButton: [], skipButton: [] }
  const make = (name: string) => ({
    style: { width: "0", margin: "0", opacity: "1", transform: "" },
    animate: vi.fn((frames: any, options: any) => {
      const done = deferred()
      const handle = {
        finished: done.promise,
        cancel: vi.fn(),
        frames,
        options,
        ...done
      }
      handles[name]?.push(handle)
      return handle
    })
  })
  const refs = Object.fromEntries(
    [
      "sidebar",
      "video",
      "backgroundBlur",
      "loadingSpinner",
      "backButton",
      "skipButton"
    ].map((name) => [name, make(name)])
  )
  const c = new AnimationControllerImpl(reduced)
  if (ready) c.registerRefs(refs as any)
  return { c, refs, handles }
}
const abort = () =>
  Object.assign(new Error("synthetic delayed cancellation"), {
    name: "AbortError"
  })
for (const name of ["backButton", "skipButton"] as const) {
  for (const direction of ["show", "hide"] as const) {
    it(`0255 ${name} ${direction} replacement survives delayed predecessor AbortError and cancels its successor`, async () => {
      const { c, handles } = fixture()
      const other = direction === "show" ? "hide" : "show"
      if (direction === "hide") {
        const seed = c[name].show()
        await flush()
        handles[name][0].resolve()
        await seed
        handles[name].length = 0
      }
      const first = c[name][direction]()
      await flush()
      const second = c[name][other]()
      await flush()
      expect(handles[name][0].cancel).toHaveBeenCalledOnce()
      await Promise.resolve().then(() => handles[name][0].reject(abort()))
      await first
      const third = c[name][direction]()
      await flush()
      expect(handles[name][1].cancel).toHaveBeenCalledOnce()
      handles[name][2].resolve()
      await third
      handles[name][1].resolve()
      await second
      await c[name][direction]()
      expect(handles[name]).toHaveLength(3)
    })
    it(`0255 ${name} ${direction} stale successful predecessor cannot publish or clear the current animation`, async () => {
      const { c, handles } = fixture()
      const other = direction === "show" ? "hide" : "show"
      if (direction === "hide") {
        const seed = c[name].show()
        await flush()
        handles[name][0].resolve()
        await seed
        handles[name].length = 0
      }
      const first = c[name][direction]()
      await flush()
      const second = c[name][other]()
      await flush()
      handles[name][0].resolve()
      await first
      const third = c[name][direction]()
      await flush()
      expect(handles[name][1].cancel).toHaveBeenCalledOnce()
      expect(handles[name]).toHaveLength(3)
      handles[name][2].resolve()
      await third
      handles[name][1].resolve()
      await second
      await c[name][direction]()
      expect(handles[name]).toHaveLength(3)
    })
  }
  it(`0255 ${name} opposite intent before refs revokes the older show waiter`, async () => {
    const { c, refs, handles } = fixture(false, false)
    const show = c[name].show()
    const hide = c[name].hide()
    c.registerRefs(refs as any)
    await flush()
    expect(
      handles[name].every((handle) => handle.frames.at(-1).width === "0")
    ).toBe(true)
    handles[name].forEach((handle) => handle.resolve())
    await Promise.all([show, hide])
  })
  it(`0255 ${name} show/hide/show and repeated intent before refs start only the latest work`, async () => {
    const { c, refs, handles } = fixture(false, false)
    const first = c[name].show(),
      second = c[name].hide(),
      third = c[name].show(),
      duplicate = c[name].show()
    c.registerRefs(refs as any)
    await flush()
    expect(handles[name]).toHaveLength(1)
    handles[name][0].resolve()
    await Promise.all([first, second, third, duplicate])
    await c[name].show()
    expect(handles[name]).toHaveLength(1)
  })
  it(`0255 ${name} missing optional ref releases unstarted intent for later registration`, async () => {
    const { c, refs, handles } = fixture(false, false)
    const { [name]: button, ...essential } = refs
    c.registerRefs(essential as any)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    await c[name].show()
    await c[name].show()
    c.registerRefs({ [name]: button } as any)
    const show = c[name].show()
    await flush()
    expect(handles[name]).toHaveLength(1)
    handles[name][0].resolve()
    await show
    warn.mockRestore()
  })
  it(`0255 ${name} sequential and repeated pending/completed intent keep original keyframes and timings`, async () => {
    const { c, handles } = fixture()
    const first = c[name].show()
    await flush()
    await c[name].show()
    expect(handles[name]).toHaveLength(1)
    expect(handles[name][0].frames.at(-1)).toEqual({
      width: name === "backButton" ? "60%" : "40%",
      margin: name === "backButton" ? "0 1rem 0 0" : "0"
    })
    expect(handles[name][0].options).toEqual({
      duration: 300,
      easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
      fill: "forwards"
    })
    handles[name][0].resolve()
    await first
    await c[name].show()
    const second = c[name].hide()
    await flush()
    await c[name].hide()
    expect(handles[name]).toHaveLength(2)
    handles[name][1].resolve()
    await second
    await c[name].hide()
    expect(handles[name]).toHaveLength(2)
  })
  it(`0255 ${name} reduced motion retains immediate geometry without waiting for essential refs`, async () => {
    const { c, refs, handles } = fixture(true, false)
    c.registerRefs({ [name]: refs[name] } as any)
    const show = c[name].show()
    expect(refs[name].style.width).toBe(name === "backButton" ? "60%" : "40%")
    await show
    const hide = c[name].hide()
    expect(refs[name].style.width).toBe("0")
    await hide
    expect(handles[name]).toHaveLength(0)
  })
}
it("0255 Back and Skip ownership remain independent", async () => {
  const { c, handles } = fixture()
  const back = c.backButton.show(),
    skip = c.skipButton.show()
  await flush()
  const backHide = c.backButton.hide()
  await flush()
  expect(handles.backButton[0].cancel).toHaveBeenCalledOnce()
  expect(handles.skipButton[0].cancel).not.toHaveBeenCalled()
  handles.backButton[0].reject(abort())
  handles.backButton[1].resolve()
  handles.skipButton[0].resolve()
  await Promise.all([back, backHide, skip])
})
