import { Input } from "@gd/ui"
import { For, Show, createSignal, createEffect, JSX } from "solid-js"
import { VList } from "./VirtuaWrapper"

export interface SearchableOption {
  label: string
  value: string | number
  icon?: JSX.Element
  disabled?: boolean
}

interface SearchableMultiSelectProps {
  options: SearchableOption[]
  selectedValues: (string | number)[]
  placeholder?: string
  searchPlaceholder?: string
  emptyMessage?: string
  maxHeight?: string
  virtualized?: boolean
  itemHeight?: number
  virtualizationThreshold?: number
  onToggle: (option: SearchableOption, checked: boolean) => void
  children: JSX.Element
}

export function SearchableMultiSelect(props: SearchableMultiSelectProps) {
  const [searchQuery, setSearchQuery] = createSignal("")
  const [isOpen, setIsOpen] = createSignal(false)
  let inputRef: HTMLInputElement | undefined

  const filteredOptions = () => {
    const query = searchQuery().toLowerCase().trim()
    if (!query) return props.options

    return props.options.filter(
      (option) =>
        option.label.toLowerCase().includes(query) ||
        option.value.toString().toLowerCase().includes(query)
    )
  }

  const shouldVirtualize = () => {
    const threshold = props.virtualizationThreshold ?? 100
    const useVirtualization =
      props.virtualized ?? filteredOptions().length > threshold
    return useVirtualization
  }

  const isSelected = (option: SearchableOption) => {
    return props.selectedValues.includes(option.value)
  }

  const handleToggle = (option: SearchableOption) => {
    if (option.disabled) return
    const wasSelected = isSelected(option)
    props.onToggle(option, !wasSelected)
  }

  // Auto-focus input when dropdown opens
  createEffect(() => {
    if (isOpen()) {
      const timer = setTimeout(() => {
        if (inputRef) {
          inputRef.focus()
        }
      }, 10)
      return () => clearTimeout(timer)
    }
  })

  const renderOption = (option: SearchableOption) => (
    <div
      class={`flex items-center gap-2 px-2 py-2 text-sm rounded cursor-pointer transition-colors ${
        option.disabled
          ? "opacity-50 cursor-not-allowed"
          : "hover:bg-darkSlate-700 focus:bg-darkSlate-700"
      }`}
      onClick={() => handleToggle(option)}
    >
      {/* Custom Checkbox */}
      <div
        class={`flex h-4 w-4 items-center justify-center rounded border border-darkSlate-500 flex-shrink-0 ${
          isSelected(option)
            ? "bg-primary-500 border-primary-500"
            : "bg-darkSlate-700"
        }`}
      >
        <Show when={isSelected(option)}>
          <div class="i-ri:check-line h-3 w-3 text-white" />
        </Show>
      </div>

      <Show when={option.icon}>
        <div class="h-4 w-4 flex-shrink-0">{option.icon}</div>
      </Show>
      <span class="flex-1 truncate">{option.label}</span>
    </div>
  )

  return (
    <div class="relative">
      <div onClick={() => setIsOpen(!isOpen())} class="cursor-pointer">
        {props.children}
      </div>

      <Show when={isOpen()}>
        <div class="absolute top-full left-0 right-0 z-50 mt-1 rounded-md border border-darkSlate-600 bg-darkSlate-800 shadow-lg">
          <div
            class="p-2 border-b border-darkSlate-600"
            onClick={(e) => {
              e.stopPropagation()
              if (inputRef) {
                inputRef.focus()
              }
            }}
          >
            <Input
              ref={inputRef}
              placeholder={props.searchPlaceholder || "Search..."}
              value={searchQuery()}
              onInput={(e) => setSearchQuery(e.target.value)}
              icon={<div class="i-ri:search-line h-4 w-4" />}
              variant="transparent"
              onClick={(e) => {
                e.stopPropagation()
                if (inputRef) {
                  inputRef.focus()
                }
              }}
            />
          </div>

          <div
            class={
              shouldVirtualize()
                ? "p-1 max-h-[200px]"
                : "max-h-[200px] overflow-y-auto p-1"
            }
            style={props.maxHeight ? { "max-height": props.maxHeight } : {}}
          >
            <Show
              when={filteredOptions().length > 0}
              fallback={
                <div class="px-2 py-3 text-sm text-lightSlate-400 text-center">
                  {props.emptyMessage || "No results found"}
                </div>
              }
            >
              <Show
                when={shouldVirtualize()}
                fallback={
                  <For each={filteredOptions()}>
                    {(option) => renderOption(option)}
                  </For>
                }
              >
                <VList
                  data={filteredOptions()}
                  itemSize={props.itemHeight ?? 32}
                  class="overflow-y-auto h-full"
                >
                  {(item) => renderOption(item)}
                </VList>
              </Show>
            </Show>
          </div>
        </div>
      </Show>

      {/* Backdrop to close dropdown */}
      <Show when={isOpen()}>
        <div
          class="fixed inset-0 z-40"
          onClick={() => {
            setIsOpen(false)
            setSearchQuery("")
          }}
        />
      </Show>
    </div>
  )
}
