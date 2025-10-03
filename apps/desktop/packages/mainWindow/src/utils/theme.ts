import { mainTheme, Theme as UITheme, pixelato, win95, inferno, aether, frost } from "@gd/ui"
import { createEffect } from "solid-js"
import { rspc } from "./rspcClient"

enum _Theme {
  _Main = "main",
  _Pixelato = "pixelato",
  _Win95 = "win95",
  _Inferno = "inferno",
  _Aether = "aether",
  _Frost = "frost"
}

const initThemes = () => {
  const theme = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const themeName = () => theme.data?.theme
  createEffect(() => {
    applyThemeByName(themeName())
  })
}

export function applyThemeByName(themeName: string | undefined) {
  if (!themeName) {
    applyTheme(mainTheme)
    return
  }

  switch (themeName) {
    case _Theme._Pixelato: {
      applyTheme(pixelato)
      break
    }
    case _Theme._Win95: {
      applyTheme(win95)
      break
    }
    case _Theme._Inferno: {
      applyTheme(inferno)
      break
    }
    case _Theme._Aether: {
      applyTheme(aether)
      break
    }
    case _Theme._Frost: {
      applyTheme(frost)
      break
    }
    default: {
      applyTheme(mainTheme)
      break
    }
  }
}

export function applyTheme(theme: UITheme) {
  // Inject theme
  for (const key in theme) {
    if (key === "additional-styles") {
      const existingStyle = document.getElementById(key)
      if (existingStyle) {
        existingStyle.remove()
      }

      const style = document.createElement("style")
      style.setAttribute("id", key)
      style.innerHTML = theme[key as keyof UITheme]
      document.head.appendChild(style)
      continue
    }

    document.documentElement.style.setProperty(
      `--${key}`,
      theme[key as keyof UITheme]
    )
  }
}

export function getAvailableThemes(): string[] {
  return Object.values(_Theme)
}

export function getThemeColor(themeName: string, color: keyof UITheme): string {
  switch (themeName) {
    case _Theme._Pixelato: {
      return `rgb(${pixelato[color]})`
    }
    case _Theme._Win95: {
      return `rgb(${win95[color]})`
    }
    case _Theme._Inferno: {
      return `rgb(${inferno[color]})`
    }
    case _Theme._Aether: {
      return `rgb(${aether[color]})`
    }
    case _Theme._Frost: {
      return `rgb(${frost[color]})`
    }
    default: {
      return `rgb(${mainTheme[color]})`
    }
  }
}

export default initThemes
