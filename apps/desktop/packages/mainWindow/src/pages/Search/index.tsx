import { JSX } from "solid-js"
import { FilterSidebar } from "./FilterSidebar"
import { DecorationSlot } from "@/themes/DecorationSlot"

export function Search(props: { children?: JSX.Element }) {
  return (
    <div
      data-gdl="panel-frame"
      style={{
        // Documented for themes: the panel box inside this frame, and its radius.
        "--gdl-panel-inset": "1rem 1rem 0 0",
        "--gdl-panel-radius": "0 calc(1rem * var(--radius-scale, 1)) 0 0"
      }}
      class="text-lightSlate-50 bg-darkSlate-700 relative h-content box-border flex max-h-full min-h-full w-full flex-1 justify-center overflow-hidden pr-4 pt-4"
    >
      <div
        data-gdl="panel"
        id="gdl-content-wrapper"
        class="bg-darkSlate-800 relative isolate box-border flex h-auto w-full flex-1 flex-col overflow-auto rounded-r-2xl rounded-br-none"
        style={{
          "scrollbar-gutter": "stable"
        }}
      >
        <div class="flex h-full flex-1 overflow-hidden">
          <FilterSidebar />
          <div class="flex h-full flex-1 flex-col overflow-hidden">
            {props.children}
          </div>
        </div>
      </div>
      <DecorationSlot
        anchor="panel"
        class="!left-0 !right-4 !top-4 !bottom-0"
      />
    </div>
  )
}

export default Search
