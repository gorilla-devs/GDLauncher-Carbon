import {
  createContext,
  useContext,
  createSignal,
  createEffect,
  JSX
} from "solid-js"
import { mainTheme, pixelato, win95, type Theme } from "../../../src"

type ThemeName = "main" | "pixelato" | "win95"

interface ThemeContextType {
  currentTheme: () => ThemeName
  setTheme: (theme: ThemeName) => void
  themeObject: () => Theme
}

const ThemeContext = createContext<ThemeContextType>()

const themes: Record<ThemeName, Theme> = {
  main: mainTheme,
  pixelato: pixelato,
  win95: win95
}

export function ThemeProvider(props: { children: JSX.Element }) {
  const [currentTheme, setCurrentTheme] = createSignal<ThemeName>("main")

  // Load theme from localStorage on init
  const savedTheme = localStorage.getItem("ui-theme") as ThemeName
  if (savedTheme && themes[savedTheme]) {
    setCurrentTheme(savedTheme)
  }

  // Apply theme to CSS custom properties and styles
  createEffect(() => {
    const theme = themes[currentTheme()]
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
    setCurrentTheme(theme)
    localStorage.setItem("ui-theme", theme)
  }

  const themeObject = () => themes[currentTheme()]

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
