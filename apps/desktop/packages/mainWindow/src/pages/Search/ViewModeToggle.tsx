import useSearchContext from "@/components/SearchInputContext"
import { Button } from "@gd/ui"

export function ViewModeToggle() {
  const searchContext = useSearchContext()

  const switchToMode = (mode: "list" | "grid") => {
    if (searchContext?.viewMode() !== mode) {
      // Reset scroll when switching views since they have different layouts
      searchContext?.setLastScrollOffset(0)
      searchContext?.setViewMode(mode)
    }
  }

  return (
    <div class="flex items-center gap-1">
      <Button
        type={searchContext?.viewMode() === "list" ? "primary" : "glass"}
        size="small"
        onClick={() => switchToMode("list")}
        class="px-2"
      >
        <div class="i-hugeicons:left-to-right-list-bullet h-4 w-4" />
      </Button>
      <Button
        type={searchContext?.viewMode() === "grid" ? "primary" : "glass"}
        size="small"
        onClick={() => switchToMode("grid")}
        class="px-2"
      >
        <div class="i-hugeicons:dashboard-square-01 h-4 w-4" />
      </Button>
    </div>
  )
}

export default ViewModeToggle
