import {
  Button,
  Checkbox,
  Input,
  Progress,
  Skeleton,
  Switch,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { For, Show, createMemo, createSignal } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import Mod from "./Mod"
import { useParams, useRouteData } from "@solidjs/router"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import { rspc } from "@/utils/rspcClient"
import { createStore, produce, reconcile } from "solid-js/store"
import fetchData from "../../instance.data"
import { Mod as Modtype } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"

const Mods = () => {
  const [t] = useTransContext()
  const params = useParams()
  const navigator = useGDNavigate()
  const modalsContext = useModal()

  const [filter, setFilter] = createSignal("")
  const [selectedModsMap, setSelectedModsMap] = createStore<
    Record<string, boolean>
  >({})
  const [isModStatusToggleLoading, setIsModStatusToggleLoading] =
    createSignal(false)
  const routeData: ReturnType<typeof fetchData> = useRouteData()

  const isInstanceLocked = () => routeData.instanceDetails.data?.modpack?.locked

  const deleteModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteMod"]
  }))
  const disableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.disableMod"]
  }))
  const enableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.enableMod"]
  }))

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))

  const gotoSearchPage = () => {
    navigator.navigate(`/search?instanceId=${params.id}`)
  }

  const filteredMods = createMemo(() => {
    const filterName = filter().replaceAll(" ", "").toLowerCase()

    return filter()
      ? routeData.instanceMods?.filter(
          (item) =>
            item.filename
              .toLowerCase()
              .replaceAll(" ", "")
              .includes(filterName) ||
            item.metadata?.name
              ?.toLowerCase()
              .replaceAll(" ", "")
              .includes(filterName) ||
            item.curseforge?.name
              ?.toLowerCase()
              .replaceAll(" ", "")
              .includes(filterName) ||
            item.modrinth?.title
              ?.toLowerCase()
              .replaceAll(" ", "")
              .includes(filterName)
        )
      : routeData.instanceMods
  })

  const selectedMods = createMemo(() => {
    return routeData.instanceMods?.filter((mod) => selectedModsMap[mod.id])
  })

  const updateAllMods = () => {
    modalsContext?.openModal(
      {
        name: "modsUpdater"
      },
      {
        mods: routeData.instanceMods.filter((mod) => mod.has_update),
        instanceId: parseInt(params.id, 10)
      }
    )
  }

  const updateSelectedMods = () => {
    modalsContext?.openModal(
      {
        name: "modsUpdater"
      },
      {
        mods: selectedMods().filter((mod) => mod.has_update),
        instanceId: parseInt(params.id, 10)
      }
    )
  }

  const NoMods = () => {
    return (
      <div class="min-h-90 flex h-full w-full items-center justify-center">
        <div class="flex flex-col items-center justify-center gap-6 text-center">
          <PlaceholderGorilla
            size={10}
            variant="Confused Gorilla - Empty Chest"
          />
          <p class="text-lightSlate-700 max-w-100">
            <Trans key="content:_trn_no_mods_text" />
          </p>
          <Button
            type="outline"
            size="medium"
            onClick={() => {
              gotoSearchPage()
            }}
          >
            <Trans key="content:_trn_add_mod" />
          </Button>
        </div>
      </div>
    )
  }

  const sortAlphabetically = (a: Modtype, b: Modtype) => {
    const aName = a.curseforge?.name || a.metadata?.name || a.filename
    const bName = b.curseforge?.name || b.metadata?.name || b.filename
    return aName.localeCompare(bName, undefined, {
      sensitivity: "base",
      numeric: true
    })
  }

  const isSelectAllIndeterminate = () => {
    return (
      (selectedMods()?.length || 0) > 0 &&
      selectedMods()?.length !== routeData.instanceMods?.length
    )
  }

  return (
    <div>
      <div
        class="bg-darkSlate-900 border-darkSlate-700 border-1 shadow-darkSlate-900 w-130 fixed bottom-4 left-1/2 z-50 mx-auto flex h-16 origin-left -translate-x-1/2 items-center justify-between rounded-md border-solid pr-6 shadow-md transition-transform duration-100 ease-spring"
        classList={{
          "translate-y-24": selectedMods()?.length === 0
        }}
      >
        <div class="flex h-full items-center">
          <div
            class="text-lightSlate-700 hover:text-lightSlate-50 mr-2 flex h-full items-center px-6"
            onClick={() => setSelectedModsMap(reconcile({}))}
          >
            <div class="i-hugeicons:cancel-01 text-2xl" />
          </div>
          <div class="text-lightSlate-700">
            <Trans
              key="content:_trn_instance_selected_mods_count"
              options={{
                total: routeData.instanceMods?.length,
                selected: selectedMods()?.length
              }}
            />
          </div>
        </div>
        <div class="flex items-center gap-4">
          <Show when={isInstanceLocked()}>
            <Tooltip placement="top">
              <TooltipTrigger>
                <Switch
                  disabled
                  checked={selectedMods()?.every((mod) => mod.enabled) || false}
                />
              </TooltipTrigger>
              <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                <Trans key="instances:_trn_locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={!isInstanceLocked()}>
            <Switch
              isIndeterminate={
                selectedMods()?.some((mod) => mod.enabled) &&
                selectedMods()?.some((mod) => !mod.enabled)
              }
              isLoading={isModStatusToggleLoading()}
              checked={selectedMods()?.every((mod) => mod.enabled) || false}
              onChange={async (event) => {
                if (isModStatusToggleLoading()) return

                setIsModStatusToggleLoading(true)

                let action = event.target.checked

                if (
                  selectedMods()?.some((mod) => mod.enabled) &&
                  selectedMods()?.some((mod) => !mod.enabled)
                ) {
                  action = true
                }

                const modsThatNeedApply = selectedMods()?.filter(
                  (mod) => mod.enabled !== action
                )

                for (const mod of modsThatNeedApply || []) {
                  if (action) {
                    await enableModMutation.mutateAsync({
                      instance_id: parseInt(params.id, 10),
                      mod_id: mod.id
                    })
                  } else {
                    await disableModMutation.mutateAsync({
                      instance_id: parseInt(params.id, 10),
                      mod_id: mod.id
                    })
                  }

                  await new Promise((resolve) => setTimeout(resolve, 10))
                }

                setIsModStatusToggleLoading(false)
              }}
            />
          </Show>
          <Show when={isInstanceLocked()}>
            <Tooltip placement="top">
              <TooltipTrigger>
                <div class="text-lightSlate-700 flex cursor-pointer items-center gap-2">
                  <div class="i-hugeicons:delete-02 text-2xl" />
                  <Trans key="content:_trn_delete_mod" />
                </div>
              </TooltipTrigger>
              <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                <Trans key="instances:_trn_locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </Show>
          <Show when={!isInstanceLocked()}>
            <div
              class="text-lightSlate-700 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring hover:text-red-500"
              onClick={() => {
                Object.keys(selectedModsMap).forEach((mod) => {
                  deleteModMutation.mutate({
                    instance_id: parseInt(params.id, 10),
                    mod_id: mod
                  })
                })
              }}
            >
              <div class="i-hugeicons:delete-02 text-2xl" />
              <Trans key="content:_trn_delete_mod" />
            </div>
          </Show>
          <Show
            when={selectedMods().filter((mod) => mod.has_update).length > 0}
          >
            <Show when={isInstanceLocked()}>
              <Tooltip placement="top">
                <TooltipTrigger>
                  <div class="text-lightSlate-700 flex items-center gap-2">
                    <div class="i-hugeicons:download-02 text-2xl" />
                    <Trans key="content:_trn_update_mods" />
                  </div>
                </TooltipTrigger>
                <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                  <Trans key="instances:_trn_locked_cannot_apply_changes" />
                </TooltipContent>
              </Tooltip>
            </Show>
            <Show when={!isInstanceLocked()}>
              <div
                class="text-lightSlate-700 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring hover:text-green-500"
                onClick={() => {
                  updateSelectedMods()
                }}
              >
                <div class="i-hugeicons:download-02 text-2xl" />
                <Trans key="content:_trn_update_mods" />
              </div>
            </Show>
          </Show>
        </div>
      </div>

      <div class="bg-darkSlate-800 sticky top-14 z-10 flex flex-col px-6 transition-all duration-100 ease-spring">
        <div class="flex flex-wrap items-center justify-between gap-1 pb-4">
          <div class="flex cursor-pointer items-center gap-4">
            <Checkbox
              indeterminate={isSelectAllIndeterminate()}
              checked={
                (selectedMods()?.length || 0) > 0 && !isSelectAllIndeterminate()
              }
              onChange={(checked) => {
                let action = checked

                if (isSelectAllIndeterminate()) {
                  action = true
                }

                setSelectedModsMap(
                  produce((prev) => {
                    for (const mod of routeData.instanceMods || []) {
                      if (action) {
                        prev[mod.id] = action
                      } else {
                        delete prev[mod.id]
                      }
                    }

                    return prev
                  })
                )
              }}
            />
            <Input
              onInput={(e) => setFilter(e.target.value)}
              placeholder={t("content:_trn_mods.search")}
              icon={<div class="i-hugeicons:search-01" />}
              class="text-lightSlate-700 rounded-full"
            />
          </div>
          <div class="flex items-center gap-4">
            {/* <p class="text-lightSlate-700">
              <Trans key="content:_trn_sort_by" />
            </p>
            <Dropdown
              options={[
                { label: t("content:_trn_sort_by_asc"), key: "_trn_asc" },
                { label: t("content:_trn_sort_by_desc"), key: "_trn_desc" }
              ]}
              value={"asc"}
              rounded
            /> */}
            {/* <div
              class="flex items-center gap-2 cursor-pointer duration-100 ease-spring transition hover:text-lightSlate-50 text-lightSlate-700"
              onClick={() => {
                openFolderMutation.mutate({
                  folder: "Mods",
                  instance_id: parseInt(params.id, 10)
                });
              }}
            >
              <span class="text-2xl i-hugeicons:filter" />
            </div> */}
            <Show when={isInstanceLocked()}>
              <Tooltip placement="top">
                <TooltipTrigger>
                  <Button disabled type="outline" size="medium">
                    <Trans key="content:_trn_add_mod" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                  <Trans key="instances:_trn_locked_cannot_apply_changes" />
                </TooltipContent>
              </Tooltip>
            </Show>
            <Show when={!isInstanceLocked()}>
              <Button
                disabled={isInstanceLocked()}
                type="outline"
                size="medium"
                onClick={() => {
                  gotoSearchPage()
                }}
              >
                <Trans key="content:_trn_add_mod" />
              </Button>
            </Show>

            <Show
              when={
                routeData.instanceMods?.filter((mod) => mod.has_update).length >
                0
              }
            >
              <Tooltip placement="top">
                <TooltipTrigger>
                  <div
                    class="text-lightSlate-700 flex items-center gap-2 transition duration-100 ease-spring hover:text-green-500"
                    onClick={() => {
                      if (isInstanceLocked()) return

                      updateAllMods()
                    }}
                  >
                    <div class="i-hugeicons:download-02 text-2xl" />
                    <div
                      classList={{
                        "w-0": isInstanceLocked()
                      }}
                      class="transition-width duration-100"
                    >
                      <Progress value={15} />
                    </div>
                  </div>
                </TooltipTrigger>
                <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                  <Show when={isInstanceLocked()}>
                    <Trans key="instances:_trn_locked_cannot_apply_changes" />
                  </Show>
                  <Show when={!isInstanceLocked()}>
                    <Trans key="content:_trn_update_all_mods" />
                  </Show>
                </TooltipContent>
              </Tooltip>
            </Show>

            <Tooltip placement="top">
              <TooltipTrigger>
                <div
                  class="hover:text-lightSlate-50 text-lightSlate-700 flex cursor-pointer items-center gap-2 transition duration-100 ease-spring"
                  onClick={() => {
                    openFolderMutation.mutate({
                      folder: "Mods",
                      instance_id: parseInt(params.id, 10)
                    })
                  }}
                >
                  <div class="i-hugeicons:folder-open text-2xl" />
                </div>
              </TooltipTrigger>
              <TooltipContent class="max-w-38 overflow-hidden text-ellipsis">
                <Trans key="content:_trn_open_mods_folder" />
              </TooltipContent>
            </Tooltip>
          </div>
        </div>
      </div>
      <div class="h-full w-full overflow-y-hidden pb-14">
        <Show
          when={
            routeData.instanceMods &&
            routeData.instanceMods?.length > 0 &&
            !routeData.instanceDetails.isLoading
          }
          fallback={<NoMods />}
        >
          <For each={[...(filteredMods() || [])].sort(sortAlphabetically)}>
            {(mod) => (
              <Mod
                mod={mod}
                setSelectedMods={setSelectedModsMap}
                selectMods={selectedModsMap}
                isInstanceLocked={isInstanceLocked()}
              />
            )}
          </For>
        </Show>
        <Show when={routeData.instanceDetails.isLoading}>
          <Skeleton.sidebarInstances />
        </Show>
      </div>
    </div>
  )
}

export default Mods
