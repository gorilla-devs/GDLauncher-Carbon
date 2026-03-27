import useSearchContext from "@/components/SearchInputContext"

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
    <div class="flex items-center gap-0.5">
      <button
        class="flex items-center justify-center rounded p-1 transition-colors border-none bg-transparent text-inherit"
        classList={{
          "bg-darkSlate-600 text-white": searchContext?.viewMode() === "list",
          "hover:bg-darkSlate-700 text-lightSlate-700": searchContext?.viewMode() !== "list"
        }}
        onClick={() => switchToMode("list")}
      >
        <div class="i-hugeicons:left-to-right-list-bullet h-4 w-4" />
      </button>
      <button
        class="flex items-center justify-center rounded p-1 transition-colors border-none bg-transparent text-inherit"
        classList={{
          "bg-darkSlate-600 text-white": searchContext?.viewMode() === "grid",
          "hover:bg-darkSlate-700 text-lightSlate-700": searchContext?.viewMode() !== "grid"
        }}
        onClick={() => switchToMode("grid")}
      >
        <div class="i-hugeicons:dashboard-square-01 h-4 w-4" />
      </button>
    </div>
  )
}

export default ViewModeToggle
