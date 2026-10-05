import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal, untrack } from "solid-js"
import { createStore } from "solid-js/store"
import { LogEntryLevel, LogEntrySourceKind } from "@/utils/logs"

const control = vi.hoisted(() => ({
  finder: null as any,
  mounts: 0,
  setIndex: null as any,
  initialOpen: false
}))
vi.mock("@gd/ui", () => import("@/test/task25/ui"))
vi.mock("@gd/i18n", () => ({ Trans: (props: any) => <span>{props.key}</span> }))
vi.mock(".", async () => {
  const [full, setFull] = createSignal(false)
  return { isFullScreen: full, setIsFullScreen: setFull }
})
vi.mock("@/hooks/useKeyboardShortcut", () => ({ default: vi.fn() }))
vi.mock("./components/FullscreenToggle", () => ({ default: () => <div /> }))
vi.mock("./components/LogsOptions", () => ({ default: () => <div /> }))
vi.mock("./LogsFinders", () => ({
  default: (props: any) => {
    control.finder = props
    if (control.initialOpen) props.setOpen(true)
    return <div />
  }
}))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (props: any) => {
    const [index, setIndex] = createSignal(0)
    control.setIndex = setIndex
    control.mounts++
    return untrack(() => props.children(props.data[0], index))
  }
}))
import LogsContent from "./LogsContent"
let root: HTMLDivElement
let dispose: () => void
afterEach(() => {
  dispose?.()
  root?.remove()
  control.initialOpen = false
  control.mounts = 0
})
function mount(results?: any[]) {
  const [state, setState] = createStore({ results })
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <LogsContent
        logs={[
          {
            message: "😀foo foo",
            level: LogEntryLevel.Info,
            sourceKind: LogEntrySourceKind._StdOut,
            logger: "N/A",
            thread: "N/A",
            timestamp: 0
          }
        ]}
        query={{
          query: null,
          matchCase: false,
          matchWholeWord: false,
          useRegex: false
        }}
        setQuery={vi.fn()}
        logSearchResults={state.results}
        isActive={false}
        adopted={false}
        isLoading={false}
        scrollToBottom={vi.fn()}
        onScroll={vi.fn()}
        assignScrollBottomRef={vi.fn()}
        assignLogsContentRef={vi.fn()}
        assignVirtualizerRef={vi.fn()}
        newLogsCount={0}
        autoFollowPreference={true}
        setAutoFollowPreference={vi.fn()}
        scrollToIndex={vi.fn()}
        isIndexLoaded={() => true}
      />
    ),
    root
  )
  return setState
}
const matches = [
  { entry_index: 0, pos: 2, len: 3 },
  { entry_index: 0, pos: 6, len: 3 }
]
test("an already mounted untracked row updates real UTF16 formatter spans and active styling without remount", () => {
  const setState = mount()
  const row = root.querySelector(".break-words")
  expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(0)
  control.finder.setOpen(true)
  setState("results", matches)
  control.finder.setCurrentResultIndex(1)
  let spans = root.querySelectorAll('[id^="finder-result-"]')
  expect([...spans].map((span) => span.textContent)).toEqual(["foo", "foo"])
  expect(spans[0].classList.contains("outline")).toBe(false)
  expect(spans[1].classList.contains("outline")).toBe(true)
  control.finder.setCurrentResultIndex(0)
  expect(spans[0].classList.contains("outline")).toBe(true)
  expect(spans[1].classList.contains("outline")).toBe(false)
  setState("results", [
    { entry_index: 9, pos: 0, len: 1 },
    { entry_index: 1, pos: 2, len: 3 }
  ])
  control.setIndex(1)
  control.finder.setCurrentResultIndex(1)
  spans = root.querySelectorAll('[id^="finder-result-"]')
  expect(spans).toHaveLength(1)
  expect(spans[0].id).toBe("finder-result-1")
  expect(spans[0].textContent).toBe("foo")
  expect(spans[0].classList.contains("outline")).toBe(true)
  control.finder.setOpen(false)
  expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(0)
  control.finder.setOpen(true)
  expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(1)
  expect(root.querySelector(".break-words")).toBe(row)
  expect(control.mounts).toBe(1)
})
test("mount-after-search and no-match controls retain normal formatter content", () => {
  control.initialOpen = true
  const setState = mount(matches)
  expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(2)
  setState("results", [])
  expect(root.querySelectorAll('[id^="finder-result-"]')).toHaveLength(0)
  expect(root.textContent).toContain("😀foo foo")
})
