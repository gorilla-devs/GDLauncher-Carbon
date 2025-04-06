import { ModRowProps } from "@/utils/mods"
import { formatDownloadCount } from "@/utils/helpers"
import { CategoryIcon } from "@/utils/instances"
import { Trans } from "@gd/i18n"
import { Badge } from "@gd/ui"
import { formatDistanceToNowStrict } from "date-fns"
import { For, Match, Show, Switch } from "solid-js"
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
                  <img
                    src={author.avatarUrl as string}
                    class="h-3 w-3 rounded-full"
                  />
                </Match>
                <Match when={!author.avatarUrl}>
                  <div class="text-lightSlate-100 i-ri:user-fill h-3 w-3" />
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

  const categories = () => {
    const cats = props.data.data.categories

    if (props.data.data.platform === "curseforge") {
      return cats.map(
        (cat) => globalStore.categories.data?.curseforge[cat as number]
      )
    }

    return cats.map(
      (cat) => globalStore.categories.data?.modrinth[cat.toString()]
    )
  }

  return (
    <div
      class="bg-darkSlate-900 max-h-100 flex gap-2 overflow-hidden pb-4"
      classList={{
        "w-120": props.data.data.screenshotUrls.length > 0,
        "w-80": props.data.data.screenshotUrls.length === 0
      }}
      onMouseDown={(e) => {
        e.stopPropagation()
        e.preventDefault()
      }}
    >
      <div class="relative flex flex-1 flex-col overflow-hidden">
        <Show when={props.data.data.websiteUrl}>
          <div
            class="h-6 w-6 cursor-pointer rounded-lg"
            onClick={() => {
              const url = props.data.data.websiteUrl
              if (url) window.openExternalLink(url)
            }}
          >
            <div class="text-lightSlate-500 hover:text-lightSlate-50 transition-color transition-100 i-ri:external-link-line absolute right-4 top-4 z-30 h-4 w-4 ease-in-out" />
          </div>
        </Show>
        <h4 class="text-lightSlate-100 z-30 mb-2 px-4 text-xl">
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
              {(tag) => (
                <Badge class="bg-darkSlate-600 flex items-center gap-2">
                  <CategoryIcon category={tag} />
                  {tag.name}
                </Badge>
              )}
            </For>
          </div>
          <div class="mt-4 flex w-full flex-1 flex-col items-start gap-2">
            <div class="text-lightSlate-700 flex items-start gap-2">
              <span class="flex items-center gap-2">
                <div class="text-lightSlate-100 i-ri:user-fill h-4 w-4" />
                <p class="text-lightSlate-100 m-0 text-sm">
                  <Trans key="modpack.authors" />
                </p>
              </span>
              <Authors data={props.data} />
            </div>

            <div class="text-lightSlate-700 flex items-center gap-2">
              <div class="text-lightSlate-100 i-ri:time-fill" />
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
              <div class="text-lightSlate-100 i-ri:calendar-fill" />
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
              <div class="text-lightSlate-100 i-ri:download-fill" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.total_download" />
              </p>
              <div class="whitespace-nowrap text-sm">
                {formatDownloadCount(props.data.data.downloadsCount)}
              </div>
            </div>
            <div class="text-lightSlate-700 flex items-center gap-2">
              <div class="text-lightSlate-100 i-ri:gamepad-fill" />
              <p class="text-lightSlate-100 m-0 text-sm">
                <Trans key="modpack.mcVersion" />
              </p>
              <div class="scrollbar-hide flex max-w-full flex-wrap gap-2 text-sm">
                {/* {getLatestVersion(props.data)} */}
              </div>
            </div>
          </div>
        </div>
      </div>
      <Show when={props.data.data.screenshotUrls.length > 0}>
        <div class="border-darkSlate-800 relative flex flex-col gap-4 overflow-y-auto overflow-x-hidden border-t p-4">
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
