import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { JSX, Show } from "solid-js"
import {
  BlockedText,
  type ChangesBlocked
} from "@/pages/Library/shared/addons/components/ChangesBlocked"

/** A row or header action drawn as an icon, as the Addons table draws them.
 *  A disabled one says why in its tooltip: `blocked`, else `reason`. */
const IconAction = (props: {
  icon: string
  label: JSX.Element
  onClick: () => void
  disabled?: boolean
  blocked?: ChangesBlocked
  reason?: JSX.Element
  danger?: boolean
  testId: string
}) => (
  <Tooltip>
    <TooltipTrigger>
      <div
        data-testid={props.testId}
        data-disabled={!!props.disabled}
        class={`${props.icon} h-5 w-5 shrink-0 transition-colors`}
        classList={{
          "text-lightSlate-700 cursor-not-allowed": !!props.disabled,
          "text-lightSlate-500 cursor-pointer": !props.disabled,
          "hover:text-red-400": !props.disabled && !!props.danger,
          "hover:text-lightSlate-200": !props.disabled && !props.danger
        }}
        onClick={() => {
          if (!props.disabled) props.onClick()
        }}
      />
    </TooltipTrigger>
    <TooltipContent>
      <Show
        when={props.disabled && props.blocked}
        fallback={
          <Show when={props.disabled && props.reason} fallback={props.label}>
            {props.reason}
          </Show>
        }
      >
        <BlockedText blocked={props.blocked!} />
      </Show>
    </TooltipContent>
  </Tooltip>
)

export default IconAction
