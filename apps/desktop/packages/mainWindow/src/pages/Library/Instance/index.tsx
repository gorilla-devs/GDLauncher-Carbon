import getRouteIndex from "@/route/getRouteIndex"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem
} from "@gd/ui"
import { useLocation, useParams } from "@solidjs/router"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  createMemo,
  onMount
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { queryClient, rspc } from "@/utils/rspcClient"
import useInstanceData from "./instance.data"
import { detectDuplicatedMods } from "@/utils/duplicateMods"
import { InstanceDetails, ListInstance } from "@gd/core_module/bindings"
import {
  getInstanceImageUrl,
  getPreparingState,
  getRunningState
} from "@/utils/instances"
import DefaultImg from "/assets/images/default-instance-img.png"
import { useModal } from "@/managers/ModalsManager"
import { convertSecondsToHumanTime } from "@/utils/helpers"
import Authors from "./Info/Authors"
import { getModloaderIcon } from "@/utils/sidebar"
import { getInstanceIdFromPath } from "@/utils/routes"
import {
  setPayload,
  setExportStep
} from "@/managers/ModalsManager/modals/InstanceExport"
import { setCheckedFiles } from "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent"
import { isFullScreen } from "./Tabs/Log"
import FeatureStatusBadge from "@/components/FeatureStatusBadge"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import DetailPageLayout, {
  type DetailPageTab
} from "@/pages/Library/shared/DetailPageLayout"

interface InstancePage {
  label: string | JSX.Element
  path: string
  noPadding?: boolean
}

const Instance = (props: { children?: any }) => {
  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const instanceId = () => parseInt(params.id!, 10)
  const [editableName, setEditableName] = createSignal(false)
  const [isFavorite, setIsFavorite] = createSignal(false)
  const routeData = useInstanceData()
  const [newName, setNewName] = createSignal(
    routeData.instanceDetails.data?.name || ""
  )

  const [t] = useTransContext()
  const modalsContext = useModal()

  // Detect duplicated mods
  const duplicatedMods = createMemo(() => {
    if (!routeData.instanceMods) return []
    return detectDuplicatedMods(routeData.instanceMods)
  })

  let nameRef: HTMLHeadingElement | undefined

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"],
    onMutate: async (
      obj
    ): Promise<
      | {
          instancesUngrouped: ListInstance[]
          instanceDetails: InstanceDetails
        }
      | undefined
    > => {
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceDetails", instanceId()]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getAllInstances"]
      })

      const instancesUngrouped: ListInstance[] | undefined =
        queryClient.getQueryData(["instance.getAllInstances"])

      const instanceDetails: InstanceDetails | undefined =
        queryClient.getQueryData(["instance.getInstanceDetails", instanceId()])

      queryClient.setQueryData(
        ["instance.getInstanceDetails", instanceId()],
        (old: InstanceDetails | undefined) => {
          const newDetails = old
          if (newDetails) newDetails.favorite = obj.favorite
          if (newDetails) return newDetails
          else return old
        }
      )

      if (instancesUngrouped && instanceDetails)
        return { instancesUngrouped, instanceDetails }
    },
    onSettled() {
      queryClient.invalidateQueries({
        queryKey: ["instance.getInstanceDetails", instanceId()]
      })
      queryClient.invalidateQueries({
        queryKey: ["instance.getAllInstances"]
      })
      setIsFavorite((prev) => !prev)
    },
    onError(
      _error,
      _variables,
      context:
        | {
            instancesUngrouped: ListInstance[]
            instanceDetails: InstanceDetails
          }
        | undefined
    ) {
      if (context?.instanceDetails) {
        setIsFavorite(context.instanceDetails.favorite)
        queryClient.setQueryData(
          ["instance.getInstanceDetails"],
          context.instanceDetails
        )
      }
    }
  }))

  createEffect(() => {
    if (routeData.instanceDetails.data)
      setIsFavorite(routeData.instanceDetails.data?.favorite)
  })

  const instancePages = (): InstancePage[] => [
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:dashboard-square-01 text-lg" />
          <Trans key="ui:_trn_overview" />
        </div>
      ),
      path: `/library/${params.id}`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:puzzle text-lg" />
          <Trans key="ui:_trn_addons" />
        </div>
      ),
      path: `/library/${params.id}/addons`,
      noPadding: true
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:settings-01 text-lg" />
          <Trans key="ui:_trn_settings" />
        </div>
      ),
      path: `/library/${params.id}/settings`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:file-script text-lg" />
          <Trans key="ui:_trn_logs" />
          <FeatureStatusBadge type="beta" />
        </div>
      ),
      path: `/library/${params.id}/logs`
    }
  ]

  const selectedValue = () => {
    const index = getRouteIndex(instancePages(), location.pathname, true)
    return instancePages()[index]?.path || instancePages()[0]?.path
  }

  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  const updateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateInstance"]
  }))

  const killInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.killInstance"]
  }))

  const globalStore = useGlobalStore()

  onMount(() => {
    const id = instanceId()
    if (!isNaN(id)) {
      globalStore.markInstanceAsSeen(id)
    }
  })

  const handlePlay = () => {
    const parsedInstanceId = instanceId()
    if (isRunning()) {
      killInstanceMutation.mutate(parsedInstanceId)
      return
    }
    if (
      globalStore.currentlySelectedAccount()?.status === "expired" ||
      globalStore.currentlySelectedAccount()?.status === "invalid"
    ) {
      modalsContext?.openModal(
        {
          name: "accountExpired"
        },
        {
          id: parsedInstanceId
        }
      )
      return
    }
    launchInstanceMutation.mutate({
      id: parsedInstanceId,
      skipMemoryCheck: false
    })
  }

  const isRunning = () =>
    routeData.instanceDetails.data?.state &&
    getRunningState(routeData.instanceDetails.data?.state)

  const isPreparing = () =>
    routeData.instanceDetails.data?.state &&
    getPreparingState(routeData.instanceDetails.data?.state)

  const curseforgeProjectId = () => {
    const modpack = routeData.instanceDetails.data?.modpack
    if (modpack?.modpack.type === "curseforge") {
      return modpack.modpack.value.project_id
    }
    return null
  }

  const modrinthProjectId = () => {
    const modpack = routeData.instanceDetails.data?.modpack
    if (modpack?.modpack.type === "modrinth") {
      return modpack.modpack.value.project_id
    }
    return null
  }

  const curseforgeModpack = rspc.createQuery(() => ({
    queryKey: [
      "modplatforms.curseforge.getMod",
      { modId: curseforgeProjectId() ?? 0 }
    ],
    enabled: curseforgeProjectId() !== null
  }))

  const modrinthModpack = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getProject", modrinthProjectId() ?? ""],
    enabled: modrinthProjectId() !== null
  }))

  const modpackDetails = () => {
    if (curseforgeProjectId()) {
      return curseforgeModpack.data
    } else if (modrinthProjectId()) {
      return modrinthModpack.data
    }
    return undefined
  }

  const handleNameChange = () => {
    if (newName()) {
      updateInstanceMutation.mutate({
        name: { Set: newName() },
        useLoadedIcon: null,
        memory: null,
        notes: null,
        instance: instanceId()
      })
    }
    setEditableName(false)
  }

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))

  const handleEdit = () => {
    modalsContext?.openModal(
      {
        name: "instanceCreation"
      },
      {
        id: params.id!,
        modloader: routeData.instanceDetails.data?.modloaders[0]?.type_,
        title: routeData.instanceDetails.data?.name,
        mcVersion: routeData.instanceDetails.data?.version,
        modloaderVersion:
          routeData.instanceDetails.data?.modloaders[0]?.version,
        img: routeData.instanceDetails.data?.iconRevision
          ? getInstanceImageUrl(
              params.id!,
              routeData.instanceDetails.data?.iconRevision
            )
          : null
      }
    )
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: instanceId(),
      folder: "Root"
    })
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      {
        name: "confirmInstanceDeletion"
      },
      {
        id: instanceId(),
        name: routeData.instanceDetails.data?.name
      }
    )
  }

  const menuItems = () => [
    {
      icon: "i-hugeicons:pencil-edit-01",
      label: t("instances:_trn_action_edit"),
      action: handleEdit
    },
    {
      icon: "i-hugeicons:folder-open",
      label: t("instances:_trn_action_open_folder"),
      action: handleOpenFolder
    },
    {
      icon: "i-hugeicons:file-export",
      label: t("instances:_trn_export_instance"),
      action: () => {
        const exportInstanceId = getInstanceIdFromPath(location.pathname)

        setPayload({
          target: "Curseforge",
          save_path: undefined,
          self_contained_addons_bundling: false,
          filter: { entries: {} },
          instance_id: parseInt(exportInstanceId!, 10)
        })
        setCheckedFiles([])
        setExportStep(0)

        modalsContext?.openModal(
          {
            name: "exportInstance"
          },
          {
            instanceId: parseInt(exportInstanceId!, 10)
          }
        )
      }
    },
    {
      icon: "i-hugeicons:delete-02",
      label: t("instances:_trn_action_delete"),
      action: handleDelete
    }
  ]

  createEffect(() => {
    if (
      routeData.instancesUngrouped.data &&
      !routeData.instancesUngrouped.data?.find(
        (instance: { id: number }) => instance.id === instanceId()
      )
    ) {
      navigator.navigate("/library")
    }
  })

  const iconUrl = () =>
    routeData.instanceDetails.data?.iconRevision
      ? getInstanceImageUrl(
          params.id!,
          routeData.instanceDetails.data?.iconRevision
        )
      : DefaultImg

  const tabs = (): DetailPageTab[] =>
    instancePages().map((page) => ({
      id: page.path,
      label: page.label as JSX.Element
    }))

  return (
    <DetailPageLayout
      containerId="main-container-instance-details"
      headerImage={iconUrl()}
      icon={iconUrl()}
      headerInfoContent={
        <>
          <div
            class="flex w-fit items-center gap-4 pl-1"
            classList={{
              "border-2 border-darkSlate-800 border-solid rounded-lg bg-darkSlate-700":
                editableName(),
              "border-2 border-transparent border-solid rounded-lg":
                !editableName()
            }}
          >
            <span class="flex cursor-pointer gap-2">
              <h1
                ref={nameRef}
                onInput={(e) => {
                  setNewName(e.target.innerHTML)
                }}
                class="border-box z-10 m-0 min-h-10 cursor-text focus:outline-none focus-visible:border-0 focus-visible:outline-none"
                contentEditable={editableName()}
                onFocusIn={() => {
                  setEditableName(true)
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault()
                    handleNameChange()
                  }
                }}
                style={{
                  "view-transition-name": `instance-tile-title`,
                  contain: "layout"
                }}
              >
                {routeData.instanceDetails.data?.name}
              </h1>
              <Show when={!editableName()}>
                <div
                  class="i-hugeicons:pencil-edit-01 transition-color hover:text-lightSlate-700 ease-spring duration-100"
                  onClick={() => setEditableName(true)}
                />
              </Show>
            </span>
            <div
              class="relative flex h-full items-center gap-2 pr-2"
              classList={{ "bg-darkSlate-800 pl-2": editableName() }}
            >
              <div
                class="text-lightSlate-50 duration-50 ease-spring i-hugeicons:tick-02 z-10 cursor-pointer text-3xl transition hover:text-green-500"
                classList={{
                  hidden: !editableName()
                }}
                onClick={() => handleNameChange()}
              />
              <div
                class="text-lightSlate-50 duration-50 ease-spring i-hugeicons:cancel-01 z-10 cursor-pointer text-3xl transition hover:text-red-500"
                classList={{
                  hidden: !editableName()
                }}
                onClick={() => {
                  if (routeData.instanceDetails.data?.name && nameRef) {
                    setNewName(routeData.instanceDetails.data?.name)
                    nameRef.innerHTML = routeData.instanceDetails.data?.name
                  }
                  setEditableName(false)
                }}
              />
            </div>
          </div>
          <div class="flex cursor-default flex-row justify-between">
            <div class="text-lightGray-600 ml-2 mt-2 flex flex-row flex-wrap items-start gap-4">
              <div
                class="m-0 flex min-h-6 items-center gap-2"
                style={{
                  "view-transition-name": `instance-tile-modloader`,
                  contain: "layout"
                }}
              >
                <For each={routeData.instanceDetails.data?.modloaders}>
                  {(modloader) => (
                    <>
                      <Show when={modloader.type_}>
                        <img
                          class="h-5 w-5"
                          src={getModloaderIcon(modloader.type_)}
                          alt="Modloader icon"
                        />
                      </Show>
                      <span>{modloader.type_}</span>
                    </>
                  )}
                </For>
                <span>{routeData.instanceDetails.data?.version}</span>
              </div>
              <Show
                when={
                  routeData.instanceDetails.data?.secondsPlayed !== undefined
                }
              >
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:clock-01 text-lg" />
                  <span class="whitespace-nowrap">
                    {convertSecondsToHumanTime(
                      routeData.instanceDetails.data!.secondsPlayed
                    )}
                  </span>
                </div>
              </Show>
              <Authors
                modpackDetails={modpackDetails()}
                isCurseforge={curseforgeProjectId() !== null}
                isModrinth={modrinthProjectId() !== null}
              />
            </div>
          </div>
        </>
      }
      headerActions={
        <Button
          uppercase
          size="large"
          variant={isRunning() && "red"}
          loading={isPreparing() !== undefined}
          style={{
            "view-transition-name": `instance-tile-play-button`,
            contain: "layout"
          }}
          onClick={handlePlay}
        >
          <Switch>
            <Match when={!isRunning()}>
              <div class="i-hugeicons:play text-xl" />
              <Trans key="instances:_trn_play" />
            </Match>
            <Match when={isRunning()}>
              <div class="i-hugeicons:stop text-xl" />
              <Trans key="instances:_trn_stop" />
            </Match>
          </Switch>
        </Button>
      }
      headerTopRight={
        <>
          <DropdownMenu placement="bottom-end">
            <DropdownMenuTrigger class="b-0 bg-transparent p-0">
              <Button
                as="div"
                rounded
                class="h-full w-full"
                size="small"
                type="transparent"
              >
                <div class="i-hugeicons:more-horizontal text-xl" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <For each={menuItems()}>
                {(item) => (
                  <DropdownMenuItem onSelect={item.action}>
                    <div class="flex items-center gap-2">
                      <div class={item.icon} />
                      <span>{item.label}</span>
                    </div>
                  </DropdownMenuItem>
                )}
              </For>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button
            onClick={() =>
              setFavoriteMutation.mutate({
                instance: instanceId(),
                favorite: !routeData.instanceDetails.data?.favorite
              })
            }
            rounded
            size="small"
            type="transparent"
          >
            <div
              class="i-hugeicons:star text-xl"
              classList={{
                "text-yellow-500": isFavorite()
              }}
            />
          </Button>
        </>
      }
      tabs={tabs()}
      activeTabId={selectedValue()}
      onTabClick={(tab) => navigator.navigate(tab.id)}
      onBackClick={() => navigator.navigate("/library")}
      stickyRightButton={
        <Button
          size="small"
          variant={isRunning() && "red"}
          loading={isPreparing() !== undefined}
          onClick={handlePlay}
        >
          <Switch>
            <Match when={!isRunning()}>
              <div class="i-hugeicons:play text-xl" />
            </Match>
            <Match when={isRunning()}>
              <div class="i-hugeicons:stop text-xl" />
            </Match>
          </Switch>
        </Button>
      }
      noPaddingPaths={["/addons", "/logs"]}
      isFullScreen={isFullScreen}
    >
      <Show
        when={
          duplicatedMods().length > 0 &&
          !routeData.instanceDetails.data?.modpack?.locked
        }
      >
        <div
          class="mb-4 flex items-center justify-between rounded-xl border border-yellow-600/30 bg-yellow-900/20 p-4"
          classList={{
            "mx-6 mt-4": location.pathname.includes("/addons")
          }}
        >
          <div class="flex items-center gap-3">
            <div class="i-hugeicons:alert-01 text-2xl text-yellow-500" />
            <div>
              <h3 class="m-0 mb-1 font-semibold text-yellow-200">
                <Trans key="content:_trn_duplicated_mods_detected" />
              </h3>
              <p class="m-0 text-sm text-yellow-300/70">
                <Trans key="content:_trn_duplicated_mods_message" />
              </p>
            </div>
          </div>
          <Button
            type="primary"
            size="small"
            onClick={() => {
              modalsContext?.openModal(
                { name: "duplicatedModsResolution" },
                {
                  duplicatedMods: duplicatedMods().map((g) => g.mods),
                  instanceId: instanceId()
                }
              )
            }}
          >
            <div class="i-hugeicons:magic-wand-01" />
            <Trans key="instances:_trn_fix_now" />
          </Button>
        </div>
      </Show>
      {props.children}
    </DetailPageLayout>
  )
}

export default Instance
