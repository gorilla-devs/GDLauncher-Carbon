import { getEmptyAddonPhraseKey } from "./translationKeys"
import { useSearchAddonType } from "./useSearchAddonType"
import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import { Skeleton } from "@gd/ui"
import { Show, Match, Switch } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import { Grid } from "./Grid"
import SharePreviewContent from "@/components/SharePreviewContent"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { isAddonInstalled } from "@/utils/installedAddon"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  effectiveFileId,
  instanceLinkTarget,
  LinkedSearchResult,
  serverLinkTarget
} from "@/utils/linkedFile"

export function List() {
  const [t] = useTransContext()
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()

  const allRows = () => searchContext?.allRows() || []

  const instanceId = () => searchContext?.selectedInstanceId() || NaN
  const instanceMods = () => searchContext?.selectedInstanceMods
  const serverId = () => searchContext?.selectedServerId() || NaN
  const serverAddons = () => searchContext?.selectedServerAddons
  const isServer = () => !isNaN(serverId()) && serverId() > 0
  const globalStore = useGlobalStore()
  // A link row whose linked file does not fit the instance or server installs
  // that target's own version, so any installed copy counts as installed. On
  // a server it fits, a copy named like the linked file is that file.
  const rowRequested = (row: LinkedSearchResult) =>
    effectiveFileId(
      row.requestedVersionId ?? undefined,
      row.linkedFile,
      row.type,
      isServer()
        ? serverLinkTarget(searchContext?.selectedServer?.data)
        : instanceLinkTarget(globalStore?.instances?.data, instanceId())
    )

  const type = useSearchAddonType(searchContext)

  // Note: Scroll restoration is handled in VList's ref callback
  // to ensure it works when switching between view modes

  const installedMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  return (
    <div class="flex h-full flex-col overflow-hidden">
      <Show when={searchContext?.directErrors().length}>
        <div role="alert">{searchContext?.directErrors().join("; ")}</div>
      </Show>
      <div class="flex-1 overflow-hidden">
        <Show when={searchContext?.isShareMode() && searchContext?.shareCode()}>
          <div class="flex h-full justify-center overflow-auto px-6 pt-6">
            <div class="w-full max-w-2xl h-full pb-6">
              <SharePreviewContent
                shareCode={searchContext!.shareCode()}
                expandMods
                onImportSuccess={() => navigator.navigate("/library")}
              />
            </div>
          </div>
        </Show>
        <Show when={!searchContext?.isShareMode()}>
          <Switch>
            <Match when={searchContext?.viewMode() === "grid"}>
              <Grid />
            </Match>
            <Match when={searchContext?.viewMode() === "list"}>
              <Show
                when={
                  !searchContext?.isInitialLoading() ||
                  allRows().some((r) => r.type === "value")
                }
                fallback={<Skeleton.searchList />}
              >
                <Show
                  when={allRows().length > 0}
                  fallback={
                    <div class="flex flex-col items-center justify-center px-6 py-16 text-center">
                      <PlaceholderGorilla
                        size={12}
                        variant="Searching Gorilla - Magnifying Glass"
                      />
                      <h3 class="mb-2 mt-6 text-xl font-semibold text-gray-300">
                        <Trans key="search:_trn_no_results_found" />
                      </h3>
                      <p class="max-w-md text-gray-500">
                        <Trans
                          key="search:_trn_no_results_description"
                          options={{ type: t(getEmptyAddonPhraseKey(type())) }}
                        />
                      </p>
                    </div>
                  }
                >
                  <VList
                    data={allRows()}
                    class="flex h-full max-w-full flex-col gap-4 overflow-auto px-6 pb-6"
                    ref={(v) => {
                      if (v) {
                        searchContext?.setRef(v)
                        // Restore scroll position when VList mounts
                        queueMicrotask(() => {
                          v.scrollTo(searchContext?.lastScrollOffset() ?? 0)
                        })
                      }
                    }}
                    onScroll={searchContext?.virtualOnScrollHandler}
                  >
                    {(result) => {
                      if (result.type === "loader") {
                        return <Skeleton.searchListItem />
                      }

                      if (result.type === "skeleton") {
                        return (
                          <div class="my-1 overflow-hidden rounded-md">
                            <div class="relative flex h-full cursor-pointer gap-2 overflow-hidden rounded-md border border-transparent px-8 py-4">
                              <div class="relative z-10 flex w-full items-center gap-4">
                                <div class="bg-darkSlate-500 skeleton-shimmer relative flex h-16 w-16 items-center justify-center rounded-md">
                                  <Show when={result.platform}>
                                    <img
                                      src={
                                        result.platform === "curseforge"
                                          ? CurseforgeLogo
                                          : ModrinthLogo
                                      }
                                      class="h-6 w-6 opacity-40"
                                      alt={`${result.platform} logo`}
                                    />
                                  </Show>
                                </div>
                                <div class="w-7/10 flex flex-col gap-2">
                                  <div class="truncate text-left text-xl font-medium">
                                    <div class="bg-darkSlate-500 skeleton-shimmer h-7 w-3/4 rounded-md" />
                                  </div>
                                  <div class="text-lightSlate-700 truncate text-left text-sm">
                                    <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-full rounded-md" />
                                  </div>
                                  <div class="flex items-center gap-2 py-1">
                                    <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-16 rounded-md" />
                                    <div class="bg-darkSlate-500 skeleton-shimmer h-5 w-20 rounded-md" />
                                  </div>
                                </div>
                                <div class="ml-auto flex items-center">
                                  <div class="relative flex items-center">
                                    <div class="flex items-center gap-2">
                                      <div class="text-lightSlate-700 text-sm">
                                        <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-14 rounded-md" />
                                      </div>
                                      <div class="bg-darkSlate-500 skeleton-shimmer h-5.5 w-12 rounded" />
                                    </div>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        )
                      }

                      return (
                        <ListItem
                          instanceMods={instanceMods()?.data ?? undefined}
                          serverAddons={serverAddons()?.data ?? undefined}
                          instanceId={instanceId()}
                          serverId={serverId()}
                          result={result.value!}
                          isInstalled={isAddonInstalled(
                            result.value!,
                            !isNaN(serverId()) && serverId() > 0
                              ? {
                                  kind: "server",
                                  addons: serverAddons()?.data ?? []
                                }
                              : {
                                  kind: "instance",
                                  addons: installedMods.data ?? []
                                },
                            rowRequested(result.value!),
                            result.value!.linkedFile
                          )}
                          onItemClick={() => {
                            const sid = serverId()
                            const iid = instanceId()
                            const idParam =
                              !isNaN(sid) && sid > 0
                                ? `serverId=${sid}`
                                : `instanceId=${iid}`
                            navigator.navigate(
                              `/addon/${result.value!.id}/${result.value!.platform}?${idParam}${result.value!.requestedVersionId ? `&requestedVersionId=${encodeURIComponent(result.value!.requestedVersionId)}` : ""}`
                            )
                          }}
                        />
                      )
                    }}
                  </VList>
                </Show>
              </Show>
            </Match>
          </Switch>
        </Show>
      </div>
    </div>
  )
}

export default List
