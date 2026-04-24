import { JSX, Show, splitProps } from "solid-js"
import { useTransContext, NamespacedTranslationKey } from "@gd/i18n"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { useGlobalStore } from "./GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import { logoUrl } from "@/utils/logos"
import { useLocation } from "@solidjs/router"

interface Props {
  children: JSX.Element
  tooltipKey?: NamespacedTranslationKey
  showIndicator?: boolean
  returnPath?: string
  onUnauthenticatedClick?: () => void
}

/**
 * Wrapper component that indicates a feature requires a GDL account.
 * When no valid GDL account exists:
 * - Applies primary color to children
 * - Shows GDL logo badge
 * - Wraps in tooltip explaining requirement
 * - Opens modal explaining GDL account requirement on click
 *
 * When valid GDL account exists, renders children normally.
 */
const RequiresGdlAccount = (props: Props) => {
  const [local, others] = splitProps(props, [
    "children",
    "tooltipKey",
    "showIndicator",
    "returnPath",
    "onUnauthenticatedClick"
  ])

  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  const location = useLocation()
  const [t] = useTransContext()

  const hasValidAccount = () => globalStore.gdlAccount.data?.status === "valid"

  const showBadge = () => local.showIndicator !== false

  const handleClick = (e: MouseEvent) => {
    if (hasValidAccount()) return

    e.preventDefault()
    e.stopPropagation()

    if (local.onUnauthenticatedClick) {
      local.onUnauthenticatedClick()
    } else {
      const returnTo =
        local.returnPath || `${location.pathname}${location.search}`
      modalsContext?.openModal(
        { name: "requiresGdlAccount" },
        { returnPath: returnTo }
      )
    }
  }

  return (
    <Show
      when={hasValidAccount()}
      fallback={
        <Tooltip>
          <TooltipTrigger
            as="div"
            class="flex cursor-pointer items-center"
            onClick={handleClick}
            {...others}
          >
            <div class="text-primary-500 flex items-center">
              {local.children}
              <Show when={showBadge()}>
                <img src={logoUrl} alt="" class="ml-1 h-3.5 w-3.5" />
              </Show>
            </div>
          </TooltipTrigger>
          <TooltipContent>
            {t(local.tooltipKey || "accounts:_trn_requires_gdl_account")}
          </TooltipContent>
        </Tooltip>
      }
    >
      {local.children}
    </Show>
  )
}

export default RequiresGdlAccount
