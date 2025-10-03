import { createContext, useContext, createSignal, JSX } from "solid-js"

interface NewsContextType {
  selectedTab: () => number
  setSelectedTab: (tab: number) => void
  scrollPosition: () => number
  setScrollPosition: (position: number) => void
  isNavigatingToDetail: () => boolean
  setIsNavigatingToDetail: (navigating: boolean) => void
}

const NewsContext = createContext<NewsContextType>()

export const NewsProvider = (props: { children: JSX.Element }) => {
  const [selectedTab, setSelectedTab] = createSignal(0)
  const [scrollPosition, setScrollPosition] = createSignal(0)
  const [isNavigatingToDetail, setIsNavigatingToDetail] = createSignal(false)

  const contextValue: NewsContextType = {
    selectedTab,
    setSelectedTab,
    scrollPosition,
    setScrollPosition,
    isNavigatingToDetail,
    setIsNavigatingToDetail
  }

  return (
    <NewsContext.Provider value={contextValue}>
      {props.children}
    </NewsContext.Provider>
  )
}

export const useNewsContext = () => {
  const context = useContext(NewsContext)
  if (!context) {
    throw new Error("useNewsContext must be used within a NewsProvider")
  }
  return context
}
