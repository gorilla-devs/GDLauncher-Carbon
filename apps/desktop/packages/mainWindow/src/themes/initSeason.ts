import { createEffect, onCleanup, onMount } from "solid-js"
import { useLocation } from "@solidjs/router"
import { toast } from "@gd/ui"
import { i18n } from "@gd/i18n"
import { getThemeKey } from "@gd/i18n/helpers"
import { rspc } from "@/utils/rspcClient"
import { useModal } from "@/managers/ModalsManager"
import { isBuiltinThemeId } from "./builtin"
import { betaPromptChecked } from "@/utils/betaPromptLatch"
import {
  betaPromptPending as computeBetaPending,
  gateAnswer,
  seasonDialogGate,
  shouldCheckOnFocus
} from "./season"

const HOUR_MS = 3_600_000
const SETTLE_MS = 1_000

// Wiring only (imports JSX modules, so unit tests never load it). The core
// decides the season; this asks it to check and shows what it reports.
const initSeason = () => {
  const modals = useModal()
  const location = useLocation()

  const season = rspc.createQuery(() => ({ queryKey: ["themes.getSeason"] }))
  const isFirstLaunch = rspc.createQuery(() => ({
    queryKey: ["settings.isFirstLaunch"]
  }))
  const shouldShowChangelog = rspc.createQuery(() => ({
    queryKey: ["settings.shouldShowChangelog"]
  }))
  const shouldShowBetaPrompt = rspc.createQuery(() => ({
    queryKey: ["settings.shouldShowBetaPrompt"]
  }))

  const checkSeason = rspc.createMutation(() => ({
    mutationKey: ["themes.checkSeason"]
  }))
  const ackNotice = rspc.createMutation(() => ({
    mutationKey: ["themes.ackSeasonNotice"]
  }))

  let lastCheck = 0
  const check = () => {
    lastCheck = Date.now()
    checkSeason.mutate(undefined)
  }

  onMount(() => {
    check()
    const onFocus = () => {
      if (shouldCheckOnFocus(lastCheck, Date.now())) check()
    }
    window.addEventListener("focus", onFocus)
    const timer = setInterval(check, HOUR_MS)
    onCleanup(() => {
      window.removeEventListener("focus", onFocus)
      clearInterval(timer)
    })
  })

  const notice = () => season.data?.notice ?? null

  // "ended": tell the user right away, then acknowledge.
  const toasted = new Set<string>()
  createEffect(() => {
    const n = notice()
    if (n?.kind !== "ended" || toasted.has(n.themeId)) return
    toasted.add(n.themeId)
    const name = isBuiltinThemeId(n.themeId)
      ? i18n.t(getThemeKey(n.themeId))
      : n.themeId
    toast(
      i18n.t("settings:_trn_season_ended", {
        name,
        interpolation: { escapeValue: false }
      })
    )
    ackNotice.mutate(undefined)
  })

  // The beta prompt blocks only until app.tsx has opened it; after that it is
  // a modal and `modalOpen` covers it.
  const betaPromptPending = () =>
    computeBetaPending(gateAnswer(shouldShowBetaPrompt), betaPromptChecked())

  // "selected": open the dialog once the launcher has been calm for a moment.
  const opened = new Set<string>()
  createEffect(() => {
    const n = notice()
    if (n?.kind !== "selected") return
    const key = `${n.themeId}:${n.endsOn}`
    if (opened.has(key)) return
    const ok = seasonDialogGate({
      route: location.pathname,
      firstLaunch: gateAnswer(isFirstLaunch),
      changelog: gateAnswer(shouldShowChangelog),
      betaPromptPending: betaPromptPending(),
      modalOpen: modals?.hasOpenModals() ?? true
    })
    if (!ok) return
    const timer = setTimeout(() => {
      opened.add(key)
      modals?.openModal(
        { name: "seasonalTheme" },
        { themeId: n.themeId, endsOn: n.endsOn }
      )
    }, SETTLE_MS)
    onCleanup(() => clearTimeout(timer))
  })
}

export default initSeason
