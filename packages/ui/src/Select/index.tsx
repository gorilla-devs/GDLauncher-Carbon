import { splitProps, type ParentProps, type ValidComponent } from "solid-js"
import type { PolymorphicProps } from "@kobalte/core/polymorphic"
import {
  Select as SelectPrimitive,
  type SelectContentProps,
  type SelectItemProps,
  type SelectTriggerProps
} from "@kobalte/core/select"

import { cn } from "../util"
import { PRESS_CLASSES } from "../Clickable"

export const Select = SelectPrimitive
export const SelectValue = SelectPrimitive.Value
export const SelectDescription = SelectPrimitive.Description
export const SelectErrorMessage = SelectPrimitive.ErrorMessage
export const SelectItemDescription = SelectPrimitive.ItemDescription
export const SelectHiddenSelect = SelectPrimitive.HiddenSelect
export const SelectSection = SelectPrimitive.Section

type selectTriggerProps<T extends ValidComponent = "button"> = ParentProps<
  SelectTriggerProps<T> & { class?: string; variant?: "default" | "unstyled" }
>

export const SelectTrigger = <T extends ValidComponent = "button">(
  props: PolymorphicProps<T, selectTriggerProps<T>>
) => {
  const [local, rest] = splitProps(props as selectTriggerProps, [
    "class",
    "children",
    "variant"
  ])

  return (
    <SelectPrimitive.Trigger
      class={cn(
        local.variant !== "unstyled" &&
          "flex h-9 w-full items-center justify-between rounded-md border-0 border-transparent outline-none hover:outline-darkSlate-600 focus-visible:outline-darkSlate-500 !bg-darkSlate-800 px-3 py-2 text-sm text-lightSlate-100 shadow-sm ring-offset-darkSlate-900 placeholder:text-darkSlate-400 disabled:cursor-not-allowed disabled:opacity-50",
        local.variant === "unstyled" && "flex items-center justify-between",
        PRESS_CLASSES,
        local.class
      )}
      {...rest}
    >
      {local.children}
      {local.variant !== "unstyled" && (
        <SelectPrimitive.Icon
          as="svg"
          xmlns="http://www.w3.org/2000/svg"
          width="1em"
          height="1em"
          viewBox="0 0 24 24"
          class="flex size-4 items-center justify-center opacity-50"
        >
          <path
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="m8 9l4-4l4 4m0 6l-4 4l-4-4"
          />
        </SelectPrimitive.Icon>
      )}
    </SelectPrimitive.Trigger>
  )
}

type selectContentProps<T extends ValidComponent = "div"> =
  SelectContentProps<T> & {
    class?: string
  }

export const SelectContent = <T extends ValidComponent = "div">(
  props: PolymorphicProps<T, selectContentProps<T>>
) => {
  const [local, rest] = splitProps(props as selectContentProps, ["class"])
  let listboxRef: HTMLElement | undefined

  const handleListboxRef = (el: HTMLElement) => {
    listboxRef = el

    // Scroll the selected item into view within the dropdown only
    // Use manual scroll calculation to avoid affecting parent scroll containers
    setTimeout(() => {
      const selectedItem = listboxRef?.querySelector(
        '[aria-selected="true"]'
      ) as HTMLElement | null
      const scrollContainer = listboxRef?.parentElement
      if (selectedItem && scrollContainer) {
        const containerRect = scrollContainer.getBoundingClientRect()
        const itemRect = selectedItem.getBoundingClientRect()
        const relativeTop =
          itemRect.top - containerRect.top + scrollContainer.scrollTop
        const centerOffset =
          relativeTop - scrollContainer.clientHeight / 2 + itemRect.height / 2
        scrollContainer.scrollTop = Math.max(0, centerOffset)
      }
    }, 0)
  }

  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        class={cn(
          "relative z-[100000] min-w-[8rem] rounded-md border border-darkSlate-500 bg-darkSlate-800 text-lightSlate-100 shadow-md data-[expanded]:animate-selectEnter data-[closed]:animate-selectLeave",
          local.class
        )}
        {...rest}
      >
        <div class="max-h-80 overflow-y-auto">
          <SelectPrimitive.Listbox
            ref={handleListboxRef}
            class="p-1 focus-visible:outline-none"
          />
        </div>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  )
}

type selectItemProps<T extends ValidComponent = "li"> = ParentProps<
  SelectItemProps<T> & { class?: string }
>

export const SelectItem = <T extends ValidComponent = "li">(
  props: PolymorphicProps<T, selectItemProps<T>>
) => {
  const [local, rest] = splitProps(props as selectItemProps, [
    "class",
    "children"
  ])

  return (
    <SelectPrimitive.Item
      class={cn(
        "relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-2 pr-8 text-sm outline-none focus:bg-darkSlate-700 focus:text-lightSlate-50 data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
        local.class
      )}
      {...rest}
    >
      <SelectPrimitive.ItemIndicator class="absolute right-2 flex h-3.5 w-3.5 items-center justify-center">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-4 w-4"
          viewBox="0 0 24 24"
        >
          <path
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="m5 12l5 5L20 7"
          />
          <title>Checked</title>
        </svg>
      </SelectPrimitive.ItemIndicator>
      <SelectPrimitive.ItemLabel class="flex-1 w-full">
        {local.children}
      </SelectPrimitive.ItemLabel>
    </SelectPrimitive.Item>
  )
}
