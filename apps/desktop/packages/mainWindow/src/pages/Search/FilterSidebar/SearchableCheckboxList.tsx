import { Checkbox } from "@gd/ui"
import {
  Index,
  Show,
  createSignal,
  createMemo,
  createEffect,
  onCleanup,
  type JSX
} from "solid-js"
import { VList } from "@/components/VirtuaWrapper"

interface CheckboxItem {
  label: string | null | undefined
  value: string | number
  icon?: JSX.Element
}

interface SearchableCheckboxListProps {
  items: CheckboxItem[]
  selectedValues: () => (string | number)[]
  onToggle: (value: string | number, checked: boolean) => void
  searchPlaceholder?: string
  showSearch?: boolean
  maxHeight?: number
  virtualizeThreshold?: number
  emptyMessage?: JSX.Element
  /** Hide search when item count is below this threshold (default: 10) */
  searchThreshold?: number
}

const SEARCH_THRESHOLD = 10

export function SearchableCheckboxList(props: SearchableCheckboxListProps) {
  const [searchQuery, setSearchQuery] = createSignal("")
  const [debouncedQuery, setDebouncedQuery] = createSignal("")
  let inputRef: HTMLInputElement | undefined

  createEffect(() => {
    const query = searchQuery()
    const timeoutId = setTimeout(() => setDebouncedQuery(query), 150)
    onCleanup(() => clearTimeout(timeoutId))
  })

  const filteredItems = createMemo(() => {
    const query = debouncedQuery().toLowerCase().trim()
    if (!query) return props.items
    return props.items.filter((item) =>
      (item.label ?? "").toLowerCase().includes(query)
    )
  })

  const maxHeight = () => props.maxHeight ?? 200
  const shouldVirtualize = createMemo(
    () => filteredItems().length > (props.virtualizeThreshold ?? 100)
  )

  const shouldShowSearch = () => {
    if (props.showSearch === false) return false
    const threshold = props.searchThreshold ?? SEARCH_THRESHOLD
    return props.items.length >= threshold
  }

  const renderItem = (item: CheckboxItem) => (
    <div class="hover:bg-darkSlate-700 rounded-md px-1.5 py-1 transition-colors">
      <Checkbox
        checked={props.selectedValues().includes(item.value)}
        onChange={(checked) => props.onToggle(item.value, checked)}
      >
        <div class="flex min-w-0 items-center gap-2 text-sm">
          <Show when={item.icon}>
            <div class="h-4 w-4 shrink-0">{item.icon}</div>
          </Show>
          <span class="truncate">{item.label}</span>
        </div>
      </Checkbox>
    </div>
  )

  return (
    <div class="flex flex-col gap-2">
      <Show when={shouldShowSearch()}>
        <div
          class="bg-darkSlate-700 flex items-center gap-2 rounded-md px-3 ring-1 ring-darkSlate-600"
          style={{ height: "32px" }}
        >
          <div class="i-hugeicons:search-01 text-darkSlate-400 h-4 w-4 shrink-0" />
          <input
            ref={inputRef}
            class="min-w-0 flex-1 bg-transparent text-sm text-lightSlate-50 placeholder:text-darkSlate-400 outline-none"
            placeholder={props.searchPlaceholder ?? "Search..."}
            value={searchQuery()}
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
          />
          <Show when={searchQuery().length > 0}>
            <div
              class="i-hugeicons:cancel-01 text-darkSlate-400 hover:text-lightSlate-300 h-4 w-4 shrink-0 cursor-pointer transition-colors"
              onClick={() => {
                setSearchQuery("")
                inputRef?.focus()
              }}
            />
          </Show>
        </div>
      </Show>

      <Show
        when={filteredItems().length > 0}
        fallback={
          <div class="text-lightSlate-400 px-2 py-3 text-center text-sm">
            {props.emptyMessage ?? "No results found"}
          </div>
        }
      >
        <Show
          when={shouldVirtualize()}
          fallback={
            <div
              class="flex flex-col gap-0.5 overflow-y-auto"
              style={{ "max-height": `${maxHeight()}px` }}
            >
              <Index each={filteredItems()}>
                {(item) => renderItem(item())}
              </Index>
            </div>
          }
        >
          <div class="overflow-hidden" style={{ height: `${maxHeight()}px` }}>
            <VList data={filteredItems()} class="h-full w-full">
              {(item) => renderItem(item)}
            </VList>
          </div>
        </Show>
      </Show>
    </div>
  )
}
