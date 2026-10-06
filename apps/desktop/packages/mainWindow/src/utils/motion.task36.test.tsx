import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { applyReducedMotion } from "./reducedMotion"
import {
  Popover,
  PopoverTrigger,
  PopoverContent
} from "../../../../../../packages/ui/src/Popover"
import { News } from "../../../../../../packages/ui/src/News"
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem
} from "../../../../../../packages/ui/src/DropdownMenu"
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem
} from "../../../../../../packages/ui/src/Select"
import stylesheet from "../../../../../../packages/ui/src/style.css?raw"

let dispose: (() => void) | undefined
const flush = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve()
  await new Promise((r) => setTimeout(r, 40))
}
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  document.body.className = ""
  vi.useRealTimers()
  vi.restoreAllMocks()
})
it("reduced motion covers body portal, closes real popup, restores focus and releases owner class", async () => {
  const root = document.createElement("div")
  root.id = "root"
  const overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(root, overlay)
  const style = document.createElement("style")
  style.textContent = stylesheet
  root.append(style)
  const [motion, setMotion] = createSignal(true)
  const computed = window.getComputedStyle.bind(window)
  vi.spyOn(window, "getComputedStyle").mockImplementation((element, pseudo) => {
    const style = computed(element, pseudo)
    if (!element.matches(".potato-pc *")) return style
    return new Proxy(style, {
      get: (target, name) =>
        name === "animationName"
          ? "none"
          : name === "animationDuration"
            ? "0s"
            : Reflect.get(target, name)
    })
  })
  const Content = () => {
    applyReducedMotion(motion)
    return (
      <Popover>
        <PopoverTrigger>open popup</PopoverTrigger>
        <PopoverContent>
          <button>choice</button>
        </PopoverContent>
      </Popover>
    )
  }
  dispose = render(() => <Content />, root)
  const trigger = root.querySelector("button")!
  trigger.focus()
  trigger.click()
  await flush()
  const popup = document.querySelector("[data-expanded][role=dialog]")!
  expect(popup).toBeTruthy()
  expect(root.contains(popup)).toBe(false)
  expect(popup.matches(".potato-pc *")).toBe(true)
  expect(trigger.matches(".potato-pc *")).toBe(true)
  expect(overlay.matches(".potato-pc *")).toBe(true)
  popup.querySelector<HTMLButtonElement>("button[aria-label=Dismiss]")!.click()
  await flush()
  expect(document.querySelector("[role=dialog]")).toBeNull()
  expect(document.activeElement).toBe(trigger)
  trigger.click()
  await flush()
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
  )
  await flush()
  expect(document.querySelector("[role=dialog]")).toBeNull()
  setMotion(false)
  expect(document.body.classList.contains("potato-pc")).toBe(false)
  setMotion(true)
  dispose()
  dispose = undefined
  expect(document.body.classList.contains("potato-pc")).toBe(false)
})
it.each(["menu", "select"])(
  "reduced motion %s portal supports selection and presence cleanup",
  async (kind) => {
    const root = document.createElement("div")
    root.id = "root"
    document.body.append(root)
    const selected = vi.fn(),
      computed = window.getComputedStyle.bind(window)
    vi.spyOn(window, "getComputedStyle").mockImplementation(
      (element, pseudo) =>
        new Proxy(computed(element, pseudo), {
          get: (target, name) =>
            name === "animationName"
              ? "none"
              : name === "animationDuration"
                ? "0s"
                : Reflect.get(target, name)
        })
    )
    const Content = () => {
      applyReducedMotion(() => true)
      return kind === "menu" ? (
        <DropdownMenu>
          <DropdownMenuTrigger>open</DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem onSelect={selected}>choice</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ) : (
        <Select
          options={["choice"]}
          onChange={selected}
          itemComponent={(p) => <SelectItem item={p.item}>choice</SelectItem>}
        >
          <SelectTrigger>open</SelectTrigger>
          <SelectContent />
        </Select>
      )
    }
    dispose = render(() => <Content />, root)
    const trigger = root.querySelector("button")!
    trigger.focus()
    trigger.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })
    )
    await flush()
    const option = document.querySelector<HTMLElement>(
      kind === "menu" ? "[role=menuitem]" : "[role=option]"
    )!
    expect(option).toBeTruthy()
    expect(root.contains(option)).toBe(false)
    expect(option.matches(".potato-pc *")).toBe(true)
    option.focus()
    option.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
    await flush()
    expect(selected).toHaveBeenCalledTimes(1)
    expect(
      document.querySelector(kind === "menu" ? "[role=menu]" : "[role=listbox]")
    ).toBeNull()
    expect(document.activeElement).toBe(trigger)
  }
)
const slides = Array.from({ length: 3 }, (_, i) => ({
  image: "",
  title: `Slide ${i + 1}`,
  description: "fixture",
  date: "2026-01-01"
}))
function news(disableAutoRotation = true) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <News slides={slides} disableAutoRotation={disableAutoRotation} />,
    root
  )
  const right = root.querySelector<HTMLElement>(".right-5")!
  const left = root.querySelector<HTMLElement>(".left-5")!
  const track = root.querySelector<HTMLElement>(".flex.h-full")!
  return { root, right, left, track }
}
function transition(
  track: HTMLElement,
  propertyName = "transform",
  type = "transitionend"
) {
  const event = new Event(type, { bubbles: true })
  Object.defineProperty(event, "propertyName", { value: propertyName })
  track.dispatchEvent(event)
}
it("carousel missing completion event has bounded fallback so second arrow works", async () => {
  vi.useFakeTimers()
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "transform",
        transitionDuration: "0.45s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news()
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
  await vi.advanceTimersByTimeAsync(1000)
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-300%)")
})
it("already selected indicator leaves the next arrow immediately available", () => {
  vi.useFakeTimers()
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "transform",
        transitionDuration: "0.45s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news()
  f.root.querySelectorAll<HTMLElement>(".h-2.w-2")[0].click()
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
})
it("carousel reduced/zero transition completes immediately and preserves sentinel wraparound", () => {
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "none",
        transitionDuration: "0s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news()
  f.left.click()
  expect(f.track.style.transform).toBe("translateX(-300%)")
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-100%)")
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
})
it("carousel relevant transform completion unlocks; child/unrelated events cannot unlock motion", async () => {
  vi.useFakeTimers()
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "transform",
        transitionDuration: "0.45s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news()
  f.right.click()
  await vi.advanceTimersByTimeAsync(450)
  transition(f.track, "opacity")
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
  transition(f.track.firstElementChild as HTMLElement)
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
  transition(f.track)
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-300%)")
  expect(f.track.style.transition).toBe("transform 450ms ease")
})
it("indicator replacement retires the old fallback and old event while its own cancellation completes", async () => {
  vi.useFakeTimers()
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "transform",
        transitionDuration: "0.45s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news()
  f.right.click()
  const oldEnd = new Event("transitionend", { bubbles: true })
  Object.defineProperty(oldEnd, "propertyName", { value: "transform" })
  await vi.advanceTimersByTimeAsync(300)
  const indicators = f.root.querySelectorAll<HTMLElement>(".h-2.w-2")
  indicators[0].click()
  f.track.dispatchEvent(oldEnd)
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-100%)")
  await vi.advanceTimersByTimeAsync(220)
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-100%)")
  vi.mocked(window.getComputedStyle).mockReturnValueOnce({
    transitionProperty: "none",
    transitionDuration: "0s",
    transitionDelay: "0s"
  } as CSSStyleDeclaration)
  transition(f.track, "transform", "transitioncancel")
  f.right.click()
  expect(f.track.style.transform).toBe("translateX(-200%)")
  dispose!()
  dispose = undefined
  expect(vi.getTimerCount()).toBe(0)
})
it("reduced body preference completes repeated arrows without clocks and slide links remain actionable", () => {
  document.body.classList.add("potato-pc")
  const click = vi.fn(),
    root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <News slides={slides} disableAutoRotation onClick={click} />,
    root
  )
  const right = root.querySelector<HTMLElement>(".right-5")!,
    track = root.querySelector<HTMLElement>(".flex.h-full")!
  right.click()
  right.click()
  right.click()
  expect(track.style.transform).toBe("translateX(-100%)")
  root.querySelector<HTMLElement>(".absolute.box-border")!.click()
  expect(click).toHaveBeenCalledTimes(1)
})
it("automatic rotation wraps, indicator navigation resets its interval and disposal retires its clocks", async () => {
  vi.useFakeTimers()
  vi.spyOn(window, "getComputedStyle").mockImplementation(
    () =>
      ({
        transitionProperty: "none",
        transitionDuration: "0s",
        transitionDelay: "0s"
      }) as CSSStyleDeclaration
  )
  const f = news(false)
  await vi.advanceTimersByTimeAsync(5000)
  expect(f.track.style.transform).toBe("translateX(-200%)")
  await vi.advanceTimersByTimeAsync(5000)
  expect(f.track.style.transform).toBe("translateX(-300%)")
  await vi.advanceTimersByTimeAsync(5000)
  expect(f.track.style.transform).toBe("translateX(-100%)")
  f.root.querySelectorAll<HTMLElement>(".h-2.w-2")[2].click()
  expect(f.track.style.transform).toBe("translateX(-300%)")
  await vi.advanceTimersByTimeAsync(5000)
  expect(f.track.style.transform).toBe("translateX(-100%)")
  dispose!()
  dispose = undefined
  expect(vi.getTimerCount()).toBe(0)
})
