import { Menu, app } from "electron"
import type { Input, MenuItemConstructorOptions } from "electron"

export interface AppMenuLabels {
  view: string
  resetTheme: string
}

export const DEFAULT_MENU_LABELS: AppMenuLabels = {
  view: "View",
  resetTheme: "Reset theme"
}

export const RESET_THEME_ACCELERATOR = "CmdOrCtrl+Alt+Shift+T"

export function buildAppMenuTemplate(
  platform: NodeJS.Platform,
  labels: AppMenuLabels,
  onResetTheme: () => void
): MenuItemConstructorOptions[] {
  const template = [
    ...(platform === "darwin"
      ? [
          {
            label: "GDLauncher",
            submenu: [
              {
                label: "About GDLauncher",
                role: "about"
              },
              { type: "separator" },
              {
                label: "Services",
                role: "services",
                submenu: []
              },
              { type: "separator" },
              {
                label: "Hide GDLauncher",
                accelerator: "Command+H",
                role: "hide"
              },
              {
                label: "Hide Others",
                accelerator: "Command+Alt+H",
                role: "hideOthers"
              },
              {
                label: "Show All",
                role: "unhide"
              },
              { type: "separator" },
              {
                label: "Quit GDLauncher",
                accelerator: "Command+Q",
                click: () => {
                  app.quit()
                }
              }
            ]
          }
        ]
      : []),
    {
      label: "Edit",
      submenu: [
        {
          label: "Cut",
          accelerator: "CmdOrCtrl+X",
          selector: "cut:"
        },
        {
          label: "Copy",
          accelerator: "CmdOrCtrl+C",
          selector: "copy:"
        },
        {
          label: "Paste",
          accelerator: "CmdOrCtrl+V",
          selector: "paste:"
        },
        {
          label: "Select All",
          accelerator: "CmdOrCtrl+A",
          selector: "selectAll:"
        },
        { type: "separator" },
        { label: "Undo", accelerator: "CmdOrCtrl+Z", selector: "undo:" },
        { label: "Redo", accelerator: "Shift+CmdOrCtrl+Z", selector: "redo:" }
      ]
    },
    {
      label: labels.view,
      submenu: [
        {
          id: "reset-theme",
          label: labels.resetTheme,
          accelerator: RESET_THEME_ACCELERATOR,
          click: onResetTheme
        }
      ]
    }
  ]
  return template as any
}

export function installAppMenu(
  labels: AppMenuLabels,
  onResetTheme: () => void
): void {
  Menu.setApplicationMenu(
    Menu.buildFromTemplate(
      buildAppMenuTemplate(process.platform, labels, onResetTheme)
    )
  )
}

export function isResetThemeShortcut(
  input: Pick<Input, "type" | "control" | "meta" | "alt" | "shift" | "code">,
  platform: NodeJS.Platform
): boolean {
  return (
    input.type === "keyDown" &&
    input.code === "KeyT" &&
    input.alt &&
    input.shift &&
    (platform === "darwin" ? input.meta : input.control)
  )
}

export function sanitizeMenuLabels(value: unknown): AppMenuLabels | null {
  if (typeof value !== "object" || value === null) return null
  const { view, resetTheme } = value as Record<string, unknown>
  if (typeof view !== "string" || typeof resetTheme !== "string") return null
  const v = view.trim()
  const r = resetTheme.trim()
  if (v.length < 1 || v.length > 64 || r.length < 1 || r.length > 64) {
    return null
  }
  return { view: v, resetTheme: r }
}
