import { For, Match, Switch } from "solid-js"
import { usePatchNotes } from "@/utils/news"
import { Trans } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"

const getVersionTypeColor = (versionType?: string) => {
  switch (versionType?.toLowerCase()) {
    case "release":
      return "bg-green-600 border-green-500 text-green-100"
    case "snapshot":
      return "bg-orange-600 border-orange-500 text-orange-100"
    case "beta":
      return "bg-yellow-600 border-yellow-500 text-yellow-100"
    case "rc":
    case "release-candidate":
      return "bg-purple-600 border-purple-500 text-purple-100"
    default:
      return "bg-primary-600 border-primary-500 text-primary-100"
  }
}

const getVersionTypeIcon = (versionType?: string) => {
  switch (versionType?.toLowerCase()) {
    case "release":
      return "i-ri:check-double-line"
    case "snapshot":
      return "i-ri:camera-line"
    case "beta":
      return "i-ri:test-tube-line"
    case "rc":
    case "release-candidate":
      return "i-ri:star-line"
    default:
      return "i-ri:bookmark-line"
  }
}

interface PatchesTabProps {
  scrollContainer?: HTMLDivElement
  onNavigateToDetail?: () => void
}

const PatchesTab = (props: PatchesTabProps) => {
  const navigator = useGDNavigate()
  const patchNotes = usePatchNotes()

  return (
    <div class="p-6">
      <h1 class="text-2xl font-medium mb-6">
        <Trans key="news.minecraft_patches">Minecraft Patch Notes</Trans>
      </h1>

      <Switch>
        <Match
          when={!patchNotes.isPending && (patchNotes.data?.length || 0) > 0}
        >
          <div class="relative patch-timeline">
            <div class="absolute left-8 top-8 bottom-8 w-px bg-gradient-to-b from-lightSlate-600 to-lightSlate-700"></div>

            <For each={patchNotes.data}>
              {(item) => (
                <div class="relative flex group mb-8">
                  <div class="relative flex flex-col items-center mr-8 z-10">
                    <div class="relative">
                      <div
                        class={`flex items-center justify-center w-16 h-16 rounded-full font-bold text-sm border-2 shadow-lg transition-transform group-hover:scale-110 ${getVersionTypeColor(item.versionType)}`}
                      >
                        <div class="text-center">
                          <div class="text-xs font-semibold">
                            {item.version ||
                              /\d+\.\d+/.exec(item.title)?.[0] ||
                              "v"}
                          </div>
                          <i
                            class={`${getVersionTypeIcon(item.versionType)} text-xs`}
                          />
                        </div>
                      </div>
                      <div
                        class={`absolute -top-1 -right-1 w-6 h-6 rounded-full border-2 border-darkSlate-800 flex items-center justify-center text-xs font-bold ${getVersionTypeColor(item.versionType)}`}
                      >
                        <i class={getVersionTypeIcon(item.versionType)} />
                      </div>
                    </div>
                  </div>

                  <div
                    class="flex-1 cursor-pointer p-6 rounded-xl border border-darkSlate-600 bg-darkSlate-700 shadow-lg transition-transform hover:scale-[1.02]"
                    onClick={() => {
                      props.onNavigateToDetail?.()
                      navigator.navigate(`/news/${item.id}`)
                    }}
                  >
                    <div class="flex gap-4">
                      <div class="flex-shrink-0">
                        <img
                          src={item.image}
                          alt={item.title}
                          class="w-20 h-20 object-cover rounded-lg border border-lightSlate-600/50 shadow-md"
                        />
                      </div>

                      <div class="flex-1">
                        <div class="flex items-start justify-between mb-3">
                          <h3 class="text-xl font-bold text-white leading-tight">
                            {item.title}
                          </h3>
                          <div class="flex items-center gap-2 ml-4">
                            <span
                              class={`text-xs px-3 py-1 rounded-full border font-semibold ${getVersionTypeColor(item.versionType)}`}
                            >
                              {item.versionType}
                            </span>
                          </div>
                        </div>

                        <p class="text-lightSlate-300 text-sm mb-4 line-clamp-3 leading-relaxed">
                          {item.description}
                        </p>

                        <div class="flex items-center justify-between">
                          <span class="text-lightSlate-400 text-sm font-medium">
                            {new Date(item.date).toLocaleDateString(undefined, {
                              year: "numeric",
                              month: "long",
                              day: "numeric"
                            })}
                          </span>
                          <div class="flex items-center text-primary-400 text-sm opacity-0 group-hover:opacity-100 transition-opacity">
                            <span class="mr-2 font-medium">Read More</span>
                            <i class="i-ri:arrow-right-line" />
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Match>
        <Match when={patchNotes.isPending}>
          <div class="space-y-6">
            <For each={Array.from({ length: 5 })}>
              {() => (
                <div class="relative flex mb-8">
                  <div class="relative flex flex-col items-center mr-8">
                    <div class="w-16 h-16 bg-darkSlate-500 skeleton-shimmer rounded-full"></div>
                  </div>
                  <div class="flex-1 p-6 rounded-xl border border-darkSlate-600 bg-darkSlate-700">
                    <div class="flex gap-4">
                      <div class="w-20 h-20 bg-darkSlate-500 skeleton-shimmer rounded-lg"></div>
                      <div class="flex-1 space-y-3">
                        <div class="h-6 bg-darkSlate-500 skeleton-shimmer rounded w-3/4"></div>
                        <div class="h-4 bg-darkSlate-500 skeleton-shimmer rounded w-full"></div>
                        <div class="h-4 bg-darkSlate-500 skeleton-shimmer rounded w-2/3"></div>
                        <div class="h-4 bg-darkSlate-500 skeleton-shimmer rounded w-1/3"></div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default PatchesTab
