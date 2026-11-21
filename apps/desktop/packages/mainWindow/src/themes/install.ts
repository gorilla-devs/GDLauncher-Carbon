import type { FEThemePreview } from "@gd/core_module/bindings"
import type { ThemeErrorKey } from "./errors"

export const THEME_EXTENSION = ".gdltheme"

/** A file name that ends with ".gdltheme" (any case) and has a name before
 *  it. Only the name is looked at; the core checks the file itself. */
export function isThemeFileName(name: string): boolean {
  return (
    name.length > THEME_EXTENSION.length &&
    name.toLowerCase().endsWith(THEME_EXTENSION)
  )
}

/** The index of the first theme file among dropped files, or -1. */
export function firstThemeFile(names: readonly string[]): number {
  return names.findIndex(isThemeFileName)
}

/** The dropped file to install, or -1: nothing opens over a dialog that
 *  can't be closed right now (a running install, for one). */
export function dropTarget(names: readonly string[], locked: boolean): number {
  return locked ? -1 : firstThemeFile(names)
}

/** Where the install dialog is. */
export type InstallStep =
  | { kind: "inspecting" }
  | { kind: "ready"; preview: FEThemePreview }
  | { kind: "installing"; preview: FEThemePreview }
  | { kind: "installed"; preview: FEThemePreview }
  | {
      kind: "failed"
      /** Set when the file was inspected and the install itself failed. */
      preview: FEThemePreview | null
      reason: ThemeErrorKey
      /** The core's English detail. */
      detail: string | null
    }

/** The dialog's one primary button, if it has one. While installing it is
 *  the same button, loading. */
export function primaryAction(
  step: InstallStep
): "install" | "replace" | "apply" | null {
  switch (step.kind) {
    case "ready":
    case "installing":
      return step.preview.installed ? "replace" : "install"
    case "installed":
      return "apply"
    default:
      return null
  }
}

/** The inspected preview, only when it is an inline image (the core makes it a
 *  `data:` URL); anything else shows no picture. */
export function previewImage(preview: FEThemePreview | null): string | null {
  const src = preview?.preview
  return src?.startsWith("data:image/") ? src : null
}

/** The core can't stop an install halfway, so the dialog stays until it
 *  answers: no close button, Escape, backdrop or header X. */
export function blocksClose(step: InstallStep): boolean {
  return step.kind === "installing"
}

/** The quiet close button reads Cancel until there's nothing left to do. */
export function closeLabel(
  step: InstallStep
): "general:_trn_cancel" | "settings:_trn_theme_install_close" {
  return step.kind === "installed" || step.kind === "failed"
    ? "settings:_trn_theme_install_close"
    : "general:_trn_cancel"
}

/** Replacing the saved theme reloads it in place, so the dialog just closes;
 *  any other theme goes on to the Apply step. */
export function closesAfterInstall(
  id: string,
  savedTheme: string | undefined
): boolean {
  return savedTheme === `user:${id}`
}
