import { For, Show, Suspense } from "solid-js"
import { Announcement } from "@gd/core_module/bindings"
import { marked } from "marked"
import sanitizeHtml from "sanitize-html"
import { useGlobalStore } from "./GlobalStoreContext"
const Announcements = () => {
  const globalStore = useGlobalStore()

  const htmlTitle = (title: string) => {
    return marked.parse(title, {
      async: false
    })
  }

  const htmlContent = (content: string) => {
    return marked.parse(content, {
      async: false
    })
  }

  const getBackgroundColor = (type: Announcement["type"]) => {
    switch (type) {
      case "error":
        return "bg-red-500"
      case "warning":
        return "bg-yellow-500"
      default:
        return "bg-primary-500"
    }
  }

  const opts: sanitizeHtml.IOptions = {
    allowedAttributes: {
      ...sanitizeHtml.defaults.allowedAttributes,
      a: ["href", "name", "target", "class"],
      div: ["class"]
    }
  }

  return (
    <Suspense>
      <Show when={globalStore.announcements?.data?.length || 0 > 0}>
        <For each={globalStore.announcements?.data}>
          {(announcement) => (
            <div class="mb-8 box-border flex w-full flex-col">
              <div
                class={`box-border flex h-10 w-full items-center justify-center rounded-t-xl font-bold ${getBackgroundColor(announcement.type)}`}
                // eslint-disable-next-line solid/no-innerhtml
                innerHTML={sanitizeHtml(htmlTitle(announcement.title), opts)}
              />
              <div
                class="border-1 border-darkSlate-600 border-x-solid border-b-solid box-border w-full flex-wrap rounded-b-xl p-4"
                // eslint-disable-next-line solid/no-innerhtml
                innerHTML={sanitizeHtml(
                  htmlContent(announcement.content),
                  opts
                )}
              />
            </div>
          )}
        </For>
      </Show>
    </Suspense>
  )
}

export default Announcements
