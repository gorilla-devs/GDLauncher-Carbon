import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  isAddonPath,
  isSearchPath,
  isNewsPath,
  isNewsDetailPath
} from "@/utils/routes"
import { useLocation, useNavigate } from "@solidjs/router"
import { JSX, createContext, createSignal, useContext } from "solid-js"
import { createPageSlides } from "./pageSlide"

const getTransitionClassToApply = (from: string, to: string) => {
  if (isSearchPath(from) && isAddonPath(to)) {
    return "slide-right-transition"
  } else if (isAddonPath(from) && isSearchPath(to)) {
    return "slide-left-transition"
  } else if (
    isNewsPath(from) &&
    !isNewsDetailPath(from) &&
    isNewsDetailPath(to)
  ) {
    return "slide-right-transition"
  } else if (
    isNewsDetailPath(from) &&
    isNewsPath(to) &&
    !isNewsDetailPath(to)
  ) {
    return "slide-left-transition"
  }
}

/** The layout's box around the current page (withAds). */
export const PAGE_ID = "gdl-page"

interface NavigateOptions {
  replace?: boolean
}

interface NavigationContext {
  navigate: (_path: string, _options?: NavigateOptions) => void
  prev: () => void
  lastPathVisited: () => { path: string; searchParams: string }
}

const NavigationContext = createContext<NavigationContext>()

export const NavigationManager = (props: { children: JSX.Element }) => {
  const navigate = useNavigate()
  const location = useLocation()
  const globalstore = useGlobalStore()
  const [lastPathVisited, setLastPathVisited] = createSignal({
    path: location.pathname,
    searchParams: location.search
  })

  const startSlide = createPageSlides(document, PAGE_ID)

  const shouldTransition = () =>
    !globalstore.settings.data?.reducedMotion && document.startViewTransition

  const gdNavigate = (destinationPath: string, options?: NavigateOptions) => {
    const currentPath = location.pathname

    const transitionClass = getTransitionClassToApply(
      currentPath,
      destinationPath
    )

    if (shouldTransition()) {
      // Only the page slides: the bars, the theme's background and its
      // effects stay put (pageSlide.ts).
      const endSlide = startSlide(transitionClass)
      const result = document.startViewTransition(() => {
        navigate(destinationPath, { replace: options?.replace })
      })
      void result.finished.finally(endSlide)
    } else {
      navigate(destinationPath, { replace: options?.replace })
    }

    if (!options?.replace) {
      setLastPathVisited({
        path: currentPath,
        searchParams: location.search
      })
    }
  }

  const prev = () => {
    gdNavigate(`${lastPathVisited().path}${lastPathVisited().searchParams}`)
  }

  const navigationCtx = {
    navigate: gdNavigate,
    prev,
    lastPathVisited
  }

  return (
    <NavigationContext.Provider value={navigationCtx}>
      {props.children}
    </NavigationContext.Provider>
  )
}

export const useGDNavigate = (): NavigationContext => {
  const ctx = useContext(NavigationContext)
  if (ctx) return ctx

  // Fallback so call sites never crash on `navigator.navigate(...)` if the
  // provider is briefly missing (HMR, stale tree during a route transition,
  // etc.). Router context is still present — it wraps the provider — so
  // plain solid-router navigate works. The transition polish and
  // last-path tracking are skipped in this path.
  console.warn(
    "useGDNavigate called without NavigationManager provider; using fallback"
  )
  const navigate = useNavigate()
  return {
    navigate: (path, options) => navigate(path, { replace: options?.replace }),
    prev: () => navigate(-1),
    lastPathVisited: () => ({ path: "", searchParams: "" })
  }
}
