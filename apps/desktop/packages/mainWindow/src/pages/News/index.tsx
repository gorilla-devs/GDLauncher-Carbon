import { onMount, createSignal } from "solid-js"
import { Tabs, TabsList, TabsTrigger, TabsContent, TabsIndicator } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { isNewsDetailPath } from "@/utils/routes"
import { useNewsContext } from "@/components/NewsContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import NewsTab from "./NewsTab"
import PatchesTab from "./PatchesTab"

const NewsPage = () => {
  let scrollContainer: HTMLDivElement | undefined
  const newsContext = useNewsContext()
  const navigation = useGDNavigate()

  const isComingFromNewsDetail = () => {
    const lastPathInfo = navigation?.lastPathVisited?.()
    if (!lastPathInfo) return false
    return isNewsDetailPath(lastPathInfo.path)
  }

  const shouldLoadSavedTab = isComingFromNewsDetail()
  const defaultTab = shouldLoadSavedTab
    ? newsContext.selectedTab() === 0
      ? "news"
      : "patches"
    : "news"
  const [selectedTab, setSelectedTab] = createSignal(defaultTab)

  onMount(() => {
    if (scrollContainer && shouldLoadSavedTab) {
      setTimeout(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = newsContext.scrollPosition()
        }
      }, 50)
    }
  })

  const handleTabChange = (newTab: string) => {
    setSelectedTab(newTab)
    // Reset scroll position to top when switching tabs
    if (scrollContainer) {
      scrollContainer.scrollTop = 0
    }
  }

  const saveStateForNewsDetail = () => {
    if (scrollContainer) {
      newsContext.setScrollPosition(scrollContainer.scrollTop)
    }
    newsContext.setSelectedTab(selectedTab() === "news" ? 0 : 1)
    newsContext.setIsNavigatingToDetail(true)
  }

  return (
    <div class="flex h-full flex-col">
      <div class="flex h-full flex-col">
        <Tabs value={selectedTab()} onChange={handleTabChange}>
          <div class="bg-darkSlate-800 sticky top-0 z-50 shrink-0 px-6 py-4">
            <TabsList class="w-fit gap-0">
              <TabsIndicator />
              <TabsTrigger value="news">
                <div class="flex items-center gap-2 py-1">
                  <div class="i-hugeicons:news-01 h-5 w-5" />
                  <Trans key="news:_trn_minecraft_news">News</Trans>
                </div>
              </TabsTrigger>
              <TabsTrigger value="patches">
                <div class="flex items-center gap-2 py-1">
                  <div class="i-hugeicons:note h-5 w-5" />
                  <Trans key="news:_trn_minecraft_patches">Patch Notes</Trans>
                </div>
              </TabsTrigger>
            </TabsList>
          </div>

          <div ref={scrollContainer} class="flex-1 overflow-y-auto">
            <TabsContent value="news">
              <NewsTab
                scrollContainer={scrollContainer}
                onNavigateToDetail={saveStateForNewsDetail}
              />
            </TabsContent>

            <TabsContent value="patches">
              <PatchesTab
                scrollContainer={scrollContainer}
                onNavigateToDetail={saveStateForNewsDetail}
              />
            </TabsContent>
          </div>
        </Tabs>
      </div>
    </div>
  )
}

export default NewsPage
