import { ModRowProps } from "@/utils/mods"
import { formatDownloadCount } from "@/utils/helpers"
import { Trans } from "@gd/i18n"
import { Badge } from "@gd/ui"
import { formatDistanceToNowStrict } from "date-fns"
import { For, Match, Show, Switch, createMemo } from "solid-js"
import { useGlobalStore } from "../GlobalStoreContext"

const Authors = (props: { data: ModRowProps }) => {
  return (
    <div class="scrollbar-hide flex max-w-full flex-wrap gap-2">
      <For each={props.data.data.authors}>
        {(author) => (
          <>
            <Badge class="bg-darkSlate-600 flex items-center gap-2">
              <Switch>
                <Match when={author.avatarUrl}>
                  <img src={author.avatarUrl!} class="h-3 w-3 rounded-full" />
                </Match>
                <Match when={!author.avatarUrl}>
                  <div class="i-hugeicons:user h-3 w-3 text-lightSlate-100 shrink-0" />
                </Match>
              </Switch>
              {author.name}
            </Badge>
          </>
        )}
      </For>
    </div>
  )
}

const OverviewPopover = (props: { data: ModRowProps }) => {
  const globalStore = useGlobalStore()

  const categories = createMemo(() => {
    const cats = props.data.data.categories

    if (props.data.data.platform === "curseforge") {
      return cats.map(
        (cat) => globalStore.categories.data?.curseforge[cat as number]
      )
    }

    return cats.map(
      (cat) => globalStore.categories.data?.modrinth[cat.toString()]
    )
  })

  const hasScreenshots = props.data.data.screenshotUrls.length > 0

  return (
    <div
      class="bg-darkSlate-900 relative overflow-hidden pb-4"
      classList={{
        "w-120": hasScreenshots,
        "w-80": !hasScreenshots
      }}
      onMouseDown={(e) => {
        e.stopPropagation()
        e.preventDefault()
      }}
    >
      <div
        class="max-h-140 relative flex flex-col overflow-hidden"
        style={{ width: hasScreenshots ? "calc(100% - 176px)" : "100%" }}
      >
        <Show when={props.data.data.websiteUrl}>
          <div
            class="h-6 w-6 cursor-pointer rounded-lg"
            onClick={() => {
              const url = props.data.data.websiteUrl
              if (url) window.openExternalLink(url)
            }}
          >
            <div class="i-hugeicons:link-square-02 h-4 w-4 text-lightSlate-500 hover:text-lightSlate-50 transition-color transition-100 absolute right-4 top-4 z-30 ease-in-out shrink-0" />
          </div>
        </Show>
        <h4 class="text-lightSlate-100 z-30 mb-2 w-fit px-4 text-xl">
          {props.data.data.title}
        </h4>
        <div class="from-darkSlate-900 absolute bottom-0 left-0 right-0 top-0 z-20 bg-gradient-to-t from-70%" />
        <div class="from-darkSlate-900 absolute bottom-0 left-0 right-0 top-0 z-20 bg-gradient-to-l" />
        <Show when={props.data.data.imageUrl}>
          <img
            class="absolute bottom-0 right-0 top-0 z-10 h-full w-full select-none blur-sm"
            src={props.data.data.imageUrl || undefined}
          />
        </Show>
        <div class="z-30 w-full flex-1 px-4">
          <p class="text-lightSlate-700 m-0 overflow-hidden text-ellipsis text-sm">
            {props.data.data.description}
          </p>
          <div class="scrollbar-hide mt-4 flex flex-wrap gap-2">
            <For each={categories().filter((cat) => cat !== undefined)}>
              {(category) => (
                <Badge class="bg-darkSlate-600 flex items-center gap-2">
                  <span>{category?.name || ""}</span>
                </Badge>
              )}
            </For>
          </div>
          <div class="mt-4 flex w-full flex-1 flex-col items-start gap-2">
            <div class="text-lightSlate-700 flex items-start gap-2">
              <span class="flex items-center gap-2">
                <div class="i-hugeicons:user h-4 w-4 text-lightSlate-100 shrink-0" />
                <p class="text-lightSlate-100 m-0 text-sm">
                  <Trans key="modpack.authors" />
                </p>
              </span>
              <Authors data={props.data} />
            </div>

            <div class="text-lightSlate-700 flex items-center gap-2">
              <div class="i-hugeicons:clock-01 text-lightSlate-100 h-5 w-5 shrink-0" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.last_updated" />
              </p>
              <div class="whitespace-nowrap text-sm">
                <Trans
                  key="modpack.last_updated_time"
                  options={{
                    time: formatDistanceToNowStrict(
                      new Date(props.data.data.lastUpdated).getTime()
                    )
                  }}
                />
              </div>
            </div>

            <div class="text-lightSlate-700 flex items-center gap-2">
              <div class="i-hugeicons:calendar-01 text-lightSlate-100 h-5 w-5 shrink-0" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.release_date" />
              </p>
              <div class="whitespace-nowrap text-sm">
                <Trans
                  key="modpack.release_date_time"
                  options={{
                    time: formatDistanceToNowStrict(
                      new Date(props.data.data.releaseDate).getTime()
                    )
                  }}
                />
              </div>
            </div>

            <div class="text-lightSlate-700 flex items-center gap-2">
              <div class="i-hugeicons:download-02 text-lightSlate-100 h-5 w-5 shrink-0" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.total_download" />
              </p>
              <div class="whitespace-nowrap text-sm">
                {formatDownloadCount(props.data.data.downloadsCount)}
              </div>
            </div>
            <div class="text-lightSlate-700 flex w-full items-center gap-2">
              <div class="i-hugeicons:joystick-01 text-lightSlate-100 h-5 w-5 shrink-0" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.mcVersion" />
              </p>
              <div class="flex max-h-20 w-full flex-wrap gap-2 overflow-auto text-sm">
                {props.data.data.minecraftVersions.join(" - ")}
              </div>
            </div>
          </div>
        </div>
      </div>

      <Show when={hasScreenshots}>
        <div class="border-darkSlate-800 max-h-140 absolute bottom-0 right-0 top-0 flex w-44 flex-col gap-4 overflow-y-auto overflow-x-hidden border-l p-4">
          <For each={props.data.data.screenshotUrls}>
            {(url) => (
              <div class="bg-darkSlate-900 h-40 w-40 rounded-md">
                <img src={url} class="h-full w-full rounded-md object-cover" />
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  )
}

export default OverviewPopover
