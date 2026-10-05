export * from "../task25/ui"
import { Wrapper } from "../task25/ui"
export const DropdownMenuPortal = Wrapper
export const DropdownMenuSub = Wrapper
export const DropdownMenuSubContent = Wrapper
export const DropdownMenuSubTrigger = Wrapper
export const DropdownMenuRadioGroup = Wrapper
export const DropdownMenuRadioItem = Wrapper
export const DropdownMenuCheckboxItem = Wrapper
export const Collapsable = (p: any) => (
  <div data-custom-header={!!p.customHeader}>
    {p.title}
    {p.children}
  </div>
)
export const Switch = (p: any) => (
  <input
    type="checkbox"
    checked={p.checked}
    disabled={p.disabled}
    onChange={p.onChange}
  />
)
export const Select = (p: any) => (
  <select
    disabled={p.disabled}
    value={p.value}
    onChange={(e) => p.onChange(e.currentTarget.value)}
  >
    {p.options?.map((v: string) => (
      <option value={v}>{v}</option>
    ))}
  </select>
)
