import {
  createContext,
  useContext,
  createSignal,
  createEffect,
  JSX
} from "solid-js"
import {
  mainTheme,
  pixelato,
  win95,
  inferno,
  aether,
  frost,
  type Theme
} from "../../../src"

export const showcaseThemes = {
  main: {
    theme: mainTheme,
    label: "Main Theme",
    description: "Default dark theme"
  },
  pixelato: {
    theme: pixelato,
    label: "Pixelato",
    description: "Pixel art inspired theme"
  },
  win95: {
    theme: win95,
    label: "Windows 95",
    description: "Retro Windows 95 style"
  },
  inferno: {
    theme: inferno,
    label: "Inferno",
    description: "Fiery crimson theme"
  },
  aether: {
    theme: aether,
    label: "Aether",
    description: "Ethereal void theme"
  },
  frost: { theme: frost, label: "Frost", description: "Icy arctic theme" }
} satisfies Record<string, { theme: Theme; label: string; description: string }>

export type ThemeName = keyof typeof showcaseThemes

const isThemeName = (value: unknown): value is ThemeName =>
  typeof value === "string" && Object.hasOwn(showcaseThemes, value)

interface ThemeContextType {
  currentTheme: () => ThemeName
  setTheme: (theme: ThemeName) => void
  themeObject: () => Theme
}

const ThemeContext = createContext<ThemeContextType>()

export function ThemeProvider(props: { children: JSX.Element }) {
  const [currentTheme, setCurrentTheme] = createSignal<ThemeName>("main")

  // Load theme from localStorage on init
  const savedTheme = localStorage.getItem("ui-theme")
  if (isThemeName(savedTheme)) {
    setCurrentTheme(savedTheme)
  }

  // Apply theme to CSS custom properties and styles
  createEffect(() => {
    const theme = showcaseThemes[currentTheme()].theme
    const root = document.documentElement

    Object.entries(theme).forEach(([key, value]) => {
      if (key === "additional-styles") {
        // Remove existing additional styles
        if (document.getElementById(key)) {
          document.getElementById(key)?.remove()
        }

        // Add new additional styles
        const style = document.createElement("style")
        style.setAttribute("id", key)
        style.innerHTML = value as string
        document.head.appendChild(style)
      } else if (typeof value === "string") {
        root.style.setProperty(`--${key}`, value)
      }
    })
  })

  const setTheme = (theme: ThemeName) => {
    if (!isThemeName(theme)) return
    setCurrentTheme(theme)
    localStorage.setItem("ui-theme", theme)
  }

  const themeObject = () => showcaseThemes[currentTheme()].theme

  return (
    <ThemeContext.Provider value={{ currentTheme, setTheme, themeObject }}>
      {props.children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const context = useContext(ThemeContext)
  if (!context) {
    throw new Error("useTheme must be used within a ThemeProvider")
  }
  return context
}
