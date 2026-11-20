// Whether Escape closes the top modal. Free of solid-js and app imports, so
// jsdom tests can drive it; the ModalsManager's keydown listener asks it.

import { POPPER_SELECTOR } from "./dialogFocus"

/** Popup content that takes an Escape itself: a listbox or a menu, and any
 *  open Kobalte popper (a popover) but a tooltip, which never held Escape
 *  back. */
const OPEN_POPUP_SELECTOR = `[role="listbox"], [role="menu"], ${POPPER_SELECTOR} > :not([data-closed]):not([role="tooltip"])`

/** Whether `event`, an Escape keydown read on the document after the focused
 *  control's own handlers, closes the top modal. Not while a text field is
 *  composing, not while a popup is open (it closes itself, so one press
 *  doesn't close both), and not when a control cancelled the Escape.
 *
 *  Kobalte's Tabs list is the exception: its selectable collection cancels
 *  every Escape, to clear the selection, which tabs never allow
 *  (`disallowEmptySelection`). So with focus on a tab, as on New Instance,
 *  its cancel doesn't count. */
export const escapeClosesModal = (
  event: KeyboardEvent,
  doc: Document
): boolean => {
  if (event.key !== "Escape" || event.isComposing) return false
  if (doc.querySelector(OPEN_POPUP_SELECTOR)) return false
  if (!event.defaultPrevented) return true
  return (
    event.target instanceof Element &&
    event.target.closest('[role="tablist"]') !== null
  )
}
