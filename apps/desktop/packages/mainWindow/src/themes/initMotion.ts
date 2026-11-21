import { createComputed, createSignal, onCleanup } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { isGameRunning, motionState, setMotion } from "./motion"

// Wiring only (imports JSX modules, so unit tests never load it): feeds the
// pure motionState from the app setting, the OS preference, the window's
// visibility and running games, and publishes html[data-gdl-motion].
const initMotion = () => {
  const store = useGlobalStore()

  const query = window.matchMedia("(prefers-reduced-motion: reduce)")
  // Every input is read before the first state is set, so the first value is
  // never a blind "run".
  const [os, setOs] = createSignal(query.matches)
  const [domVisible, setDomVisible] = createSignal(
    document.visibilityState === "visible"
  )
  const [mainVisible, setMainVisible] = createSignal(true)

  const onOs = (e: MediaQueryListEvent) => setOs(e.matches)
  const onVisibility = () =>
    setDomVisible(document.visibilityState === "visible")
  query.addEventListener("change", onOs)
  document.addEventListener("visibilitychange", onVisibility)
  const stopMain = window.themeHost.onWindowVisibility(setMainVisible)

  onCleanup(() => {
    query.removeEventListener("change", onOs)
    document.removeEventListener("visibilitychange", onVisibility)
    stopMain()
  })

  // createComputed runs now, not after the first render. Until the settings
  // are loaded the app setting is unknown, so treat it as reduced (the calm
  // choice) rather than start animating.
  createComputed(() => {
    const settings = store.settings.data
    const state = motionState({
      reducedMotion: settings ? settings.reducedMotion : true,
      osReducedMotion: os(),
      visible: domVisible() && mainVisible(),
      gameRunning: isGameRunning(store.instances.data)
    })
    setMotion(state)
    document.documentElement.dataset.gdlMotion = state
  })
}

export default initMotion
