// import { useTheme } from "../lib/theme-context"
import ThemeSelector from "./ThemeSelector"

export default function Navbar() {
  return (
    <header
      class="px-6 py-4 border-b"
      style={`background-color: rgb(var(--darkSlate-800)); border-color: rgb(var(--darkSlate-700))`}
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-4">
          <h1
            class="text-2xl font-bold"
            style={`color: rgb(var(--lightSlate-50))`}
          >
            GDLauncher UI
          </h1>
          <span class="text-sm" style={`color: rgb(var(--lightSlate-300))`}>
            Component Showcase
          </span>
        </div>

        <div class="flex items-center space-x-4">
          <ThemeSelector />
        </div>
      </div>
    </header>
  )
}
