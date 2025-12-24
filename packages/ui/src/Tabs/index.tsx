import {
  splitProps,
  type ParentProps,
  type ValidComponent,
  type ComponentProps,
  createContext,
  useContext
} from "solid-js"
import type { PolymorphicProps } from "@kobalte/core/polymorphic"
import { Tabs as TabsPrimitive } from "@kobalte/core/tabs"
import { cn } from "../util"
import { PRESS_CLASSES } from "../Clickable"

type TabSize = "small" | "medium" | "large"

// Context to pass size down to children
const TabsSizeContext = createContext<TabSize>("small")

// Size classes matching Button component
const sizeClasses = {
  small: "h-9", // 36px
  medium: "h-11", // 44px
  large: "h-12" // 48px
}

const triggerPaddingClasses = {
  small: "px-3 py-1.5",
  medium: "px-4 py-2",
  large: "px-5 py-2.5"
}

// Re-export the root Tabs component with default styling
type TabsRootProps = ComponentProps<typeof TabsPrimitive> & {
  class?: string
  size?: TabSize
}

export const Tabs = (props: TabsRootProps) => {
  const [local, rest] = splitProps(props, ["class", "size"])

  return (
    <TabsSizeContext.Provider value={local.size || "small"}>
      <TabsPrimitive
        class={cn("flex h-full w-full flex-col", local.class)}
        {...rest}
      />
    </TabsSizeContext.Provider>
  )
}

// TabsList - Container for tab triggers
type TabsListProps = ParentProps<
  ComponentProps<typeof TabsPrimitive.List> & {
    class?: string
    size?: TabSize
  }
>

export const TabsList = <T extends ValidComponent = "div">(
  props: PolymorphicProps<T, TabsListProps>
) => {
  const [local, rest] = splitProps(props as TabsListProps, [
    "class",
    "children",
    "size"
  ])
  const contextSize = useContext(TabsSizeContext)
  const size = () => local.size || contextSize

  return (
    <TabsPrimitive.List
      class={cn(
        "relative flex items-center gap-1 rounded-lg bg-darkSlate-700",
        sizeClasses[size()],
        local.class
      )}
      {...rest}
    >
      {local.children}
    </TabsPrimitive.List>
  )
}

// TabsIndicator - Animated background indicator
type TabsIndicatorProps = ParentProps<
  ComponentProps<typeof TabsPrimitive.Indicator> & { class?: string }
>

export const TabsIndicator = <T extends ValidComponent = "div">(
  props: PolymorphicProps<T, TabsIndicatorProps>
) => {
  const [local, rest] = splitProps(props as TabsIndicatorProps, ["class"])

  return (
    <TabsPrimitive.Indicator
      class={cn(
        "absolute inset-0 rounded-lg bg-primary-500 shadow-sm transition-[transform,width,height] duration-200",
        local.class
      )}
      {...rest}
    />
  )
}

// TabsTrigger - Individual tab button
type TabsTriggerProps = ParentProps<
  ComponentProps<typeof TabsPrimitive.Trigger> & {
    class?: string
    size?: TabSize
  }
>

export const TabsTrigger = <T extends ValidComponent = "button">(
  props: PolymorphicProps<T, TabsTriggerProps>
) => {
  const [local, rest] = splitProps(props as TabsTriggerProps, [
    "class",
    "children",
    "size"
  ])
  const contextSize = useContext(TabsSizeContext)
  const size = () => local.size || contextSize

  return (
    <TabsPrimitive.Trigger
      class={cn(
        "relative z-10 flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium text-lightSlate-600 transition-all duration-200",
        triggerPaddingClasses[size()],
        "hover:text-lightSlate-400",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2 focus-visible:ring-offset-darkSlate-900",
        "disabled:pointer-events-none disabled:opacity-50",
        "data-[selected]:text-lightSlate-50 data-[selected]:hover:text-lightSlate-50",
        "animate-icons-on-hover",
        PRESS_CLASSES,
        local.class
      )}
      {...rest}
    >
      {local.children}
    </TabsPrimitive.Trigger>
  )
}

// TabsContent - Panel content for each tab
type TabsContentProps = ParentProps<
  ComponentProps<typeof TabsPrimitive.Content> & { class?: string }
>

export const TabsContent = <T extends ValidComponent = "div">(
  props: PolymorphicProps<T, TabsContentProps>
) => {
  const [local, rest] = splitProps(props as TabsContentProps, [
    "class",
    "children"
  ])

  return (
    <TabsPrimitive.Content
      class={cn("h-full w-full flex-1 focus-visible:outline-none", local.class)}
      {...rest}
    >
      {local.children}
    </TabsPrimitive.Content>
  )
}
