import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { createStore } from "solid-js/store"
import { render } from "solid-js/web"
import JavaPathAutoComplete from "./JavaPathAutoComplete"
import AuthorAvatars from "./AuthorAvatars"
import { SpotlightOverlay } from "./Onboarding/SpotlightOverlay"
import {
  OnboardingProvider,
  registerTip,
  unregisterTip,
  useOnboarding
} from "./Onboarding/OnboardingContext"

const c = vi.hoisted(() => ({
  java: undefined as any,
  pending: false,
  marks: [] as string[],
  firstLaunch: false,
  seen: [] as string[],
  register: vi.fn(),
  onboarding: undefined as any
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (options: any) => {
      const key = options().queryKey[0]
      return {
        get data() {
          return key === "java.getAvailableJavas"
            ? c.java
            : key === "settings.isFirstLaunch"
              ? c.firstLaunch
              : c.seen
        }
      }
    },
    createMutation: (options: any) => ({
      get isPending() {
        return c.pending
      },
      mutate: (input: any) =>
        options().mutationKey[0] === "settings.markOnboardingTipSeen"
          ? c.marks.push(input)
          : c.register(input)
    })
  }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => (
    <span>
      {p.key} {p.options?.role}
    </span>
  ),
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../packages/ui/src/Input")),
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div data-tooltip>{p.children}</div>
}))
const disposals: (() => void)[] = []
afterEach(() => {
  disposals.splice(0).forEach((d) => d())
  document.body.textContent = ""
  Object.defineProperty(c, "java", {
    configurable: true,
    writable: true,
    value: {}
  })
  Object.defineProperty(c, "pending", {
    configurable: true,
    writable: true,
    value: false
  })
  c.marks = []
  c.seen = []
  c.firstLaunch = false
  c.register.mockReset()
  unregisterTip("search-input-syntax")
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(component, root)
  disposals.push(dispose)
  return { root, dispose }
}
function inputValue(input: HTMLInputElement, value: string) {
  input.value = value
  input.dispatchEvent(new Event("input", { bubbles: true }))
}
it.each([true, false])(
  "D5 mount default first clear stays empty with blur-only=%s",
  async (blurOnly) => {
    const update = vi.fn(),
      selection = vi.fn()
    const { root } = mount(() => (
      <JavaPathAutoComplete
        defaultValue="synthetic/old/java"
        updateValueOnlyOnBlur={blurOnly}
        updateValue={update}
        onSelection={selection}
      />
    ))
    const input = root.querySelector("input")!
    update.mockClear()
    inputValue(input, "")
    await flush()
    expect(input.value).toBe("")
    if (blurOnly) expect(update).not.toHaveBeenCalled()
    input.dispatchEvent(new FocusEvent("blur"))
    expect(update).toHaveBeenLastCalledWith(null, "")
    expect(selection).toHaveBeenLastCalledWith({
      id: null,
      path: "",
      pending: false
    })
    inputValue(input, "synthetic/new/java")
    expect(input.value).toBe("synthetic/new/java")
    inputValue(input, "")
    expect(input.value).toBe("")
  }
)
it("D5 pristine delayed default hydrates once and first clear remains empty", async () => {
  const [value, setValue] = createSignal<string>()
  const { root } = mount(() => <JavaPathAutoComplete defaultValue={value()} />)
  const input = root.querySelector("input")!
  input.dispatchEvent(new FocusEvent("focus"))
  setValue("synthetic/delayed/java")
  await flush()
  expect(input.value).toBe("synthetic/delayed/java")
  inputValue(input, "")
  setValue("synthetic/second/java")
  await flush()
  expect(input.value).toBe("")
})
it.each(["", "synthetic/edited/java"])(
  "D5 deliberate input %j revokes delayed hydration",
  async (edit) => {
    const [value, setValue] = createSignal<string>()
    const { root } = mount(() => (
      <JavaPathAutoComplete defaultValue={value()} />
    ))
    const input = root.querySelector("input")!
    inputValue(input, edit)
    setValue("synthetic/late/java")
    await flush()
    expect(input.value).toBe(edit)
  }
)
it("Task32 exact validity and pending selection contract survives real Java option selection", async () => {
  const [javas, setJavas] = createSignal<any>({
    synthetic: [
      {
        id: "valid",
        path: "synthetic/valid/java",
        type: "custom",
        version: "21",
        isValid: true
      },
      {
        id: "invalid",
        path: "synthetic/invalid/java",
        type: "custom",
        version: "17",
        isValid: false
      }
    ]
  })
  const [pending, setPending] = createSignal(false)
  Object.defineProperty(c, "java", {
    configurable: true,
    get: javas,
    set: () => {}
  })
  Object.defineProperty(c, "pending", {
    configurable: true,
    get: pending,
    set: () => {}
  })
  const selection = vi.fn(),
    update = vi.fn()
  const { root } = mount(() => (
    <JavaPathAutoComplete
      updateValueOnlyOnBlur
      onSelection={selection}
      updateValue={update}
    />
  ))
  const input = root.querySelector("input")!
  input.dispatchEvent(new FocusEvent("focus"))
  await flush()
  const option = [...document.querySelectorAll("li")].find((el) =>
    el.textContent?.includes("synthetic/valid/java")
  )!
  option.click()
  await flush()
  expect(input.value).toBe("synthetic/valid/java")
  expect(selection).toHaveBeenLastCalledWith({
    id: "valid",
    path: "synthetic/valid/java",
    pending: false
  })
  expect(update).not.toHaveBeenCalled()
  input.dispatchEvent(new FocusEvent("blur"))
  expect(update).toHaveBeenLastCalledWith("valid", "synthetic/valid/java")
  setPending(true)
  expect(selection).toHaveBeenLastCalledWith({
    id: null,
    path: "synthetic/valid/java",
    pending: true
  })
  setPending(false)
  inputValue(input, "synthetic/invalid/java")
  expect(selection).toHaveBeenLastCalledWith({
    id: null,
    path: "synthetic/invalid/java",
    pending: false
  })
  inputValue(input, "synthetic/new/java")
  setJavas({
    synthetic: [
      {
        id: "late",
        path: "synthetic/valid/java",
        isValid: true,
        type: "custom",
        version: "21"
      }
    ]
  })
  expect(selection).toHaveBeenLastCalledWith({
    id: null,
    path: "synthetic/new/java",
    pending: false
  })
  Object.defineProperty(c, "java", {
    configurable: true,
    writable: true,
    value: {}
  })
  Object.defineProperty(c, "pending", {
    configurable: true,
    writable: true,
    value: false
  })
})
function spotlight() {
  registerTip("search-input-syntax", {
    id: "search-input-syntax",
    trigger: "onClick",
    title: "Search",
    description: "Synthetic syntax tip",
    placement: "bottom"
  })
  function Child() {
    c.onboarding = useOnboarding()
    return <SpotlightOverlay />
  }
  const mounted = mount(() => (
    <OnboardingProvider>
      <Child />
    </OnboardingProvider>
  ))
  c.onboarding.showTip("search-input-syntax", new DOMRect(30, 30, 100, 40))
  return mounted
}
it.each(["close", "got-it", "escape", "overlay"])(
  "D4 actual %s dismissal clears active owner and marks exactly once",
  async (route) => {
    spotlight()
    await flush()
    expect(document.querySelector('[role="dialog"]')).not.toBeNull()
    if (route === "close")
      (document.querySelector('button[aria-label="Dismiss"]') ||
        document.querySelector("button.absolute"))!.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      )
    else if (route === "got-it")
      [...document.querySelectorAll("button")]
        .find((b) => b.textContent?.includes("got_it"))!
        .click()
    else if (route === "escape")
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
      )
    else document.querySelector<HTMLElement>(".fixed.inset-0")!.click()
    await flush()
    expect(c.onboarding.activeTip()).toBeNull()
    expect(c.marks).toEqual(["search-input-syntax"])
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    expect(document.querySelector(".fixed.inset-0")).toBeNull()
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }))
    expect(c.marks).toHaveLength(1)
  }
)
it("D4 disposal cleans the Escape listener; seen and first-launch guards remain", async () => {
  const { dispose } = spotlight()
  await flush()
  dispose()
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }))
  expect(c.marks).toEqual([])
  c.firstLaunch = true
  spotlight()
  expect(c.onboarding.activeTip()).toBeNull()
  c.firstLaunch = false
  c.seen = ["search-input-syntax"]
  spotlight()
  expect(c.onboarding.activeTip()).toBeNull()
})
const author = (id: string, name: string, avatarUrl?: string) => ({
  id,
  name,
  avatarUrl,
  platform: "modrinth" as const,
  role: "Developer",
  url: `https://example.invalid/${id}`
})
it("D6 one failed author image shows its own initials, tooltip, role and external link while peers survive", () => {
  const open = vi.fn()
  window.openExternalLink = open
  const { root } = mount(() => (
    <AuthorAvatars
      authors={[
        author("a", "Alice Author", "synthetic-A.png"),
        author("b", "Bob Builder", "synthetic-B.png")
      ]}
    />
  ))
  const failed = root.querySelector('img[alt="Alice Author avatar"]')!
  failed.dispatchEvent(new Event("error"))
  expect(root.querySelector('img[alt="Alice Author avatar"]')).toBeNull()
  expect(root.querySelector('img[alt="Bob Builder avatar"]')).not.toBeNull()
  const initials = [...root.querySelectorAll("div.font-bold")].find(
    (el) => el.textContent === "AA"
  )!
  expect(initials).toBeDefined()
  expect(root.querySelector("[data-tooltip]")?.textContent).toContain(
    "Alice Author"
  )
  expect(root.querySelector("[data-tooltip]")?.textContent).toContain(
    "Developer"
  )
  initials.dispatchEvent(new MouseEvent("click", { bubbles: true }))
  expect(open).toHaveBeenCalledWith("https://example.invalid/a")
})
it("D6 URL replacement and ABA can retry; late old-image failures cannot poison the current URL", () => {
  const [authors, setAuthors] = createStore([
    author("a", "Alice Author", "synthetic-A.png")
  ])
  const { root } = mount(() => <AuthorAvatars authors={authors} />)
  const old = root.querySelector('img[alt="Alice Author avatar"]')!
  old.dispatchEvent(new Event("error"))
  expect(root.textContent).toContain("AA")
  setAuthors(0, "avatarUrl", "synthetic-B.png")
  const replacement = root.querySelector('img[alt="Alice Author avatar"]')!
  expect(replacement?.getAttribute("src")).toBe("synthetic-B.png")
  old.dispatchEvent(new Event("error"))
  expect(root.querySelector('img[alt="Alice Author avatar"]')).toBe(replacement)
  setAuthors(0, "avatarUrl", "synthetic-A.png")
  const returned = root.querySelector('img[alt="Alice Author avatar"]')!
  expect(returned).not.toBe(old)
  old.dispatchEvent(new Event("error"))
  expect(root.querySelector('img[alt="Alice Author avatar"]')).toBe(returned)
  returned.dispatchEvent(new Event("error"))
  expect(root.querySelector('img[alt="Alice Author avatar"]')).toBeNull()
  setAuthors(0, author("c", "Carol Coder", "synthetic-A.png"))
  replacement.dispatchEvent(new Event("error"))
  returned.dispatchEvent(new Event("error"))
  const current = root.querySelector('img[alt="Carol Coder avatar"]')!
  expect(current).not.toBeNull()
  current.dispatchEvent(new Event("error"))
  expect(root.textContent).toContain("CC")
})
it("D6 missing-avatar initials, valid images, CurseForge link policy and overflow stay available", () => {
  const open = vi.fn()
  window.openExternalLink = open
  const { root } = mount(() => (
    <AuthorAvatars
      maxDisplay={2}
      authors={[
        { ...author("a", "Alice Author"), platform: "curseforge" },
        author("b", "Bob Builder", "synthetic-B.png"),
        author("c", "Carol Coder")
      ]}
    />
  ))
  expect(root.textContent).toContain("AA")
  expect(root.textContent).toContain("+1")
  expect(root.querySelector('img[alt="Bob Builder avatar"]')).not.toBeNull()
  const initials = [...root.querySelectorAll("div.font-bold")].find(
    (el) => el.textContent === "AA"
  )!
  initials.dispatchEvent(new MouseEvent("click", { bubbles: true }))
  expect(open).not.toHaveBeenCalled()
})
