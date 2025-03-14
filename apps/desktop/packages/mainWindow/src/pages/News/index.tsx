import { createResource, For, Match, Switch } from "solid-js"
import { initNews } from "@/utils/news"
import { News, Skeleton } from "@gd/ui"

const NewsPage = () => {
  const newsInitializer = initNews()

  const [news] = createResource(() => newsInitializer)
  return (
    <div>
      <Switch>
        <Match when={(news()?.length || 0) > 0}>
          <News
            slides={news()}
            disableAutoRotation
            onClick={(news) => {
              window.openExternalLink(news.url || "")
            }}
          />
        </Match>
        <Match when={news.length === 0}>
          <Skeleton.news />
        </Match>
      </Switch>
      <div class="flex flex-col gap-4 p-6">
        <h1 class="text-2xl font-medium">Minecraft News</h1>
        <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          <For each={news()}>
            {(item) => (
              <div
                onClick={() => window.open(item.url, "_blank")}
                class="cursor-pointer transition-transform hover:scale-[1.02]"
              >
                <img
                  src={item.image}
                  alt={item.title}
                  class="aspect-video w-full object-cover"
                />
                <div class="p-4">
                  <h2 class="mb-2 text-lg font-medium">{item.title}</h2>
                  <p class="text-lightSlate-400 text-sm">{item.description}</p>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  )
}

export default NewsPage
