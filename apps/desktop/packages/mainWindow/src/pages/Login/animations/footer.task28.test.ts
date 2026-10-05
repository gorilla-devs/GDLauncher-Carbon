import { expect, it, vi } from "vitest"
import { AnimationControllerImpl } from "./AnimationController"
function fixture(reduced = true) {
  const make = () => ({
    style: { width: "0", margin: "0", opacity: "1", transform: "" },
    animate: vi.fn((frames: any) => {
      const element = {}
      return {
        finished: Promise.resolve().then(() => element),
        cancel: vi.fn(),
        frames
      }
    })
  })
  const refs = {
    sidebar: make(),
    video: make(),
    backgroundBlur: make(),
    loadingSpinner: make(),
    backButton: make(),
    skipButton: make()
  }
  const controller = new AnimationControllerImpl(reduced)
  controller.registerRefs(refs as any)
  return { controller, refs, make }
}
it("GPT-0123 applies reduced footer geometry immediately across show/hide/show", async () => {
  const { controller: c, refs: r } = fixture()
  for (const [name, width, margin] of [
    ["backButton", "60%", "0 1rem 0 0"],
    ["skipButton", "40%", "0"]
  ] as const) {
    await c[name].show()
    expect(r[name].style).toMatchObject({ width, margin })
    await c[name].hide()
    expect(r[name].style).toMatchObject({ width: "0", margin: "0" })
    await c[name].show()
    expect(r[name].style.width).toBe(width)
    expect(r[name].animate).not.toHaveBeenCalled()
  }
  await c.sidebar.slideIn()
  expect(r.sidebar.style.transform).toBe("translateX(0%)")
  expect(r.loadingSpinner.style.opacity).toBe("0")
})
it("GPT-0123 a missing footer ref can retry after registration", async () => {
  const c = new AnimationControllerImpl(true)
  await c.backButton.show()
  await c.skipButton.show()
  const { refs: r } = fixture()
  c.registerRefs(r as any)
  await c.backButton.show()
  await c.skipButton.show()
  expect(r.backButton.style.width).toBe("60%")
  expect(r.skipButton.style.width).toBe("40%")
})
it("ordinary footer animations retain final keyframes", async () => {
  const { controller: c, refs: r } = fixture(false)
  await c.backButton.show()
  await c.skipButton.show()
  expect(r.backButton.animate.mock.calls[0][0][1]).toEqual({
    width: "60%",
    margin: "0 1rem 0 0"
  })
  expect(r.skipButton.animate.mock.calls[0][0][1]).toEqual({
    width: "40%",
    margin: "0"
  })
})
it("reduced footer geometry cancels its owned animation before publication", async () => {
  const { controller: c, refs: r } = fixture()
  const cancel = vi.fn()
  Object.assign(c, {
    backButtonAnimation: { cancel },
    skipButtonAnimation: { cancel }
  })
  await c.backButton.show()
  await c.skipButton.show()
  expect(cancel).toHaveBeenCalledTimes(2)
  expect(r.backButton.style.width).toBe("60%")
  expect(r.skipButton.style.width).toBe("40%")
})
