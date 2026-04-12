import { Button, Tabs, TabsList, TabsTrigger, TabsIndicator } from "@gd/ui"
import { useLocation } from "@solidjs/router"
import { For, JSX, Show, createSignal, onMount, onCleanup } from "solid-js"

export interface DetailPageTab {
  id: string
  label: JSX.Element
}

interface DetailPageLayoutProps {
  children: JSX.Element
  containerId: string
  headerImage: string
  icon: string
  headerInfoContent: JSX.Element
  headerActions: JSX.Element
  headerTopRight?: JSX.Element
  tabs: DetailPageTab[]
  activeTabId: string
  onTabClick: (tab: DetailPageTab) => void
  onBackClick: () => void
  stickyRightButton: JSX.Element
  noPaddingPaths: string[]
  isFullScreen?: () => boolean
}

const DetailPageLayout = (props: DetailPageLayoutProps) => {
  const location = useLocation()
  const [isSticky, setIsSticky] = createSignal(false)
  const [scrollTop, setScrollTop] = createSignal(0)

  let headerRef!: HTMLElement
  let refStickyTabs!: HTMLDivElement

  const handleScroll = () => {
    if (!headerRef?.parentElement) return
    requestAnimationFrame(() => {
      setScrollTop(headerRef.parentElement?.scrollTop || 0)
      const rect = refStickyTabs.getBoundingClientRect()
      setIsSticky(rect.top <= 104)
    })
  }

  onMount(() => {
    headerRef.parentElement?.addEventListener("scroll", handleScroll)
  })

  onCleanup(() => {
    headerRef.parentElement?.removeEventListener("scroll", handleScroll)
  })

  const isNoPaddingPath = () =>
    props.noPaddingPaths.some((p) => location.pathname.includes(p))

  return (
    <main
      id={props.containerId}
      class="bg-darkSlate-800 relative flex h-full flex-col overflow-x-hidden"
      classList={{
        "overflow-hidden": props.isFullScreen?.(),
        "overflow-y-auto overflow-x-hidden": !props.isFullScreen?.()
      }}
      style={{
        "scrollbar-gutter": "stable"
      }}
    >
      <header
        ref={(el) => {
          headerRef = el
        }}
        class="transition-100 ease-spring relative flex flex-col items-stretch justify-between overflow-hidden transition-all"
        classList={{
          "min-h-0 h-0": props.isFullScreen?.(),
          "min-h-60": !props.isFullScreen?.()
        }}
      >
        <img
          src={props.headerImage}
          alt="Cover"
          class="absolute h-full w-full object-cover"
          style={{
            transform: `translate3d(0, ${scrollTop() * 0.4}px, 0)`,
            "will-change": "transform"
          }}
        />
        <div class="from-darkSlate-800 relative z-10 h-full bg-gradient-to-t">
          <div class="sticky left-5 top-5 z-50 w-fit">
            <Button
              rounded
              onClick={props.onBackClick}
              size="small"
              type="transparent"
            >
              <div class="i-hugeicons:arrow-left-01 text-xl" />
            </Button>
          </div>
          <Show when={props.headerTopRight}>
            <div class="absolute right-5 top-5 z-50 flex w-fit gap-2">
              {props.headerTopRight}
            </div>
          </Show>
          <div class="from-darkSlate-800 sticky top-52 z-20 box-border flex h-24 w-full justify-center bg-gradient-to-t px-6 pb-2">
            <div class="flex w-full justify-start">
              <div class="flex w-full items-end justify-between">
                <div class="flex flex-1 flex-row justify-end gap-4">
                  <img
                    src={props.icon}
                    alt="Icon"
                    class="h-16 w-16 rounded-xl object-cover"
                  />
                  <div class="flex flex-1 flex-col">
                    {props.headerInfoContent}
                    <div class="flex cursor-default flex-row justify-between">
                      <div class="flex-1" />
                      <div class="flex items-center gap-2">
                        {props.headerActions}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </header>

      <div
        class="bg-darkSlate-800"
        classList={{
          "flex-1 flex flex-col min-h-0": props.isFullScreen?.(),
          sticky: !props.isFullScreen?.()
        }}
      >
        <div
          class="flex justify-center py-0"
          classList={{
            "px-6": !isNoPaddingPath(),
            "flex-1 min-h-0": props.isFullScreen?.()
          }}
        >
          <div
            class="bg-darkSlate-800 w-full"
            classList={{
              "flex flex-col min-h-0": props.isFullScreen?.()
            }}
          >
            <div
              class="bg-darkSlate-800 sticky top-0 z-30 flex h-14 items-center justify-between py-10"
              classList={{
                "px-6": isNoPaddingPath()
              }}
              ref={(el) => {
                refStickyTabs = el
              }}
            >
              <div class="flex h-full items-center">
                <div
                  class="ease-spring flex items-center overflow-hidden transition-all duration-150"
                  classList={{
                    "w-14 mr-4 opacity-100":
                      isSticky() || props.isFullScreen?.(),
                    "w-0 mr-0 opacity-0": !isSticky() && !props.isFullScreen?.()
                  }}
                >
                  <Button
                    onClick={props.onBackClick}
                    size="small"
                    type="secondary"
                  >
                    <div class="i-hugeicons:arrow-left-01 text-xl" />
                  </Button>
                </div>
                <div class="flex items-center">
                  <Tabs value={props.activeTabId} class="h-auto">
                    <TabsList class="w-fit gap-0">
                      <TabsIndicator />
                      <For each={props.tabs}>
                        {(tab) => (
                          <TabsTrigger
                            value={tab.id}
                            onClick={() => props.onTabClick(tab)}
                          >
                            {tab.label}
                          </TabsTrigger>
                        )}
                      </For>
                    </TabsList>
                  </Tabs>
                </div>
              </div>
              <div
                class="ease-spring flex items-center justify-end overflow-hidden transition-all duration-150"
                classList={{
                  "w-14 ml-4 opacity-100": isSticky() || props.isFullScreen?.(),
                  "w-0 ml-0 opacity-0": !isSticky() && !props.isFullScreen?.()
                }}
              >
                {props.stickyRightButton}
              </div>
            </div>
            <div
              class="px-0"
              classList={{
                "flex-1 min-h-0 pt-4 px-4": !!props.isFullScreen?.(),
                "pt-0": !props.isFullScreen?.() && isNoPaddingPath(),
                "pt-4 px-4": !props.isFullScreen?.() && !isNoPaddingPath()
              }}
            >
              {props.children}
            </div>
          </div>
        </div>
      </div>
    </main>
  )
}

export default DetailPageLayout
