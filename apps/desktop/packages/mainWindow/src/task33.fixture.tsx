export * from "./task32.fixture"
import { Button as ActualButton } from "../../../../../packages/ui/src/Button"
export { Checkbox } from "../../../../../packages/ui/src/Checkbox"
export const Button = (p: any) => (
  <ActualButton
    {...p}
    ref={(e) => {
      ;(e as any).invoke = p.onClick
      if (typeof p.ref === "function") p.ref(e)
    }}
  />
)
export const Separator = () => <hr />
import { vi } from "vitest"
import { toast as originalToast } from "./task31.fixture"
export const toast = Object.assign(vi.fn(), originalToast)
