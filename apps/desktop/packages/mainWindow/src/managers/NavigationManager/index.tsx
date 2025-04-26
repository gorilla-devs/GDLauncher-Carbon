import { useGlobalStore } from "@/components/GlobalStoreContext"
import { isAddonPath, isSearchPath } from "@/utils/routes"
import { useLocation, useNavigate } from "@solidjs/router"
import { JSX, createContext, createSignal, useContext } from "solid-js"

const getTransitionClassToApply = (from: string, to: string) => {
  if (isSearchPath(from) && isAddonPath(to)) {
    return "slide-right-transition"
  } else if (isAddonPath(from) && isSearchPath(to)) {
    return "slide-left-transition"
  }
}

interface NavigateOptions {
  replace?: boolean
}

interface NavigationContext {
  navigate: (_path: string, _options?: NavigateOptions) => void
  prev: () => void
}

const NavigationContext = createContext<NavigationContext>()

export const NavigationManager = (props: { children: JSX.Element }) => {
  const navigate = useNavigate()
  const location = useLocation()
  const globalstore = useGlobalStore()
  const [lastPathVisited, setLastPathVisited] = createSignal(location.pathname)

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
      setLastPathVisited(currentPath)
    }
  }

  const prev = () => {
    gdNavigate(lastPathVisited())
  }

  const navigationCtx = {
    navigate: gdNavigate,
    prev
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
