import {
  createContext,
  useContext,
  createSignal,
  createEffect,
  JSX
} from "solid-js"

/** The themes the showcase offers, by the id of their package. */
export const showcaseThemes = {
  main: { label: "Main Theme", description: "Default dark theme" },
  pixelato: { label: "Pixelato", description: "Pixel art inspired theme" },
  win95: { label: "Windows 95", description: "Retro Windows 95 style" },
  inferno: { label: "Inferno", description: "Fiery crimson theme" },
  aether: { label: "Aether", description: "Ethereal void theme" },
  frost: { label: "Frost", description: "Icy arctic theme" }
} satisfies Record<string, { label: string; description: string }>

export type ThemeName = keyof typeof showcaseThemes

const isThemeName = (value: unknown): value is ThemeName =>
  typeof value === "string" && Object.hasOwn(showcaseThemes, value)

interface Manifest {
  id: string
  css?: string
  colors?: Record<string, string>
  tokens?: { font?: string; "font-mono"?: string }
}

interface ThemeContextType {
  currentTheme: () => ThemeName
  setTheme: (theme: ThemeName) => void
  themeObject: () => Manifest
}

const ThemeContext = createContext<ThemeContextType>()

// The built-in theme packages are the single source of truth
const manifests = import.meta.glob<Manifest>(
  "../../../themes/*/manifest.json",
  { eager: true, import: "default" }
)
const styles = import.meta.glob<string>("../../../themes/*/theme.css", {
  eager: true,
  import: "default",
  query: "?raw"
})

const themes = {} as Record<ThemeName, Manifest>
const themeCss = {} as Record<ThemeName, string>
for (const [path, manifest] of Object.entries(manifests)) {
  const id = path.split("/themes/")[1].split("/")[0] as ThemeName
  themes[id] = manifest
  themeCss[id] = styles[path.replace("manifest.json", "theme.css")] ?? ""
}

export function ThemeProvider(props: { children: JSX.Element }) {
  const [currentTheme, setCurrentTheme] = createSignal<ThemeName>("main")

  // Load theme from localStorage on init
  const savedTheme = localStorage.getItem("ui-theme")
  if (isThemeName(savedTheme)) {
    setCurrentTheme(savedTheme)
  }

  // Apply theme to CSS custom properties and styles
  createEffect(() => {
    const name = currentTheme()
    const theme = themes[name]
    const root = document.documentElement

    Object.entries(theme.colors ?? {}).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value)
    })
    if (theme.tokens?.font) root.style.setProperty("--font", theme.tokens.font)
    if (theme.tokens?.["font-mono"]) {
      root.style.setProperty("--font-mono", theme.tokens["font-mono"])
    }

    document.getElementById("additional-styles")?.remove()
    const style = document.createElement("style")
    style.setAttribute("id", "additional-styles")
    style.innerHTML = themeCss[name]
    document.head.appendChild(style)
  })

  const setTheme = (theme: ThemeName) => {
    if (!isThemeName(theme)) return
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
