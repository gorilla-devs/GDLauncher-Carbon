// Kept free of app imports (no solid-js, no router, no i18n) so it can be
// unit tested without pulling in the rest of the module graph — the
// ModalsManager `index.tsx` and `ModalLayout.tsx` both import from here.

/** Whether a modal stack entry currently blocks Escape/backdrop close: true
 *  if either the static registry's `preventClose` (the caller looks this up
 *  by modal name and passes it in) or the entry's live ModalLayout-
 *  registered accessor says so. Neither source shadows the other — a modal
 *  can rely on just one, or on both simultaneously. */
export function resolvePreventClose(
  registryPreventClose: boolean | (() => boolean) | undefined,
  entry: { preventCloseAccessor?: () => boolean }
): boolean {
  const registryPrevents =
    typeof registryPreventClose === "function"
      ? registryPreventClose()
      : registryPreventClose === true
  return registryPrevents || entry.preventCloseAccessor?.() === true
}
