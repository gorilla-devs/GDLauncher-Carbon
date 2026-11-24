// Keeps keyboard focus with the open dialogs: it moves in when one opens,
// stays inside the top one while it's open, and goes back when it closes.
// Free of solid-js and app imports, so jsdom tests can drive it; the
// ModalsManager wires one instance to its stack.
//
// Kobalte's `createFocusScope` doesn't fit: it pulls focus back to nothing
// when nothing inside had focus yet (the Download Java case), it knows
// nothing of somoto's toasts, and it can't wait for an `autofocus` control
// that isn't ready yet.
//
// A dialog opens with focus on the dialog element itself, so nothing shows
// focus, no tooltip opens and Enter does nothing until the user moves on;
// Tab then goes to the first control and Shift+Tab to the last. A dialog
// that should start on a control puts `autofocus` on it. If that control
// can't take focus yet (disabled while its data loads, or lazy content not
// there yet), focus moves to it once it can, unless the user moved focus
// meanwhile.
//
// A dialog can also focus a control in `onMount`, but one opened from a
// menu's `onSelect` may lose that focus to the menu's focus scope, which
// lasts until the menu closes in a timeout. The trap then tries again, on
// the control that had focus last, so `autofocus` is the reliable way.
//
// Focus goes back on close to the control that opened the dialog. Opened
// from a menu item, that item goes away, and a DropdownMenu then refocuses
// its trigger, which becomes the one to go back to. A Kobalte ContextMenu
// refocuses nothing, so a dialog opened from one returns focus to body.

/** Marks content a dialog renders outside its own element through a
 *  Portal. It still belongs to the dialog: focus may go there, and Tab stays
 *  inside it while focus is there. Nothing moves focus there, so the dialog
 *  does that itself when the content shows. */
export const DIALOG_PORTAL_ATTR = "data-dialog-portal"

/** How often, and how far apart, the trap tries again to move focus into a
 *  dialog when something takes it straight back. */
const MAX_RETRIES = 10
const RETRY_DELAY_MS = 20

/** The attributes that can make an `autofocus` control ready (or not) to
 *  take focus. */
const FOCUSABILITY_ATTRIBUTES = [
  "autofocus",
  "disabled",
  "hidden",
  "inert",
  "aria-hidden",
  "tabindex"
]

/** Kobalte's wrapper around portalled popper content: Select listboxes,
 *  menus, popovers, tooltips. */
export const POPPER_SELECTOR = "[data-popper-positioner]"

/** Content that stays usable over an open dialog: somoto's toast lists,
 *  Kobalte's top layer, and a dialog's own portalled content. */
const ALWAYS_REACHABLE_SELECTOR = `[data-somoto-toaster], [data-kb-top-layer], [${DIALOG_PORTAL_ATTR}]`

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button",
  "input:not([type='hidden'])",
  "select",
  "textarea",
  "iframe",
  "[contenteditable]:not([contenteditable='false'])",
  "[tabindex]"
].join(",")

/** Whether `el` can take focus, by its selectors and attributes: jsdom has no
 *  layout. `aria-hidden` counts only up to `boundary` (the dialog), since a
 *  Kobalte modal popup aria-hides everything around itself, dialogs
 *  included. Where the browser can say whether CSS hides it, it asks. */
export const isFocusable = (
  el: Element | null | undefined,
  boundary?: Element
): el is HTMLElement => {
  if (!(el instanceof HTMLElement) || !el.isConnected) return false
  if (!el.matches(FOCUSABLE_SELECTOR) || el.matches(":disabled")) return false
  let withinBoundary = true
  for (let node: Element | null = el; node; node = node.parentElement) {
    if (node.hasAttribute("hidden") || node.hasAttribute("inert")) return false
    if (withinBoundary && node.getAttribute("aria-hidden") === "true")
      return false
    if (node === boundary) withinBoundary = false
  }
  return (
    typeof el.checkVisibility !== "function" ||
    el.checkVisibility({ visibilityProperty: true })
  )
}

const inTabOrder = (el: HTMLElement) => {
  const tabindex = el.getAttribute("tabindex")
  return tabindex === null || Number.parseInt(tabindex, 10) >= 0
}

const isGroupedRadio = (el: Element): el is HTMLInputElement =>
  el instanceof HTMLInputElement && el.type === "radio" && el.name !== ""

/** The controls inside `container` that Tab reaches, in DOM order. A named
 *  radio group is one stop, as Tab stops there once: its checked radio, else
 *  its first. */
export const getTabbables = (container: HTMLElement): HTMLElement[] => {
  const tabbables = Array.from(
    container.querySelectorAll(FOCUSABLE_SELECTOR)
  ).filter(
    (el): el is HTMLElement => isFocusable(el, container) && inTabOrder(el)
  )
  return tabbables.filter((el) => {
    if (!isGroupedRadio(el)) return true
    const group = tabbables.filter(
      (other): other is HTMLInputElement =>
        isGroupedRadio(other) &&
        other.name === el.name &&
        other.form === el.form
    )
    return el === (group.find((radio) => radio.checked) ?? group[0])
  })
}

/** Where focus starts in a dialog: an `autofocus` control that can take
 *  focus, else the dialog itself. */
export const initialFocusTarget = (dialog: HTMLElement): HTMLElement =>
  Array.from(dialog.querySelectorAll("[autofocus]")).find(
    (el): el is HTMLElement => isFocusable(el, dialog)
  ) ?? dialog

/** Where Tab (or Shift+Tab, `backwards`) from `active` must send focus to
 *  stay inside `dialog`, or null when the browser's own move stays inside. */
export const tabWrapTarget = (
  dialog: HTMLElement,
  active: Element | null,
  backwards: boolean
): HTMLElement | null => {
  const tabbables = getTabbables(dialog)
  if (tabbables.length === 0) return dialog
  const first = tabbables[0]
  const last = tabbables[tabbables.length - 1]
  if (!active || active === dialog || !dialog.contains(active))
    return backwards ? last : first

  const index = tabbables.indexOf(active as HTMLElement)
  if (index === -1) {
    // Focus is on something Tab skips (tabindex -1): go by DOM position.
    const follows = (el: Element) =>
      (active.compareDocumentPosition(el) &
        Node.DOCUMENT_POSITION_FOLLOWING) !==
      0
    return backwards
      ? (tabbables.findLast((el) => !follows(el)) ?? last)
      : (tabbables.find(follows) ?? first)
  }
  if (backwards) return index === 0 ? last : null
  return index === tabbables.length - 1 ? first : null
}

/** Whether focus may sit on `target`, outside the top dialog: in open Kobalte
 *  popper content (over a dialog, only one opened from it can be open; a
 *  page menu that opened the dialog is on its way out, `data-closed`), in
 *  the toast region, or in content the dialog portals out. */
export const isOutsideFocusAllowed = (target: Element): boolean => {
  const popper = target.closest(POPPER_SELECTOR)
  if (popper) return popper.querySelector(":scope > [data-closed]") === null
  return (
    target.closest(ALWAYS_REACHABLE_SELECTOR) !== null ||
    // The section somoto wraps its toast lists in.
    target.querySelector(":scope > [data-somoto-toaster]") !== null
  )
}

/** Whether closing `closed` should put focus back on `opener`: only when
 *  focus was left on body or inside the closed dialog (so not when the
 *  opener moved it itself), the opener can still take it, and it isn't
 *  outside a dialog that is still on top. */
export const shouldRestoreFocus = ({
  opener,
  active,
  closed,
  top
}: {
  opener: HTMLElement | null
  active: Element | null
  closed: HTMLElement
  top: HTMLElement | null
}): boolean => {
  if (!isFocusable(opener)) return false
  if (top && !top.contains(opener)) return false
  return (
    !active || active === active.ownerDocument.body || closed.contains(active)
  )
}

interface Entry {
  dialog: HTMLElement
  /** Where focus goes back to on close. */
  opener: HTMLElement | null
  /** The opener was in popper content (a menu item): it goes away, and the
   *  popper then refocuses its trigger, which is where to go back to. */
  openerInPopper: boolean
  lastFocused: HTMLElement | null
  /** A move in has landed: focus has been on the dialog or inside it. */
  placed: boolean
  /** Tries again so far after a move in was taken straight back. */
  retries: number
  /** A control inside has focus, by our doing or the dialog's own. */
  settled: boolean
  /** Watches for an `autofocus` control getting ready, in lazy content or
   *  by losing `disabled`, until settled. */
  observer: MutationObserver
}

export interface DialogFocusHandle {
  /** Call when the dialog closes, before or after its element goes. */
  close: () => void
}

export const createDialogFocus = (doc: Document) => {
  const stack: Entry[] = []
  const top = (): Entry | undefined => stack[stack.length - 1]
  const focus = (el: HTMLElement) => el.focus({ preventScroll: true })
  const isBody = (el: Element | null) => !el || el === doc.body

  /** Where focus belongs in `entry`: where it last was inside, else its
   *  `autofocus` control, else the dialog itself. */
  const homeOf = (entry: Entry) =>
    entry.lastFocused &&
    entry.dialog.contains(entry.lastFocused) &&
    isFocusable(entry.lastFocused, entry.dialog)
      ? entry.lastFocused
      : initialFocusTarget(entry.dialog)

  const settle = (entry: Entry) => {
    entry.settled = true
    entry.observer.disconnect()
  }

  /** Moves focus into `entry` if it's on top and no control inside has it
   *  yet. Until a move has landed it takes focus from wherever it is. After
   *  that it moves focus from body, and from the dialog itself to an
   *  `autofocus` control that got ready, but only while focus is still
   *  where the trap left it: not once the user went to a toast, or into the
   *  dialog and back out to the dialog itself. A move taken straight back
   *  (a closing menu's focus scope) is tried again a few times. */
  const moveIn = (entry: Entry) => {
    if (entry.settled || entry !== top() || !entry.dialog.isConnected) return
    const active = doc.activeElement
    if (active !== entry.dialog && entry.dialog.contains(active)) {
      settle(entry)
      return
    }
    if (
      entry.placed &&
      !isBody(active) &&
      (active !== entry.dialog || entry.lastFocused)
    )
      return
    const target = homeOf(entry)
    if (target === active) {
      entry.placed = true
      return
    }
    focus(target)
    if (doc.activeElement !== target) {
      if (entry.retries < MAX_RETRIES) {
        entry.retries++
        setTimeout(() => moveIn(entry), RETRY_DELAY_MS)
      }
      return
    }
    entry.placed = true
    if (target !== entry.dialog) settle(entry)
  }

  /** Focus that went to a toast or a popper over the top dialog falls to
   *  body when that control goes (a toast's action closes the toast) or
   *  focus leaves it for nowhere: it then comes back to the dialog. A
   *  removed element may not get a focusout, so while focus is out there,
   *  removals are watched too. */
  const pullBackFromBody = () => {
    const entry = top()
    if (!entry || !isBody(doc.activeElement)) return
    watchOutside(false)
    focus(homeOf(entry))
  }
  const outsideWatch = new MutationObserver(pullBackFromBody)
  let watchingOutside = false
  const watchOutside = (on: boolean) => {
    if (on === watchingOutside || !doc.body) return
    watchingOutside = on
    if (on) outsideWatch.observe(doc.body, { childList: true, subtree: true })
    else outsideWatch.disconnect()
  }

  const onFocusIn = (event: FocusEvent) => {
    const entry = top()
    const target = event.target
    if (!entry || !(target instanceof HTMLElement)) return
    if (entry.dialog.contains(target)) {
      watchOutside(false)
      if (target !== entry.dialog) entry.lastFocused = target
      return
    }
    if (isOutsideFocusAllowed(target)) {
      watchOutside(true)
      return
    }
    if (entry.openerInPopper && !entry.opener?.isConnected) {
      entry.opener = target
      entry.openerInPopper = false
    }
    focus(homeOf(entry))
  }

  /** Focus leaving something outside the top dialog for nowhere. Focus
   *  that only left the window stays where it was, so nothing moves. Focus
   *  falling to body from inside the dialog is the dialog's own business. */
  const onFocusOut = (event: FocusEvent) => {
    const entry = top()
    if (!entry || event.relatedTarget !== null) return
    if (event.target instanceof Node && entry.dialog.contains(event.target))
      return
    setTimeout(pullBackFromBody, 0)
  }

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Tab" || event.defaultPrevented) return
    if (event.altKey || event.ctrlKey || event.metaKey) return
    const entry = top()
    if (!entry) return
    const active = doc.activeElement
    // Content the dialog portals out keeps Tab to itself while focus is in it.
    const portal = active?.closest(`[${DIALOG_PORTAL_ATTR}]`)
    const scope = portal instanceof HTMLElement ? portal : entry.dialog
    // A popper or the toasts: their own Tab handling applies.
    if (!isBody(active) && !scope.contains(active)) return
    const target = tabWrapTarget(scope, active, event.shiftKey)
    if (!target) return
    event.preventDefault()
    target.focus()
  }

  const close = (entry: Entry) => {
    const index = stack.indexOf(entry)
    if (index === -1) return
    stack.splice(index, 1)
    entry.observer.disconnect()
    if (stack.length === 0) watchOutside(false)
    // A dialog this one opened goes back to where this one would have.
    for (const other of stack) {
      if (other.opener && entry.dialog.contains(other.opener)) {
        other.opener = entry.opener
        other.openerInPopper = entry.openerInPopper
      }
    }
    // After the closing modal's cleanup and its element's removal, and after
    // a backdrop mousedown's default focus change.
    setTimeout(() => {
      const newTop = top()
      if (
        shouldRestoreFocus({
          opener: entry.opener,
          active: doc.activeElement,
          closed: entry.dialog,
          top: newTop?.dialog ?? null
        })
      )
        focus(entry.opener!)
      if (!newTop) return
      const active = doc.activeElement
      if (active !== newTop.dialog && newTop.dialog.contains(active)) return
      if (active && !isBody(active) && isOutsideFocusAllowed(active)) return
      focus(homeOf(newTop))
    }, 0)
  }

  return {
    connect: () => {
      doc.addEventListener("focusin", onFocusIn)
      doc.addEventListener("focusout", onFocusOut)
      doc.addEventListener("keydown", onKeyDown)
    },
    disconnect: () => {
      doc.removeEventListener("focusin", onFocusIn)
      doc.removeEventListener("focusout", onFocusOut)
      doc.removeEventListener("keydown", onKeyDown)
      watchOutside(false)
      for (const entry of stack) entry.observer.disconnect()
      stack.length = 0
    },
    /** Call when a dialog opens, with its element (it may still be empty and
     *  not in the document yet). */
    open: (dialog: HTMLElement): DialogFocusHandle => {
      const active = doc.activeElement
      const opener =
        active instanceof HTMLElement && !isBody(active) ? active : null
      const entry: Entry = {
        dialog,
        opener,
        openerInPopper: opener?.closest(POPPER_SELECTOR) != null,
        lastFocused: null,
        placed: false,
        retries: 0,
        settled: false,
        observer: new MutationObserver(() => moveIn(entry))
      }
      entry.observer.observe(dialog, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: FOCUSABILITY_ATTRIBUTES
      })
      stack.push(entry)
      // After the modal's own onMount focus.
      setTimeout(() => moveIn(entry), 0)
      return { close: () => close(entry) }
    }
  }
}

/** Whether focus was lost (nowhere, the body, or one of the dialog's own
 *  buttons that just went disabled), as opposed to moved on purpose to
 *  something else such as an error overlay's Dismiss button. */
export const focusWasDropped = (
  active: Element | null,
  body: HTMLElement,
  own: (Element | undefined)[]
): boolean => !active || active === body || own.includes(active)
