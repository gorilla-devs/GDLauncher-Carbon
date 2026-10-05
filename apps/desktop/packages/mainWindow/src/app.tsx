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
import { acceptProtocolUrl } from "./utils/protocolAction"
import { checkForUpdates } from "./utils/updater"
import { windowCloseWarningAcquireLock } from "./managers/ModalsManager/modals/WindowCloseWarning"
import { ACCOUNT_BANNED_EVENT } from "./utils/bannedEventBridge"
import {
  showRuntimeMoveNotice,
  showUnavailableRuntimePath
} from "./utils/runtimeMoveNotice"
import ShutdownPrompt from "./components/ShutdownPrompt"

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

  // While subscribed, main holds a close with a running game open for this
  // warning; unsubscribing lets the close go ahead instead.
  const stopCloseWarning = window.onShowWindowCloseModal(() => {
    if (windowCloseWarningAcquireLock) {
      modalsContext?.openModal({
        name: "windowCloseWarning"
      })
    }
  })
  onCleanup(() => stopCloseWarning?.())

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
    const unsubscribe = window.onProtocolUrl?.((url) =>
      acceptProtocolUrl(url, {
        share: (shareCode) => {
          modalsContext?.openModal({ name: "sharePreview" }, { shareCode })
        },
        navigate: (route) => navigate.navigate(route)
      })
    )
    onCleanup(() => unsubscribe?.())
  })

  // Every start from the new data folder after a move names what stayed in the old
  // one, until the user acknowledges it.
  onMount(() => {
    void showRuntimeMoveNotice((notice) =>
      modalsContext?.openModal({ name: "runtimeMoveNotice" }, notice)
    )
  })

  // A start that could not open the data folder (its drive not connected, say)
  // uses the default one and says so, until the folder can be opened again.
  onMount(() => {
    void showUnavailableRuntimePath((value) =>
      modalsContext?.openModal({ name: "runtimePathUnavailable" }, value)
    )
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
      {/* While mounted, main asks here when a quit, restart or update does
          not finish, instead of in a native dialog. */}
      <ShutdownPrompt />
    </div>
  )
}

export default App
