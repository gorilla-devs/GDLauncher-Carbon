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
      return "i-hugeicons:checkmark-badge-01"
    case "snapshot":
      return "i-hugeicons:camera-01"
    case "beta":
      return "i-hugeicons:lab"
    case "rc":
    case "release-candidate":
      return "i-hugeicons:star"
    default:
      return "i-hugeicons:bookmark-01"
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
      <h1 class="mb-6 text-2xl font-medium">
        <Trans key="news:_trn_minecraft_patches">Minecraft Patch Notes</Trans>
      </h1>

      <Switch>
        <Match
          when={!patchNotes.isPending && (patchNotes.data?.length || 0) > 0}
        >
          <div class="patch-timeline relative">
            <div class="from-lightSlate-600 to-lightSlate-700 absolute bottom-8 left-8 top-8 w-px bg-gradient-to-b" />

            <For each={patchNotes.data}>
              {(item) => (
                <div class="group relative mb-8 flex">
                  <div class="relative z-10 mr-8 flex flex-col items-center">
                    <div class="relative">
                      <div
                        class={`flex h-16 w-16 items-center justify-center rounded-full border-2 text-sm font-bold shadow-lg transition-transform group-hover:scale-110 ${getVersionTypeColor(item.versionType)}`}
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
                        class={`border-darkSlate-800 absolute -right-1 -top-1 flex h-6 w-6 items-center justify-center rounded-full border-2 text-xs font-bold ${getVersionTypeColor(item.versionType)}`}
                      >
                        <i class={getVersionTypeIcon(item.versionType)} />
                      </div>
                    </div>
                  </div>

                  <div
                    class="border-darkSlate-600 bg-darkSlate-700 flex-1 cursor-pointer rounded-xl border p-6 shadow-lg transition-transform hover:scale-[1.02]"
                    onClick={() => {
                      props.onNavigateToDetail?.()
                      navigator.navigate(`/news/${item.id}`)
                    }}
                  >
                    <div class="flex gap-4">
                      <div class="shrink-0">
                        <img
                          src={item.image}
                          alt={item.title}
                          class="border-lightSlate-600/50 h-20 w-20 rounded-lg border object-cover shadow-md"
                        />
                      </div>

                      <div class="flex-1">
                        <div class="mb-3 flex items-start justify-between">
                          <h3 class="text-xl font-bold leading-tight text-white">
                            {item.title}
                          </h3>
                          <div class="ml-4 flex items-center gap-2">
                            <span
                              class={`rounded-full border px-3 py-1 text-xs font-semibold ${getVersionTypeColor(item.versionType)}`}
                            >
                              {item.versionType}
                            </span>
                          </div>
                        </div>

                        <p class="text-lightSlate-300 mb-4 line-clamp-3 text-sm leading-relaxed">
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
                          <div class="text-primary-400 flex items-center text-sm opacity-0 transition-opacity group-hover:opacity-100">
                            <span class="mr-2 font-medium">
                              <Trans key="general:_trn_common.read_more" />
                            </span>
                            <div class="i-hugeicons:arrow-right-01 text-lg" />
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
                <div class="relative mb-8 flex">
                  <div class="relative mr-8 flex flex-col items-center">
                    <div class="bg-darkSlate-500 skeleton-shimmer h-16 w-16 rounded-full" />
                  </div>
                  <div class="border-darkSlate-600 bg-darkSlate-700 flex-1 rounded-xl border p-6">
                    <div class="flex gap-4">
                      <div class="bg-darkSlate-500 skeleton-shimmer h-20 w-20 rounded-lg" />
                      <div class="flex-1 space-y-3">
                        <div class="bg-darkSlate-500 skeleton-shimmer h-6 w-3/4 rounded" />
                        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-full rounded" />
                        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-2/3 rounded" />
                        <div class="bg-darkSlate-500 skeleton-shimmer h-4 w-1/3 rounded" />
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
