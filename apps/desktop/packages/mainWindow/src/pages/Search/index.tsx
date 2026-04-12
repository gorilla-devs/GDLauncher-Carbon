import { JSX } from "solid-js"
import { FilterSidebar } from "./FilterSidebar"

export function Search(props: { children?: JSX.Element }) {
  return (
    <div class="text-lightSlate-50 bg-darkSlate-700 h-content box-border flex max-h-full min-h-full w-full flex-1 justify-center overflow-hidden pr-4 pt-4">
      <div
        id="gdl-content-wrapper"
        class="bg-darkSlate-800 relative box-border flex h-auto w-full flex-1 flex-col overflow-auto rounded-r-2xl rounded-br-none"
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
    </div>
  )
}

export default Search
