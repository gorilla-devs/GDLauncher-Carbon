import { createContext, useContext, Show } from "solid-js"

export const Wrapper = (props: any) => <div>{props.children}</div>
export const Button = (props: any) => (
  <button disabled={props.disabled} title={props.title} onClick={props.onClick}>
    {props.children}
  </button>
)
export const Input = (props: any) => (
  <input
    ref={props.ref}
    value={props.value ?? ""}
    disabled={props.disabled}
    onInput={props.onInput}
    onChange={props.onChange}
    onKeyDown={props.onKeyDown}
    onBlur={props.onBlur}
  />
)
export const Checkbox = (props: any) => (
  <input type="checkbox" checked={props.checked} />
)
export const Switch = (props: any) => (
  <input type="checkbox" checked={props.checked} onChange={props.onChange} />
)
export const Radio = { group: () => <div /> }
export const Slider = () => <div />
export const Select = () => <div />
export const SelectTrigger = Wrapper
export const SelectContent = Wrapper
export const SelectItem = Wrapper
export const SelectValue = () => <div />
const TabContext = createContext<any>()
export const Tabs = (props: any) => (
  <TabContext.Provider value={props}>{props.children}</TabContext.Provider>
)
export const TabsList = Wrapper
export const TabsIndicator = () => <div />
export const TabsTrigger = (props: any) => {
  const tabs = useContext(TabContext)
  return (
    <button onClick={() => tabs.onChange?.(props.value)}>
      {props.children}
    </button>
  )
}
export const TabsContent = (props: any) => {
  const tabs = useContext(TabContext)
  return <Show when={tabs.value === props.value}>{props.children}</Show>
}
export const Tooltip = Wrapper
export const TooltipTrigger = Wrapper
export const TooltipContent = Wrapper
export const Popover = Wrapper
export const PopoverTrigger = Wrapper
export const PopoverContent = Wrapper
export const DropdownMenu = Wrapper
export const DropdownMenuTrigger = Wrapper
export const DropdownMenuContent = Wrapper
export const DropdownMenuItem = Button
export const DropdownMenuLabel = Wrapper
export const DropdownMenuSeparator = () => <hr />
export const ContextMenu = Wrapper
export const ContextMenuTrigger = Wrapper
export const ContextMenuContent = Wrapper
export const ContextMenuGroup = Wrapper
export const ContextMenuGroupLabel = Wrapper
export const ContextMenuItem = Button
export const ContextMenuSeparator = () => <hr />
export const Spinner = () => <div />
export const Skeleton = { instances: () => <div /> }
