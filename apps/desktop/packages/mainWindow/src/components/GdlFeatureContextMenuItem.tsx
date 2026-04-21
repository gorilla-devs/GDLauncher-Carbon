import { JSX, Show } from "solid-js"
import { useLocation } from "@solidjs/router"
import { useTransContext } from "@gd/i18n"
import {
  ContextMenuItem,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { useGlobalStore } from "./GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import { logoUrl } from "@/utils/logos"

interface Props {
  children: JSX.Element
  icon?: JSX.Element
  onClick: () => void
  disabled?: boolean
  returnPath?: string
  class?: string
}

/**
 * Context menu item that requires a GDL account to function.
 * When no valid GDL account exists:
 * - Shows GDL logo badge at the end
 * - Applies primary color styling
 * - Opens modal explaining GDL account requirement on click
 *
 * When valid GDL account exists, works as a normal ContextMenuItem.
 */
const GdlFeatureContextMenuItem = (props: Props) => {
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  const location = useLocation()
  const [t] = useTransContext()

  const hasValidAccount = () => globalStore.gdlAccount.data?.status === "valid"

  const handleClick = () => {
    if (!hasValidAccount()) {
      const returnTo =
        props.returnPath || `${location.pathname}${location.search}`
      modalsContext?.openModal(
        { name: "requiresGdlAccount" },
        { returnPath: returnTo }
      )
      return
    }
    props.onClick()
  }

  return (
    <Show
      when={hasValidAccount()}
      fallback={
        <Tooltip>
          <TooltipTrigger as="div" class="w-full">
            <ContextMenuItem
              class={`flex items-center gap-2 text-primary-500 ${props.class || ""}`}
              onClick={handleClick}
              disabled={props.disabled}
            >
              <Show when={props.icon}>{props.icon}</Show>
              {props.children}
              <img
                src={logoUrl}
                alt=""
                class="ml-auto h-3.5 w-3.5 opacity-80"
              />
            </ContextMenuItem>
          </TooltipTrigger>
          <TooltipContent>
            {t("accounts:_trn_requires_gdl_account")}
          </TooltipContent>
        </Tooltip>
      }
    >
      <ContextMenuItem
        class={`flex items-center gap-2 ${props.class || ""}`}
        onClick={handleClick}
        disabled={props.disabled}
      >
        <Show when={props.icon}>{props.icon}</Show>
        {props.children}
      </ContextMenuItem>
    </Show>
  )
}

export default GdlFeatureContextMenuItem
