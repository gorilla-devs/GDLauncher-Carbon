import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  isAddonPath,
  isSearchPath,
  isNewsPath,
  isNewsDetailPath
} from "@/utils/routes"
import { parseSearchQuery } from "@/utils/searchQueryParser"
import { useLocation, useNavigate } from "@solidjs/router"
import { JSX, createContext, createSignal, onMount, useContext } from "solid-js"

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

  // Handle protocol URLs (curseforge://, modrinth://)
  onMount(() => {
    window.onProtocolUrl?.((url) => {
      console.log("Protocol URL received in renderer:", url)
      const parsed = parseSearchQuery(url)
      if (parsed.mode === "direct" && parsed.items.length > 0) {
        // Navigate to search page with the URL pre-filled
        // The search page will parse this and show the direct results
        navigate(`/search?q=${encodeURIComponent(url)}`)
      }
    })
  })

  const shouldTransition = () =>
    !globalstore.settings.data?.reducedMotion && document.startViewTransition

  const gdNavigate = (destinationPath: string, options?: NavigateOptions) => {
    const currentPath = location.pathname

    const transitionClass = getTransitionClassToApply(
      currentPath,
      destinationPath
    )

    if (shouldTransition()) {
      if (transitionClass) {
        document.body.classList.add(transitionClass)
      }
      const result = document.startViewTransition(() => {
        navigate(destinationPath, { replace: options?.replace })
      })

      result.finished.then(() => {
        if (transitionClass) {
          document.body.classList.remove(transitionClass)
        }
      })
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
  return useContext(NavigationContext)!
}
