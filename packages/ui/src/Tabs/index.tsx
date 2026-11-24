import {
  splitProps,
  type ParentProps,
  type ValidComponent,
  type ComponentProps,
  createContext,
  useContext,
  createSignal
} from "solid-js"
import type { PolymorphicProps } from "@kobalte/core/polymorphic"
import { Tabs as TabsPrimitive } from "@kobalte/core/tabs"
import { cn } from "../util"
import { PRESS_CLASSES } from "../Clickable"
import { gdlHooks } from "../hooks"

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

interface HookProps {
  "data-gdl"?: string
}

// Re-export the root Tabs component with default styling
type TabsRootProps = ComponentProps<typeof TabsPrimitive> & {
  class?: string
  size?: TabSize
}

// Context carrying the current tab value, so a trigger can mark itself active
const TabsValueContext = createContext<() => string | undefined>(
  () => undefined
)

export const Tabs = (props: TabsRootProps) => {
  const [local, rest] = splitProps(props, ["class", "size", "onChange"])
  const [internal, setInternal] = createSignal<string | undefined>(
    props.value ?? props.defaultValue
  )
  const current = () => (props.value !== undefined ? props.value : internal())

  return (
    <TabsSizeContext.Provider value={local.size || "small"}>
      <TabsValueContext.Provider value={current}>
        <TabsPrimitive
          class={cn("flex h-full w-full flex-col", local.class)}
          {...rest}
          onChange={(value: string) => {
            setInternal(value)
            local.onChange?.(value)
          }}
        />
      </TabsValueContext.Provider>
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
  const [local, rest] = splitProps(props as TabsListProps & HookProps, [
    "class",
    "children",
    "size",
    "data-gdl"
  ])
  const contextSize = useContext(TabsSizeContext)
  const size = () => local.size || contextSize

  return (
    <TabsPrimitive.List
      data-gdl={gdlHooks("tabs", local["data-gdl"])}
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
  const [local, rest] = splitProps(props as TabsIndicatorProps & HookProps, [
    "class",
    "data-gdl"
  ])

  return (
    <TabsPrimitive.Indicator
      data-gdl={gdlHooks("tabs.indicator", local["data-gdl"])}
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
  const [local, rest] = splitProps(props as TabsTriggerProps & HookProps, [
    "class",
    "children",
    "size",
    "data-gdl"
  ])
  const contextSize = useContext(TabsSizeContext)
  const currentValue = useContext(TabsValueContext)
  const size = () => local.size || contextSize
  const active = () =>
    currentValue() !== undefined && currentValue() === props.value

  return (
    <TabsPrimitive.Trigger
      data-gdl={gdlHooks(
        active() ? "tab tab.active" : "tab",
        local["data-gdl"]
      )}
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
