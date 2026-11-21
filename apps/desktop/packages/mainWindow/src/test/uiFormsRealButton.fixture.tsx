export * from "./uiFormsRealInputs.fixture"
import { Button as ActualButton } from "../../../../../../packages/ui/src/Button"
export { Checkbox } from "../../../../../../packages/ui/src/Checkbox"
export {
  createHoldToConfirm,
  HoldFill,
  HoldRing
} from "../../../../../../packages/ui/src/HoldToConfirm"
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
import { toast as originalToast } from "./uiForms.fixture"
export const toast = Object.assign(vi.fn(), originalToast)
