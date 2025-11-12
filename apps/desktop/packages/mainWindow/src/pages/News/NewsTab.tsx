import { For, Match, Switch } from "solid-js"
import { useNews } from "@/utils/news"
import { News, Skeleton } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"

interface NewsTabProps {
  scrollContainer?: HTMLDivElement
  onNavigateToDetail?: () => void
}

const NewsTab = (props: NewsTabProps) => {
  const navigator = useGDNavigate()
  const news = useNews()

  return (
    <>
      <Switch>
        <Match when={(news.data?.length || 0) > 0}>
          <News
            slides={news.data || []}
            disableAutoRotation
            onClick={(newsItem: any) => {
              props.onNavigateToDetail?.()
              navigator.navigate(`/news/${newsItem.id}`)
            }}
          />
        </Match>
        <Match when={news.isPending || (news.data?.length || 0) === 0}>
          <Skeleton.newsCarousel />
        </Match>
      </Switch>
      <Switch>
        <Match when={!news.isPending && (news.data?.length || 0) > 0}>
          <div class="flex flex-col gap-4 p-6">
            <h1 class="text-2xl font-medium">
              <Trans key="news:_trn_minecraft_news" />
            </h1>
            <div class="news-grid grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              <For each={news.data}>
                {(item) => (
                  <div
                    onClick={() => {
                      props.onNavigateToDetail?.()
                      navigator.navigate(`/news/${item.id}`)
                    }}
                    class="cursor-pointer transition-transform hover:scale-[1.02]"
                  >
                    <img
                      src={item.image}
                      alt={item.title}
                      class="aspect-video w-full object-cover"
                    />
                    <div class="p-4">
                      <h2 class="mb-2 text-lg font-medium">{item.title}</h2>
                      <p class="text-lightSlate-400 mb-2 text-sm">
                        {item.description}
                      </p>
                      <p class="text-lightSlate-500 text-xs">
                        {new Date(item.date).toLocaleDateString(undefined, {
                          year: "numeric",
                          month: "short",
                          day: "numeric"
                        })}
                      </p>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Match>
        <Match when={news.isPending}>
          <Skeleton.news />
        </Match>
      </Switch>
    </>
  )
}

export default NewsTab
