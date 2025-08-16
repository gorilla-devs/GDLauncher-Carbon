import { useTheme } from "../lib/theme-context"
import { createSignal, For } from "solid-js"

export default function ThemeSelector() {
  const { currentTheme, setTheme } = useTheme()
  const [isOpen, setIsOpen] = createSignal(false)

  const themes = [
    { value: "main" as const, label: "Main Theme" },
    { value: "pixelato" as const, label: "Pixelato" },
    { value: "win95" as const, label: "Windows 95" }
  ]

  const handleThemeSelect = (theme: "main" | "pixelato" | "win95") => {
    setTheme(theme)
    setIsOpen(false)
  }

  return (
    <div class="relative">
      <button
        onClick={() => setIsOpen(!isOpen())}
        class="flex items-center space-x-2 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
      >
        <span>
          Theme: {themes.find((t) => t.value === currentTheme())?.label}
        </span>
        <svg
          class={`w-4 h-4 transition-transform ${isOpen() ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>

      {isOpen() && (
        <div class="absolute right-0 mt-2 w-48 bg-white border border-gray-200 rounded-md shadow-lg z-50">
          <div class="py-1">
            <For each={themes}>
              {(theme) => (
                <button
                  onClick={() => handleThemeSelect(theme.value)}
                  class={`block w-full px-4 py-2 text-left text-sm hover:bg-gray-100 ${
                    currentTheme() === theme.value
                      ? "bg-indigo-50 text-indigo-700 font-medium"
                      : "text-gray-700"
                  }`}
                >
                  {theme.label}
                </button>
              )}
            </For>
          </div>
        </div>
      )}
    </div>
  )
}
