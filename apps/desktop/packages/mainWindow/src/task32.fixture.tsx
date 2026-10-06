export * from "./task31.fixture"
export { Checkbox } from "../../../../../packages/ui/src/Checkbox"
export { Radio } from "../../../../../packages/ui/src/Radio"
export { Steps } from "../../../../../packages/ui/src/Steps"
import { For, Show, createSignal } from "solid-js"
export const Input = (p: any) => {
  const [menu, setMenu] = createSignal(false)
  return (
    <div>
      <input
        placeholder={p.placeholder}
        value={p.value}
        disabled={p.disabled}
        onInput={(e) => {
          p.onInput?.(e)
          p.onSearch?.(e.currentTarget.value)
          setMenu(true)
        }}
        onBlur={(e) => {
          p.onBlur?.(e)
          setMenu(false)
        }}
      />
      {p.icon}
      <Show when={menu()}>
        <For each={p.autoCompleteOptions}>{(o) => <div>{o.label}</div>}</For>
      </Show>
    </div>
  )
}
export function pointerGesture(e: HTMLElement) {
  let captured = false
  e.setPointerCapture = () => {
    captured = true
  }
  e.hasPointerCapture = () => captured
  e.releasePointerCapture = () => {
    captured = false
  }
  e.dispatchEvent(new Event("pointerdown", { bubbles: true }))
  e.dispatchEvent(new Event("pointerup", { bubbles: true }))
  e.dispatchEvent(new MouseEvent("click", { bubbles: true }))
}
