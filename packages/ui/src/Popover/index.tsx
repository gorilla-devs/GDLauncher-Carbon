import { cn } from "../util"
import type { PolymorphicProps } from "@kobalte/core/polymorphic"
import type {
  PopoverContentProps,
  PopoverRootProps
} from "@kobalte/core/popover"
import { Popover as PopoverPrimitive } from "@kobalte/core/popover"
import type { ParentProps, ValidComponent } from "solid-js"
import { mergeProps, splitProps } from "solid-js"

export const PopoverTrigger = PopoverPrimitive.Trigger
export const PopoverTitle = PopoverPrimitive.Title
export const PopoverDescription = PopoverPrimitive.Description

export const Popover = (props: PopoverRootProps) => {
  const merge = mergeProps<PopoverRootProps[]>(
    {
      gutter: 4,
      flip: false
    },
    props
  )

  return <PopoverPrimitive {...merge} />
}

type popoverContentProps<T extends ValidComponent = "div"> = ParentProps<
  PopoverContentProps<T> & {
    class?: string
    hideCloseButton?: boolean
  }
>

export const PopoverContent = <T extends ValidComponent = "div">(
  props: PolymorphicProps<T, popoverContentProps<T>>
) => {
  const [local, rest] = splitProps(props as popoverContentProps, [
    "class",
    "children",
    "hideCloseButton"
  ])

  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        class={cn(
          "z-100000 w-72 rounded-md border border-solid border-darkSlate-500 bg-darkSlate-700 p-4 text-lightSlate-200 shadow-lg shadow-darkSlate-900/50 outline-none data-[expanded]:animate-popoverEnter data-[closed]:animate-popoverLeave",
          local.class
        )}
        {...rest}
      >
        {local.children}
        {!local.hideCloseButton && (
          <PopoverPrimitive.CloseButton class="absolute right-4 top-4 flex h-6 w-6 items-center justify-center rounded-md text-lightSlate-400 transition-all hover:bg-darkSlate-600 hover:text-lightSlate-200 active:scale-95 focus:outline-none focus:ring-[1.5px] focus:ring-darkSlate-500 disabled:pointer-events-none">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              class="h-4 w-4"
            >
              <path
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M18 6L6 18M6 6l12 12"
              />
              <title>Close</title>
            </svg>
          </PopoverPrimitive.CloseButton>
        )}
      </PopoverPrimitive.Content>
    </PopoverPrimitive.Portal>
  )
}
