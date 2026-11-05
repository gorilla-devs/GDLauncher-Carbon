import { createContext, createSignal, For, Show, useContext } from "solid-js"
import { vi } from "vitest"

export const Box = (p: any) => <div>{p.children}</div>
export const Button = (p: any) => (
  <button
    data-testid={p["data-testid"]}
    disabled={p.disabled}
    data-loading={!!p.loading}
    ref={(e) => {
      ;(e as any).invoke = p.onClick
    }}
    onClick={p.onClick}
  >
    {p.children}
  </button>
)
export const Input = (p: any) => (
  <input
    data-testid={p["data-testid"]}
    placeholder={p.placeholder}
    value={p.value}
    disabled={p.disabled}
    onInput={p.onInput}
  />
)
export const Select = (p: any) => (
  <select
    data-value={p.value}
    value={p.value}
    disabled={p.disabled}
    ref={(e) => {
      ;(e as any).choose = p.onChange
    }}
    onChange={(e) => p.onChange(e.currentTarget.value)}
  >
    <For each={p.options}>
      {(o) => (
        <option value={typeof o === "string" ? o : o.value}>
          {typeof o === "string" ? o : o.label}
        </option>
      )}
    </For>
  </select>
)
export const Badge = (p: any) => (
  <button onClick={p.onClick}>{p.children}</button>
)
export const Radio = (p: any) => (
  <label>
    <input
      type="radio"
      value={p.value}
      checked={p.checked}
      onChange={() => p.onChange(p.value)}
    />
    {p.children}
  </label>
)
export const Steps = (p: any) => (
  <For each={p.steps}>
    {(s) => <button onClick={s.onClick}>{s.label}</button>}
  </For>
)
export const Progress = (p: any) => (
  <span data-testid="progress" data-value={p.value} />
)
export const Spinner = () => <span data-testid="spinner">loading</span>
export const Skeleton = Box
export const PRESS_CLASSES = ""
export const toast = {
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  warning: vi.fn()
}
const Tab = createContext<any>()
export const Tabs = (p: any) => {
  const [value, setValue] = createSignal(p.defaultValue)
  return <Tab.Provider value={{ value, setValue }}>{p.children}</Tab.Provider>
}
export const TabsTrigger = (p: any) => {
  const tab = useContext(Tab)
  return (
    <button
      data-testid={p["data-testid"]}
      onClick={() => tab.setValue(p.value)}
    >
      {p.children}
    </button>
  )
}
export const TabsContent = (p: any) => {
  const tab = useContext(Tab)
  return <Show when={tab.value() === p.value}>{p.children}</Show>
}
export const TabsList = Box
export const TabsIndicator = Box
export const SelectTrigger = Box
export const SelectContent = Box
export const SelectItem = Box
export const SelectValue = Box
export const Popover = Box
export const PopoverTrigger = Box
export const PopoverContent = Box
export const Tooltip = Box
export const TooltipTrigger = Box
export const TooltipContent = Box
export const Collapsable = Box
export const DropdownMenu = Box
export const DropdownMenuTrigger = Box
export const DropdownMenuContent = Box
export const DropdownMenuItem = Box
export function deferred<T = any>() {
  let resolve!: (v: T) => void, reject!: (e: any) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
export async function settle() {
  for (let i = 0; i < 20; i++) await Promise.resolve()
}
export function button(root: HTMLElement, key: string) {
  const result = Array.from(root.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(key)
  )
  if (!result) throw new Error(`Missing button ${key}`)
  return result
}
export function input(root: HTMLElement, index: number, value: string) {
  const e = root.querySelectorAll("input")[index]
  e.value = value
  e.dispatchEvent(new Event("input", { bubbles: true }))
}
