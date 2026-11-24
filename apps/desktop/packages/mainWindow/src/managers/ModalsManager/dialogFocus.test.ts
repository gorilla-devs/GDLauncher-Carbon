// @vitest-environment jsdom
import { afterEach, describe, expect, it, onTestFinished } from "vitest"
import {
  DIALOG_PORTAL_ATTR,
  createDialogFocus,
  getTabbables,
  initialFocusTarget,
  isOutsideFocusAllowed,
  shouldRestoreFocus,
  tabWrapTarget
} from "./dialogFocus"

/** Lets the trap's deferred work (first focus, restore) run. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0))
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

/** ModalLayout's header: the title, then the close button. */
const header = `<div><h2>Title</h2><button type="button" id="x" aria-label="Close"></button></div>`

const mount = (html: string) => {
  const template = document.createElement("template")
  template.innerHTML = html.trim()
  const el = template.content.firstElementChild as HTMLElement
  document.body.append(el)
  return el
}

/** A dialog element like the manager's: focusable as a fallback. */
const dialog = (content = "") =>
  mount(`<div role="dialog" tabindex="-1">${content}</div>`)

const byId = (id: string) => document.getElementById(id)!
const active = () => document.activeElement

/** Presses Tab where focus is, the way the browser would deliver it. */
const tab = (shiftKey = false) => {
  const event = new KeyboardEvent("keydown", {
    key: "Tab",
    shiftKey,
    bubbles: true,
    cancelable: true
  })
  ;(document.activeElement ?? document.body).dispatchEvent(event)
  return event
}

/** Kobalte's portalled popper content: a positioner around the content. */
const popper = (content: string, state: "open" | "closing" = "open") =>
  mount(
    `<div data-popper-positioner><div ${
      state === "open" ? "data-expanded" : "data-closed"
    }>${content}</div></div>`
  )

afterEach(() => {
  document.body.innerHTML = ""
})

describe("getTabbables", () => {
  it("keeps what Tab reaches, in DOM order", () => {
    const d = dialog(`
      <button id="a">a</button>
      <input type="hidden" />
      <button disabled>disabled</button>
      <div tabindex="-1">not in the tab order</div>
      <div id="b" tabindex="0">b</div>
      <a id="c" href="#">c</a>
      <a>no href</a>
      <div hidden><button>hidden</button></div>
      <div inert><button>inert</button></div>
      <div aria-hidden="true"><button>aria-hidden</button></div>
      <input id="d" />
    `)
    expect(getTabbables(d).map((el) => el.id)).toEqual(["a", "b", "c", "d"])
  })

  it("counts a named radio group once: its checked radio, else its first", () => {
    const d = dialog(`
      <button id="a">a</button>
      <input type="radio" name="build" id="r1" />
      <input type="radio" name="build" id="r2" checked />
      <input type="radio" name="vendor" id="v1" />
      <input type="radio" name="vendor" id="v2" />
      <input type="radio" id="lone1" />
      <input type="radio" id="lone2" />
    `)
    expect(getTabbables(d).map((el) => el.id)).toEqual([
      "a",
      "r2",
      "v1",
      "lone1",
      "lone2"
    ])
  })

  it("ignores aria-hidden outside the dialog, where Kobalte's modal popups put it", () => {
    const wrapper = mount(
      `<div aria-hidden="true"><div role="dialog" tabindex="-1" id="d"><button id="a">a</button></div></div>`
    )
    expect(wrapper.getAttribute("aria-hidden")).toBe("true")
    expect(getTabbables(byId("d")).map((el) => el.id)).toEqual(["a"])
  })
})

describe("tabWrapTarget", () => {
  const threeButtons = () =>
    dialog(
      `<button id="a">a</button><button id="b">b</button><button id="c">c</button>`
    )

  it("wraps Tab from the last control to the first", () => {
    const d = threeButtons()
    expect(tabWrapTarget(d, byId("c"), false)).toBe(byId("a"))
  })

  it("wraps Shift+Tab from the first control to the last", () => {
    const d = threeButtons()
    expect(tabWrapTarget(d, byId("a"), true)).toBe(byId("c"))
  })

  it("leaves Tab between controls to the browser", () => {
    const d = threeButtons()
    expect(tabWrapTarget(d, byId("a"), false)).toBeNull()
    expect(tabWrapTarget(d, byId("c"), true)).toBeNull()
  })

  it("enters at the edges from the dialog itself or from nowhere", () => {
    const d = threeButtons()
    expect(tabWrapTarget(d, d, false)).toBe(byId("a"))
    expect(tabWrapTarget(d, d, true)).toBe(byId("c"))
    expect(tabWrapTarget(d, document.body, false)).toBe(byId("a"))
    expect(tabWrapTarget(d, null, true)).toBe(byId("c"))
  })

  it("moves on from a control outside the tab order by DOM position", () => {
    const d = dialog(
      `<button id="a">a</button><div id="x" tabindex="-1">x</div><button id="b">b</button>`
    )
    expect(tabWrapTarget(d, byId("x"), false)).toBe(byId("b"))
    expect(tabWrapTarget(d, byId("x"), true)).toBe(byId("a"))
  })

  it("keeps focus on the dialog when it has nothing to tab to", () => {
    const d = dialog(`<p>Just text</p>`)
    expect(tabWrapTarget(d, d, false)).toBe(d)
    expect(tabWrapTarget(d, d, true)).toBe(d)
  })
})

describe("initialFocusTarget", () => {
  it("starts on the dialog itself, not its first control or the close button", () => {
    const d = dialog(`${header}<p>text</p><button id="first">first</button>`)
    expect(initialFocusTarget(d)).toBe(d)
  })

  it("starts on an autofocus control that can take focus", () => {
    const d = dialog(
      `${header}<button id="first">first</button><input id="name" autofocus />`
    )
    expect(initialFocusTarget(d)).toBe(byId("name"))
  })

  it("skips an autofocus control that can't take focus yet", () => {
    const d = dialog(
      `${header}<button id="first">first</button><button id="version" autofocus disabled>v</button>`
    )
    expect(initialFocusTarget(d)).toBe(d)
  })

  it("still tabs to the close button", () => {
    const d = dialog(`${header}<button id="first">first</button>`)
    expect(tabWrapTarget(d, byId("first"), false)).toBe(byId("x"))
  })
})

describe("isOutsideFocusAllowed", () => {
  it("allows open Kobalte popper content: listboxes, menus, popovers", () => {
    popper(`<div role="listbox" tabindex="0" id="option">option</div>`)
    expect(isOutsideFocusAllowed(byId("option"))).toBe(true)
  })

  it("doesn't allow popper content that is closing", () => {
    popper(`<div role="menuitem" tabindex="0" id="item">item</div>`, "closing")
    expect(isOutsideFocusAllowed(byId("item"))).toBe(false)
  })

  it("allows the toast region, its toasts and their buttons", () => {
    mount(`
      <section tabindex="-1" id="region" aria-label="Notifications alt+T">
        <ol tabindex="-1" data-somoto-toaster id="list">
          <li tabindex="0" data-somoto-toast id="toast"><button id="action">Undo</button></li>
        </ol>
      </section>
    `)
    for (const id of ["region", "list", "toast", "action"])
      expect(isOutsideFocusAllowed(byId(id))).toBe(true)
  })

  it("allows content a dialog portals outside its own element", () => {
    mount(
      `<div ${DIALOG_PORTAL_ATTR}><button id="dismiss">Dismiss</button></div>`
    )
    expect(isOutsideFocusAllowed(byId("dismiss"))).toBe(true)
  })

  it("doesn't allow the page underneath", () => {
    mount(`<button id="page">page</button>`)
    expect(isOutsideFocusAllowed(byId("page"))).toBe(false)
  })
})

describe("shouldRestoreFocus", () => {
  const setup = () => {
    const opener = mount(`<button id="opener">open</button>`)
    const closed = dialog(`<button id="inside">inside</button>`)
    return { opener, closed }
  }

  it("restores when focus fell to body", () => {
    const { opener, closed } = setup()
    expect(
      shouldRestoreFocus({ opener, active: document.body, closed, top: null })
    ).toBe(true)
  })

  it("restores when focus is still inside the closed dialog", () => {
    const { opener, closed } = setup()
    expect(
      shouldRestoreFocus({ opener, active: byId("inside"), closed, top: null })
    ).toBe(true)
  })

  it("doesn't restore when the opener already moved focus elsewhere", () => {
    const { opener, closed } = setup()
    const elsewhere = mount(`<button>elsewhere</button>`)
    expect(
      shouldRestoreFocus({ opener, active: elsewhere, closed, top: null })
    ).toBe(false)
  })

  it("doesn't restore to an opener that is gone or can't take focus", () => {
    const { opener, closed } = setup()
    opener.remove()
    expect(
      shouldRestoreFocus({ opener, active: document.body, closed, top: null })
    ).toBe(false)
    const disabled = mount(`<button disabled>disabled</button>`)
    expect(
      shouldRestoreFocus({
        opener: disabled,
        active: document.body,
        closed,
        top: null
      })
    ).toBe(false)
  })

  it("doesn't restore outside a dialog that is still on top", () => {
    const { opener, closed } = setup()
    const top = dialog(`<button>other dialog</button>`)
    expect(
      shouldRestoreFocus({ opener, active: document.body, closed, top })
    ).toBe(false)
  })
})

describe("createDialogFocus", () => {
  const withTrap = () => {
    const trap = createDialogFocus(document)
    trap.connect()
    return trap
  }
  let trap: ReturnType<typeof withTrap> | undefined
  afterEach(() => {
    trap?.disconnect()
    trap = undefined
  })

  const opener = () => {
    const button = mount(`<button id="opener">open</button>`)
    button.focus()
    return button
  }

  describe("on open", () => {
    it("moves focus to the dialog itself", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<p>text</p><button id="first">first</button>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)
    })

    it("moves focus in without scrolling", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<p>text</p><button id="first">first</button>`)
      const options: unknown[] = []
      const focus = d.focus.bind(d)
      d.focus = (opts?: FocusOptions) => {
        options.push(opts)
        focus(opts)
      }
      trap.open(d)
      await flush()
      expect(active()).toBe(d)
      expect(options).toEqual([{ preventScroll: true }])
    })

    it("leaves focus where the dialog's own code put it", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="first">first</button><button id="primary">primary</button>`
      )
      trap.open(d)
      byId("primary").focus() // the modal's onMount
      await flush()
      expect(active()).toBe(byId("primary"))
    })

    it("focuses an autofocus control first", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="first">first</button><input id="name" autofocus />`
      )
      trap.open(d)
      await flush()
      expect(active()).toBe(byId("name"))
    })

    it("holds focus on the dialog while lazy content loads, then moves to its autofocus control", async () => {
      trap = withTrap()
      opener()
      const d = dialog()
      trap.open(d)
      await flush()
      // Holds focus off the page meanwhile.
      expect(active()).toBe(d)

      d.innerHTML = `<p>loaded</p><button id="first">first</button><input id="name" autofocus />`
      await flush()
      expect(active()).toBe(byId("name"))
    })

    it("stays on the dialog when lazy content has no autofocus control", async () => {
      trap = withTrap()
      opener()
      const d = dialog()
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      d.innerHTML = `<p>loaded</p><button id="first">first</button>`
      await flush()
      expect(active()).toBe(d)
    })

    it("moves to an autofocus control once it can take focus", async () => {
      // Download Java on a fresh app: the version select is disabled while
      // the catalog loads.
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="version" autofocus disabled>version</button><button id="advanced">advanced</button>`
      )
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      byId("version").removeAttribute("disabled")
      await flush()
      expect(active()).toBe(byId("version"))
    })

    it("doesn't move to an autofocus control that got ready after the user moved focus", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="version" autofocus disabled>version</button><button id="advanced">advanced</button>`
      )
      trap.open(d)
      await flush()
      byId("advanced").focus()

      byId("version").removeAttribute("disabled")
      await flush()
      expect(active()).toBe(byId("advanced"))
    })

    it("doesn't move to an autofocus control that got ready after the user went into the dialog and back out to it", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="version" autofocus disabled>version</button><button id="advanced">advanced</button>`
      )
      trap.open(d)
      await flush()
      byId("advanced").focus()
      d.focus() // a click on the dialog's blank space

      byId("version").removeAttribute("disabled")
      await flush()
      expect(active()).toBe(d)
    })

    it("doesn't move to an autofocus control that got ready while focus is in a toast", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="version" autofocus disabled>version</button>`
      )
      trap.open(d)
      await flush()
      mount(
        `<ol tabindex="-1" data-somoto-toaster><li tabindex="0" data-somoto-toast><button id="undo">Undo</button></li></ol>`
      )
      byId("undo").focus()

      byId("version").removeAttribute("disabled")
      await flush()
      expect(active()).toBe(byId("undo"))
    })

    it("focuses the dialog itself when it has nothing to tab to", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<p>Just text</p>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)
    })

    it("opens on the dialog, not the close button", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`${header}<button id="first">first</button>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)
    })

    it("tries again when a closing menu's focus scope takes focus back", async () => {
      // Kobalte closes a menu in a timeout after onSelect; until then its
      // focus scope sends focus back to the item, the modal's own included.
      trap = withTrap()
      const menu = popper(
        `<div role="menuitem" tabindex="0" id="item">Edit share</div>`
      )
      const content = menu.firstElementChild!
      byId("item").focus()
      let menuOpen = true
      const menuScope = (event: FocusEvent) => {
        if (menuOpen && !menu.contains(event.target as Node))
          byId("item").focus()
      }
      document.addEventListener("focusin", menuScope)
      onTestFinished(() => document.removeEventListener("focusin", menuScope))

      const d = dialog(
        `<button id="first">first</button><button id="primary">primary</button>`
      )
      trap.open(d)
      setTimeout(() => {
        menuOpen = false
        content.removeAttribute("data-expanded")
        content.setAttribute("data-closed", "")
      })
      byId("primary").focus() // the modal's onMount, sent back too
      expect(active()).toBe(byId("item"))

      await wait(100)
      expect(active()).toBe(byId("primary"))
    })
  })

  describe("while open", () => {
    it("wraps Tab and Shift+Tab inside the dialog", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `<button id="a">a</button><button id="b">b</button><button id="c">c</button>`
      )
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      // From the dialog itself, Shift+Tab goes to the last control.
      expect(tab(true).defaultPrevented).toBe(true)
      expect(active()).toBe(byId("c"))

      expect(tab().defaultPrevented).toBe(true)
      expect(active()).toBe(byId("a"))

      // Between controls the browser moves focus itself.
      expect(tab().defaultPrevented).toBe(false)
    })

    it("tabs from the dialog itself to its first control, the close button", async () => {
      trap = withTrap()
      opener()
      const d = dialog(
        `${header}<button id="a">a</button><button id="b">b</button>`
      )
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      expect(tab().defaultPrevented).toBe(true)
      expect(active()).toBe(byId("x"))
    })

    it("pulls focus that lands outside back to where it was inside", async () => {
      trap = withTrap()
      const page = opener()
      const d = dialog(`<button id="a">a</button><button id="b">b</button>`)
      trap.open(d)
      await flush()
      byId("b").focus()

      page.focus()
      expect(active()).toBe(byId("b"))
    })

    it("pulls focus back to the dialog when nothing inside had it yet", () => {
      // Download Java: the menu refocuses its + trigger, outside the dialog,
      // before the dialog placed focus anywhere.
      trap = withTrap()
      const trigger = mount(`<button id="plus">+</button>`)
      opener()
      const d = dialog(`<p>text</p><button id="first">first</button>`)
      trap.open(d)

      trigger.focus()
      expect(active()).toBe(d)
    })

    it("pulls focus back to an autofocus control when nothing inside had it yet", () => {
      trap = withTrap()
      const trigger = mount(`<button id="plus">+</button>`)
      opener()
      const d = dialog(
        `<button id="first">first</button><button id="version" autofocus>version</button>`
      )
      trap.open(d)

      trigger.focus()
      expect(active()).toBe(byId("version"))
    })

    it("lets focus into Kobalte popper content opened from the dialog", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<button id="trigger">select</button>`)
      trap.open(d)
      await flush()
      byId("trigger").focus()
      expect(active()).toBe(byId("trigger"))

      popper(`<div role="listbox" tabindex="0" id="listbox">options</div>`)
      byId("listbox").focus()
      expect(active()).toBe(byId("listbox"))
    })

    it("lets focus into the toast region", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<button id="a">a</button>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      mount(
        `<ol tabindex="-1" data-somoto-toaster><li tabindex="0" data-somoto-toast><button id="undo">Undo</button></li></ol>`
      )
      byId("undo").focus()
      expect(active()).toBe(byId("undo"))
    })

    describe("after a toast button", () => {
      const toast = () =>
        mount(
          `<ol tabindex="-1" data-somoto-toaster><li tabindex="0" data-somoto-toast id="toast"><button id="undo">Undo</button></li></ol>`
        )

      it("returns focus to where it was in the dialog when the toast goes", async () => {
        trap = withTrap()
        opener()
        const d = dialog(`<button id="a">a</button><button id="b">b</button>`)
        trap.open(d)
        await flush()
        byId("b").focus()
        toast()
        byId("undo").focus()

        // The action closes the toast; somoto removes it.
        byId("toast").remove()
        expect(active()).toBe(document.body)
        await flush()
        expect(active()).toBe(byId("b"))
      })

      it("returns focus to the dialog when it leaves the toast for nowhere", async () => {
        trap = withTrap()
        opener()
        const d = dialog(`<button id="a">a</button><button id="b">b</button>`)
        trap.open(d)
        await flush()
        byId("b").focus()
        toast()
        byId("undo").focus()

        byId("undo").blur()
        expect(active()).toBe(document.body)
        await flush()
        expect(active()).toBe(byId("b"))
      })

      it("returns focus to the dialog itself when nothing inside had it", async () => {
        trap = withTrap()
        opener()
        const d = dialog(`<button id="a">a</button>`)
        trap.open(d)
        await flush()
        toast()
        byId("undo").focus()

        byId("undo").blur()
        await flush()
        expect(active()).toBe(d)
      })

      it("leaves focus on a toast button that only lost the window's focus", async () => {
        trap = withTrap()
        opener()
        const d = dialog(`<button id="a">a</button>`)
        trap.open(d)
        await flush()
        toast()
        byId("undo").focus()

        // Switching windows: focusout with nowhere to go, focus stays put.
        byId("undo").dispatchEvent(
          new FocusEvent("focusout", { bubbles: true, relatedTarget: null })
        )
        await flush()
        expect(active()).toBe(byId("undo"))
      })
    })

    it("leaves Tab alone while focus is in a popper", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<button id="a">a</button>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)
      popper(`<button id="in-popover">in popover</button>`)
      byId("in-popover").focus()

      expect(tab().defaultPrevented).toBe(false)
      expect(active()).toBe(byId("in-popover"))
    })

    it("keeps focus the dialog moves into its portalled content", async () => {
      // ConfirmChangeRuntimePath's error overlay focuses its Dismiss.
      trap = withTrap()
      opener()
      const d = dialog(`<button id="a">a</button>`)
      trap.open(d)
      await flush()
      expect(active()).toBe(d)

      mount(
        `<div ${DIALOG_PORTAL_ATTR}><button id="dismiss">Dismiss</button></div>`
      )
      byId("dismiss").focus()
      await flush()
      expect(active()).toBe(byId("dismiss"))
    })

    it("wraps Tab inside the portalled content while focus is there", async () => {
      trap = withTrap()
      opener()
      const d = dialog(`<button id="a">a</button>`)
      trap.open(d)
      await flush()
      mount(
        `<div ${DIALOG_PORTAL_ATTR}><button id="p1">p1</button><button id="p2">p2</button></div>`
      )
      byId("p2").focus()

      expect(tab().defaultPrevented).toBe(true)
      expect(active()).toBe(byId("p1"))
      expect(tab(true).defaultPrevented).toBe(true)
      expect(active()).toBe(byId("p2"))
    })
  })

  describe("on close", () => {
    it("returns focus to the opener when focus fell to body", async () => {
      trap = withTrap()
      const page = opener()
      const d = dialog(`<button id="a">a</button>`)
      const handle = trap.open(d)
      await flush()

      handle.close()
      d.remove()
      expect(active()).toBe(document.body)
      await flush()
      expect(active()).toBe(page)
    })

    it("returns focus to the opener when focus was still inside", async () => {
      trap = withTrap()
      const page = opener()
      const d = dialog(`<button id="a">a</button>`)
      const handle = trap.open(d)
      await flush()
      byId("a").focus()

      handle.close()
      await flush()
      expect(active()).toBe(page)
    })

    it("doesn't override an opener that moved focus itself", async () => {
      trap = withTrap()
      opener()
      const add = mount(`<button id="add">add</button>`)
      const d = dialog(`<button id="a">a</button>`)
      const handle = trap.open(d)
      await flush()
      byId("a").focus()

      // The manager closes the entry first; then the modal's own cleanup
      // runs, with the Java dialogs' onClosed in it.
      handle.close()
      add.focus()
      d.remove()
      await flush()
      expect(active()).toBe(add)
    })

    it("returns focus to the menu's trigger when opened from a menu item", async () => {
      trap = withTrap()
      const trigger = mount(`<button id="plus">+</button>`)
      const menu = popper(
        `<div role="menuitem" tabindex="0" id="item">Download Java</div>`
      )
      byId("item").focus()
      const d = dialog(`<button id="first">first</button>`)
      const handle = trap.open(d)
      await flush()

      // The menu leaves, then refocuses its trigger.
      menu.remove()
      trigger.focus()
      expect(active()).toBe(d)

      handle.close()
      d.remove()
      await flush()
      expect(active()).toBe(trigger)
    })
  })

  describe("stacked dialogs", () => {
    it("keeps focus in the top dialog, out of the one underneath", async () => {
      trap = withTrap()
      opener()
      const lower = dialog(`<button id="lower">lower</button>`)
      trap.open(lower)
      await flush()
      const upper = dialog(`<button id="upper">upper</button>`)
      trap.open(upper)
      await flush()
      expect(active()).toBe(upper)

      byId("lower").focus()
      expect(active()).toBe(upper)
    })

    it("hands focus back to the dialog underneath, then to the page", async () => {
      trap = withTrap()
      const page = opener()
      const lower = dialog(
        `<button id="l1">l1</button><button id="l2">opens upper</button>`
      )
      const lowerHandle = trap.open(lower)
      await flush()
      byId("l2").focus()

      const upper = dialog(`<button id="upper">upper</button>`)
      const upperHandle = trap.open(upper)
      await flush()
      expect(active()).toBe(upper)

      upperHandle.close()
      upper.remove()
      await flush()
      expect(active()).toBe(byId("l2"))

      lowerHandle.close()
      lower.remove()
      await flush()
      expect(active()).toBe(page)
    })

    it("hands a closing lower dialog's opener on to the dialog it opened", async () => {
      trap = withTrap()
      const page = opener()
      const lower = dialog(`<button id="l1">opens upper</button>`)
      const lowerHandle = trap.open(lower)
      await flush()

      const upper = dialog(`<button id="upper">upper</button>`)
      const upperHandle = trap.open(upper)
      await flush()

      // The lower one closes itself while the upper one is still open.
      lowerHandle.close()
      lower.remove()
      await flush()
      expect(active()).toBe(upper)

      upperHandle.close()
      upper.remove()
      await flush()
      expect(active()).toBe(page)
    })
  })
})
