import {
  createEffect,
  createSignal,
  JSX,
  onCleanup,
  onMount,
  untrack
} from "solid-js"
import { useLocation } from "@solidjs/router"
import initThemes from "./utils/theme"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "./managers/ModalsManager"
import { useGDNavigate } from "./managers/NavigationManager"
import { parseSearchQuery } from "./utils/searchQueryParser"
import { checkForUpdates } from "./utils/updater"
import { windowCloseWarningAcquireLock } from "./managers/ModalsManager/modals/WindowCloseWarning"
import { ACCOUNT_BANNED_EVENT } from "./utils/bannedEventBridge"

interface Props {
  createInvalidateQuery: () => void
  children?: JSX.Element
}

const App = (props: Props) => {
  const [runItOnce, setRunItOnce] = createSignal(true)
  const modalsContext = useModal()
  const navigate = useGDNavigate()
  const currentRoute = useLocation()

  props.createInvalidateQuery()

  window.onShowWindowCloseModal(() => {
    if (windowCloseWarningAcquireLock) {
      modalsContext?.openModal({
        name: "windowCloseWarning"
      })
    }
  })

  initThemes()

  checkForUpdates()

  // Listen for account banned events from rspcClient
  onMount(() => {
    const handleBanned = () => {
      modalsContext?.openModal({ name: "accountBanned" })
    }
    window.addEventListener(ACCOUNT_BANNED_EVENT, handleBanned)
    onCleanup(() =>
      window.removeEventListener(ACCOUNT_BANNED_EVENT, handleBanned)
    )
  })

  // Handle protocol URLs (gdlauncher://, curseforge://, modrinth://).
  // Must live inside ModalProvider so gdlauncher://share/* can open the
  // sharePreview modal — NavigationManager is above the modal provider and
  // useModal() returns undefined there.
  onMount(() => {
    window.onProtocolUrl?.((url) => {
      const parsed = parseSearchQuery(url)
      if (parsed.mode !== "direct" || parsed.items.length === 0) return

      const firstItem = parsed.items[0]
      if (firstItem.type === "gdlauncher_share") {
        modalsContext?.openModal(
          { name: "sharePreview" },
          { shareCode: firstItem.shareCode }
        )
      } else {
        navigate.navigate(`/search?q=${encodeURIComponent(url)}`)
      }
    })
  })

  // First launch detection using semantic query
  const isFirstLaunch = rspc.createQuery(() => ({
    queryKey: ["settings.isFirstLaunch"]
  }))

  const completeFirstLaunch = rspc.createMutation(() => ({
    mutationKey: ["settings.completeFirstLaunch"]
  }))

  createEffect(() => {
    if (
      isFirstLaunch.data === true &&
      currentRoute.pathname !== "/" &&
      runItOnce()
    ) {
      untrack(() => {
        modalsContext?.openModal({ name: "onBoarding" })
        completeFirstLaunch.mutate(undefined)
      })
      setRunItOnce(false)
    }
  })

  // Beta prompt for stable channel users
  const [betaPromptChecked, setBetaPromptChecked] = createSignal(false)
  const shouldShowBetaPrompt = rspc.createQuery(() => ({
    queryKey: ["settings.shouldShowBetaPrompt"]
  }))

  createEffect(() => {
    // Only show if not first run, not already checked, and API says we should
    // Note: shouldShowBetaPrompt already checks first launch status internally
    if (!betaPromptChecked() && shouldShowBetaPrompt.data === true) {
      setBetaPromptChecked(true)
      untrack(() => {
        modalsContext?.openModal({ name: "betaPrompt" })
      })
    }
  })

  return (
    <div class="relative w-screen">
      <div class="z-10 flex h-auto w-screen">
        <main class="max-w-screen relative grow">{props.children}</main>
      </div>
    </div>
  )
}

export default App
