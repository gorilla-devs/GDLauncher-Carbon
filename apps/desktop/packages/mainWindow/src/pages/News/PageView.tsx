import { createResource, For } from "solid-js"
import { initNews } from "@/utils/news"

const PageView = () => {
  const [news] = createResource(initNews)

  return (
    <div class="flex flex-col gap-4 p-4">
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
                <p class="text-sm text-lightSlate-400">{item.description}</p>
              </div>
            </div>
          )}
        </For>
      </div>
    </div>
  )
}

export default PageView
