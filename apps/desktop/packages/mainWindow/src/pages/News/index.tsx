import { onMount, createSignal } from "solid-js"
import { Tab, TabList, Tabs, TabPanel } from "@gd/ui"
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
    const lastPath = navigation.lastPathVisited().path
    return isNewsDetailPath(lastPath)
  }

  const shouldLoadSavedTab = isComingFromNewsDetail()
  const defaultTab = shouldLoadSavedTab ? newsContext.selectedTab() : 0
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

  const handleTabChange = (newTab: number) => {
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
    newsContext.setSelectedTab(selectedTab())
    newsContext.setIsNavigatingToDetail(true)
  }

  return (
    <div class="flex h-full flex-col">
      <div class="flex h-full flex-col">
        <Tabs
          orientation="horizontal"
          index={selectedTab()}
          onChange={handleTabChange}
        >
          <div class="bg-darkSlate-800 sticky top-0 z-50 shrink-0 px-6">
            <TabList>
              <Tab>
                <div class="flex items-center gap-2 py-3">
                  <div class="i-hugeicons:news-01 h-5 w-5" />
                  <Trans key="news:_trn_minecraft_news">News</Trans>
                </div>
              </Tab>
              <Tab>
                <div class="flex items-center gap-2 py-3">
                  <div class="i-hugeicons:note h-5 w-5" />
                  <Trans key="news:_trn_minecraft_patches">Patch Notes</Trans>
                </div>
              </Tab>
            </TabList>
          </div>

          <div ref={scrollContainer} class="flex-1 overflow-y-auto">
            <TabPanel>
              <NewsTab
                scrollContainer={scrollContainer}
                onNavigateToDetail={saveStateForNewsDetail}
              />
            </TabPanel>

            <TabPanel>
              <PatchesTab
                scrollContainer={scrollContainer}
                onNavigateToDetail={saveStateForNewsDetail}
              />
            </TabPanel>
          </div>
        </Tabs>
      </div>
    </div>
  )
}

export default NewsPage
