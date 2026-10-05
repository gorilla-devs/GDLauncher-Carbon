import { For } from "solid-js"
import { vi } from "vitest"

export const Box = (props: any) => <div>{props.children}</div>
export const Button = (props: any) => (
  <button
    ref={(element) => {
      ;(element as any).task29Click = props.onClick
    }}
    disabled={props.disabled}
    onClick={props.onClick}
  >
    {props.children}
  </button>
)
export const Input = (props: any) => (
  <input value={props.value} onInput={props.onInput} onBlur={props.onBlur} />
)
export const Spinner = () => <span data-testid="spinner">loading</span>
export const Skeleton = Object.assign(() => <span>skeleton</span>, {
  addonVersionsTable: () => <span data-testid="skeleton">skeleton</span>
})
export const Badge = (props: any) => (
  <button onClick={props.onClick}>{props.children}</button>
)
export const Select = (props: any) => (
  <select
    value={props.value}
    disabled={props.disabled}
    onChange={(event) => props.onChange(event.currentTarget.value)}
  >
    <For each={props.options}>
      {(value) => <option value={value}>{value}</option>}
    </For>
  </select>
)
export const toast = { success: vi.fn(), error: vi.fn() }
export const Slider = Box
export const Switch = Box
export const Tabs = Box
export const TabsList = Box
export const TabsTrigger = Box
export const TabsContent = Box
export const TabsIndicator = Box
export const Tooltip = Box
export const TooltipContent = Box
export const TooltipTrigger = Box
export const SelectTrigger = Box
export const SelectContent = Box
export const SelectItem = Box
export const SelectValue = Box
export const Popover = Box
export const PopoverContent = Box
export const PopoverTrigger = Box
export const DropdownMenuTrigger = Box
export const DropdownMenuContent = Box
export const DropdownMenuItem = Box
export const DropdownMenu = Box

export function deferred<T = any>() {
  let resolve!: (value: T) => void
  let reject!: (error: any) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
export const settle = () =>
  new Promise<void>((resolve) => setTimeout(resolve, 15))
