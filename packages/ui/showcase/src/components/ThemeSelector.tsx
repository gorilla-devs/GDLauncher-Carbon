import { useTheme } from "../lib/theme-context"
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem
} from "../../../src"

export default function ThemeSelector() {
  const { currentTheme, setTheme } = useTheme()

  const themes = [
    { value: "main", label: "Main Theme", description: "Default dark theme" },
    {
      value: "pixelato",
      label: "Pixelato",
      description: "Pixel art inspired theme"
    },
    {
      value: "win95",
      label: "Windows 95",
      description: "Retro Windows 95 style"
    },
    {
      value: "inferno",
      label: "Inferno",
      description: "Fiery crimson theme"
    },
    {
      value: "aether",
      label: "Aether",
      description: "Ethereal void theme"
    },
    {
      value: "frost",
      label: "Frost",
      description: "Icy arctic theme"
    }
  ]

  return (
    <div class="w-52">
      <Select
        options={themes.map((t) => t.value)}
        value={currentTheme()}
        onChange={(value) => setTheme(value!)}
        itemComponent={(props) => {
          const theme = themes.find((t) => t.value === props.item.rawValue)
          return <SelectItem item={props.item}>{theme?.label}</SelectItem>
        }}
      >
        <SelectTrigger>
          <SelectValue<string>>
            {(state) => {
              const theme = themes.find(
                (t) => t.value === state.selectedOption()
              )
              return theme?.label || "Select theme"
            }}
          </SelectValue>
        </SelectTrigger>
        <SelectContent />
      </Select>
      <div class="mt-1 text-xs" style="color: rgb(var(--lightSlate-400))">
        {themes.find((t) => t.value === currentTheme())?.description}
      </div>
    </div>
  )
}
