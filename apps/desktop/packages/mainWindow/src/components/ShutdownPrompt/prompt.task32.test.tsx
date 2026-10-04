import { afterEach, expect, it, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { render } from "solid-js/web"
import type { ShutdownView } from "../../../../main/utils/shutdownFlow"
import type { ShutdownPromptMessage } from "../../../../main/utils/shutdownPrompt"

vi.mock("@gd/ui", () => ({
  Button: (props: any) => (
    <button data-testid={props["data-testid"]} onClick={props.onClick}>
      {props.children}
    </button>
  )
}))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => `t(${key})`]
}))

const { default: ShutdownPrompt } = await import(".")

let dispose = () => {}
afterEach(() => {
  dispose()
  document.body.innerHTML = ""
  vi.restoreAllMocks()
})
/**
 * The launcher page's own #overlay, as index.html ships it: hidden
 * (display: none) unless a modal or an expanded folder is open.
 */
function shippedOverlay() {
  const html = fs.readFileSync(
    path.resolve(__dirname, "../../../index.html"),
    "utf8"
  )
  const markup = /<div\s+id="overlay"[\s\S]*?<\/div>/.exec(html)?.[0]
  expect(markup, "index.html has no #overlay").toBeTruthy()
  const holder = document.createElement("div")
  holder.innerHTML = markup!
  return holder.firstElementChild as HTMLElement
}
function mount() {
  const overlay = shippedOverlay()
  document.body.append(overlay)
  let deliver!: (message: ShutdownPromptMessage) => void
  const answers: [number, string][] = []
  const texts: Record<string, string>[] = []
  const shown: number[] = []
  const stop = vi.fn()
  Object.assign(window, {
    onShutdownPrompt: (cb: typeof deliver) => {
      deliver = cb
      return stop
    },
    answerShutdownPrompt: (id: number, choice: string) =>
      answers.push([id, choice]),
    shownShutdownPrompt: (id: number) => shown.push(id),
    setShutdownPromptText: (text: Record<string, string>) => texts.push(text)
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <ShutdownPrompt />, root)
  return {
    deliver: (m: ShutdownPromptMessage) => deliver(m),
    answers,
    texts,
    shown,
    stop,
    overlay
  }
}
/** jsdom has no layout: the prompt shows unless it or a parent is display:none. */
function displayed(element: Element | null) {
  for (let node = element; node; node = node.parentElement)
    if (getComputedStyle(node).display === "none") return false
  return !!element
}
const blocked: ShutdownView = {
  action: "quit",
  stage: "blocked",
  reason: "unresolved",
  blockers: [
    { kind: "deletion", name: "delete instance 12" },
    { kind: "server", name: "server 7 run" }
  ],
  canWait: true,
  finishing: false,
  serversStopped: true
}
const prompt = () =>
  document.querySelector<HTMLElement>('[data-testid="shutdown-prompt"]')

it("shows over the launcher while no modal is open, outside the hidden #overlay, and tells main it is on screen", () => {
  const ui = mount()
  expect(ui.overlay.style.display).toBe("none")
  ui.deliver({ id: 7, view: { ...blocked, stage: "waiting", reason: null } })
  expect(prompt()).not.toBeNull()
  expect(ui.overlay.contains(prompt())).toBe(false)
  expect(displayed(prompt())).toBe(true)
  expect(ui.shown).toEqual([7])
})

it("does not tell main a prompt is on screen when the page cannot show it", () => {
  // Main then asks with a native dialog instead.
  ;(HTMLElement.prototype as any).checkVisibility = vi.fn(() => false)
  try {
    const ui = mount()
    ui.deliver({ id: 3, view: blocked })
    expect(prompt()).not.toBeNull()
    expect(ui.shown).toEqual([])
  } finally {
    delete (HTMLElement.prototype as any).checkVisibility
  }
})

it("shows what keeps GDLauncher from closing with Try again, Quit now and Keep GDLauncher open", () => {
  const ui = mount()
  expect(prompt()).toBeNull()
  ui.deliver({ id: 4, view: blocked })
  expect(prompt()?.dataset.stage).toBe("blocked")
  const text = prompt()!.textContent
  for (const key of [
    "title_quit_blocked",
    "body_unresolved",
    "blocker_deletion",
    "blocker_server",
    "deletion_warning",
    "force_note",
    "servers_stopped",
    "try_again",
    "force_quit",
    "keep_open"
  ])
    expect(text).toContain(`app:_trn_shutdown_prompt.${key}`)
  // Internal owner names are never shown.
  expect(text).not.toContain("delete instance 12")
  document
    .querySelector<HTMLButtonElement>('[data-testid="shutdown-prompt-keep"]')!
    .click()
  expect(ui.answers).toEqual([[4, "keep"]])
  expect(prompt()).toBeNull()
})

it("says when it is too late to keep GDLauncher open, and offers no Keep GDLauncher open", () => {
  const ui = mount()
  ui.deliver({
    id: 5,
    view: {
      ...blocked,
      action: "restart",
      stage: "waiting",
      reason: null,
      canWait: false,
      finishing: true
    }
  })
  const text = prompt()!.textContent
  expect(text).toContain("app:_trn_shutdown_prompt.too_late_to_keep")
  expect(text).not.toContain("app:_trn_shutdown_prompt.servers_stopped")
  expect(
    document.querySelector('[data-testid="shutdown-prompt-keep"]')
  ).toBeNull()
  expect(
    document.querySelector('[data-testid="shutdown-prompt-force"]')
  ).not.toBeNull()
})

it("hides Wait when only a restart clears what is in the way, and main can close the prompt", () => {
  const ui = mount()
  ui.deliver({
    id: 1,
    view: {
      ...blocked,
      action: "update",
      blockers: [{ kind: "unconfirmed", name: "lost worker" }],
      canWait: false
    }
  })
  expect(
    document.querySelector('[data-testid="shutdown-prompt-wait"]')
  ).toBeNull()
  expect(prompt()!.textContent).toContain(
    "app:_trn_shutdown_prompt.force_update"
  )
  // A close for another prompt changes nothing; its own closes it.
  ui.deliver({ id: 2, view: null })
  expect(prompt()).not.toBeNull()
  ui.deliver({ id: 1, view: null })
  expect(prompt()).toBeNull()
  expect(ui.answers).toEqual([])
})

it("sends main the prompt's text in the user's language and unsubscribes on unmount", () => {
  const ui = mount()
  expect(ui.texts).toHaveLength(1)
  expect(ui.texts[0].keep_open).toBe("t(app:_trn_shutdown_prompt.keep_open)")
  dispose()
  dispose = () => {}
  expect(ui.stop).toHaveBeenCalledOnce()
})

it("an update to the prompt on screen changes what it says without showing it anew, and one for an answered prompt is ignored", () => {
  const ui = mount()
  const waiting: ShutdownView = { ...blocked, stage: "waiting", reason: null }
  ui.deliver({ id: 6, view: waiting })
  const shownElement = prompt()
  const rows = () =>
    [
      ...document.querySelectorAll(
        '[data-testid="shutdown-prompt-blockers"] li'
      )
    ].map((row) => row.getAttribute("data-testid"))
  expect(rows()).toEqual([
    "shutdown-prompt-row-blocker_deletion",
    "shutdown-prompt-row-blocker_server"
  ])
  // The server is done; the deletion goes on.
  ui.deliver({
    id: 6,
    view: {
      ...waiting,
      blockers: [{ kind: "deletion", name: "delete instance 12" }]
    }
  })
  expect(prompt()).toBe(shownElement)
  expect(rows()).toEqual(["shutdown-prompt-row-blocker_deletion"])
  expect(ui.shown).toEqual([6])
  // An older prompt's update does not replace the one on screen.
  ui.deliver({ id: 5, view: blocked })
  expect(rows()).toEqual(["shutdown-prompt-row-blocker_deletion"])
  document
    .querySelector<HTMLButtonElement>('[data-testid="shutdown-prompt-force"]')!
    .click()
  expect(ui.answers).toEqual([[6, "force"]])
  // Main sent an update before it got the answer: the prompt stays closed.
  ui.deliver({ id: 6, view: waiting })
  expect(prompt()).toBeNull()
  // The next prompt is shown.
  ui.deliver({ id: 7, view: blocked })
  expect(prompt()).not.toBeNull()
  expect(ui.shown).toEqual([6, 7])
})

it("says GDLauncher is stopping, with nothing to choose, until main closes it", () => {
  const ui = mount()
  ui.deliver({
    id: 8,
    view: {
      ...blocked,
      action: "restart",
      stage: "stopping",
      reason: null,
      blockers: [],
      canWait: false
    }
  })
  expect(prompt()?.dataset.stage).toBe("stopping")
  const text = prompt()!.textContent
  expect(text).toContain("app:_trn_shutdown_prompt.title_restart_waiting")
  expect(text).toContain("app:_trn_shutdown_prompt.body_stopping")
  expect(document.querySelectorAll("button")).toHaveLength(0)
  expect(ui.shown).toEqual([8])
  ui.deliver({ id: 8, view: null })
  expect(prompt()).toBeNull()
})
