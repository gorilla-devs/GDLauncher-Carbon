import getRouteIndex from "@/route/getRouteIndex"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Tabs,
  TabList,
  Tab,
  Button,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem
} from "@gd/ui"
import { Outlet, useLocation, useParams, useRouteData } from "@solidjs/router"
import {
  For,
  JSX,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  createMemo
} from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { queryClient, rspc } from "@/utils/rspcClient"
import fetchData from "./instance.data"
import { detectDuplicatedMods } from "@/utils/duplicateMods"
import { InstanceDetails, ListInstance } from "@gd/core_module/bindings"
import {
  getInstanceImageUrl,
  getPreparingState,
  getRunningState
} from "@/utils/instances"
import DefaultImg from "/assets/images/default-instance-img.png"
// import { ContextMenu } from "@/components/ContextMenu";
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

interface InstancePage {
  label: string | JSX.Element
  path: string
}

const Instance = () => {
  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const [editableName, setEditableName] = createSignal(false)
  const [isFavorite, setIsFavorite] = createSignal(false)
  const [tabsTranslate, setTabsTranslate] = createSignal(0)
  const [isSticky, setIsSticky] = createSignal(false)
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const [newName, setNewName] = createSignal(
    routeData.instanceDetails.data?.name || ""
  )
  const [scrollTop, setScrollTop] = createSignal(0)

  const [t] = useTransContext()
  const modalsContext = useModal()

  // Detect duplicated mods
  const duplicatedMods = createMemo(() => {
    if (!routeData.instanceMods) return []
    return detectDuplicatedMods(routeData.instanceMods)
  })

  let backButtonRef: HTMLSpanElement
  let headerRef: HTMLElement
  let innerContainerRef: HTMLDivElement | undefined
  let refStickyTabs: HTMLDivElement
  let nameRef: HTMLHeadingElement | undefined

  const handleScroll = () => {
    if (!headerRef?.parentElement) return

    // Use requestAnimationFrame for smooth updates
    requestAnimationFrame(() => {
      setScrollTop(headerRef.parentElement?.scrollTop || 0)

      // Handle sticky tabs
      const rect = refStickyTabs.getBoundingClientRect()
      setIsSticky(rect.top <= 104)
      if (rect.top <= 104) {
        setTabsTranslate(0)
      } else {
        setTabsTranslate(-backButtonRef.offsetWidth)
      }
    })
  }

  onMount(() => {
    headerRef.parentElement?.addEventListener("scroll", handleScroll)
    setTabsTranslate(-backButtonRef.offsetWidth)
  })

  onCleanup(() => {
    headerRef.parentElement?.removeEventListener("scroll", handleScroll)
  })

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
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getAllInstances"]
      })

      const instancesUngrouped: ListInstance[] | undefined =
        queryClient.getQueryData(["instance.getAllInstances"])

      const instanceDetails: InstanceDetails | undefined =
        queryClient.getQueryData([
          "instance.getInstanceDetails",
          parseInt(params.id, 10)
        ])

      queryClient.setQueryData(
        ["instance.getInstanceDetails", parseInt(params.id, 10)],
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
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
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

  const instancePages = () => [
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:dashboard-square-01 text-lg" />
          <Trans key="ui.overview" />
        </div>
      ),
      path: `/library/${params.id}`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:puzzle text-lg" />
          <Trans key="ui.addons" />
        </div>
      ),
      path: `/library/${params.id}/addons`,
      noPadding: true
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:settings-01 text-lg" />
          <Trans key="ui.settings" />
        </div>
      ),
      path: `/library/${params.id}/settings`
    },
    {
      label: (
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:file-script text-lg" />
          <Trans key="ui.logs" />
          <FeatureStatusBadge type="beta" />
        </div>
      ),
      path: `/library/${params.id}/logs`
    }
    // {
    //   label: "Screenshots",
    //   path: `/library/${params.id}/screenshots`,
    // },
    // {
    //   label: "Versions",
    //   path: `/library/${params.id}/versions`,
    // },
  ]

  const selectedIndex = () =>
    getRouteIndex(instancePages(), location.pathname, true)

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
  const handlePlay = () => {
    const parsedInstanceId = parseInt(params.id, 10)
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
    launchInstanceMutation.mutate(parsedInstanceId)
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

  // Use rspc query hooks for automatic caching and deduplication
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

  // Combine both queries into a single computed value
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
        instance: parseInt(params.id, 10)
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
        id: params.id,
        modloader: routeData.instanceDetails.data?.modloaders[0]?.type_,
        title: routeData.instanceDetails.data?.name,
        mcVersion: routeData.instanceDetails.data?.version,
        modloaderVersion:
          routeData.instanceDetails.data?.modloaders[0]?.version,
        img: routeData.instanceDetails.data?.iconRevision
          ? getInstanceImageUrl(
              params.id,
              routeData.instanceDetails.data?.iconRevision
            )
          : null
      }
    )
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: parseInt(params.id, 10),
      folder: "Root"
    })
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      {
        name: "confirmInstanceDeletion"
      },
      {
        id: parseInt(params.id, 10),
        name: routeData.instanceDetails.data?.name
      }
    )
  }

  const menuItems = () => [
    {
      icon: "i-hugeicons:pencil-edit-01",
      label: t("instance.action_edit"),
      action: handleEdit
    },
    {
      icon: "i-hugeicons:folder-open",
      label: t("instance.action_open_folder"),
      action: handleOpenFolder
    },
    {
      icon: "i-hugeicons:file-export",
      label: t("instance.export_instance"),
      action: () => {
        const instanceId = getInstanceIdFromPath(location.pathname)

        setPayload({
          target: "Curseforge",
          save_path: undefined,
          self_contained_addons_bundling: false,
          filter: { entries: {} },
          instance_id: parseInt(instanceId!, 10)
        })
        setCheckedFiles([])
        setExportStep(0)

        modalsContext?.openModal(
          {
            name: "exportInstance"
          },
          {
            instanceId: parseInt(instanceId!, 10)
          }
        )
      }
    },
    {
      icon: "i-hugeicons:delete-02",
      label: t("instance.action_delete"),
      action: handleDelete
    }
  ]

  createEffect(() => {
    if (
      routeData.instancesUngrouped.data &&
      !routeData.instancesUngrouped.data?.find(
        (instance) => instance.id === parseInt(params.id, 10)
      )
    ) {
      navigator.navigate("/library")
    }
  })

  return (
    <main
      id="main-container-instance-details"
      class="bg-darkSlate-800 relative flex h-full flex-col overflow-x-hidden"
      classList={{
        "overflow-hidden": isFullScreen(),
        "overflow-y-auto overflow-x-hidden": !isFullScreen()
      }}
      style={{
        "scrollbar-gutter": "stable"
      }}
    >
      <header
        ref={(el) => {
          headerRef = el
        }}
        class="transition-100 relative flex min-h-60 flex-col items-stretch justify-between overflow-hidden transition-all ease-in-out"
      >
        <img
          src={
            routeData.instanceDetails.data?.iconRevision
              ? getInstanceImageUrl(
                  params.id,
                  routeData.instanceDetails.data?.iconRevision
                )
              : DefaultImg
          }
          alt="Instance cover"
          class="absolute h-full w-full object-cover"
          style={{
            transform: `translate3d(0, ${scrollTop() * 0.4}px, 0)`,
            "will-change": "transform"
          }}
        />
        <div class="from-darkSlate-800 relative z-10 h-full bg-gradient-to-t">
          <div class="sticky left-5 top-5 z-50 w-fit">
            <Button
              rounded
              onClick={() => navigator.navigate("/library")}
              size="small"
              type="transparent"
            >
              <div class="i-hugeicons:arrow-left-01 text-xl" />
            </Button>
          </div>
          <div class="absolute right-5 top-5 z-50 flex w-fit gap-2">
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
                  instance: parseInt(params.id, 10),
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
          </div>
          <div class="from-darkSlate-800 sticky top-52 z-20 box-border flex h-24 w-full justify-center bg-gradient-to-t px-6 pb-2">
            <div class="flex w-full justify-start">
              <div class="flex w-full items-end justify-between">
                <div class="flex flex-1 flex-row justify-end gap-4">
                  <img
                    src={
                      routeData.instanceDetails.data?.iconRevision
                        ? getInstanceImageUrl(
                            params.id,
                            routeData.instanceDetails.data?.iconRevision
                          )
                        : DefaultImg
                    }
                    alt="Instance icon"
                    class="h-16 w-16 rounded-xl object-cover"
                    style={{
                      "view-transition-name": `instance-tile-image`,
                      contain: "layout"
                    }}
                  />

                  <div class="flex flex-1 flex-col">
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
                            class="i-hugeicons:pencil-edit-01 transition-color hover:text-lightSlate-700 duration-100 ease-in-out"
                            onClick={() => setEditableName(true)}
                          />
                        </Show>
                      </span>
                      <div
                        class="relative flex h-full items-center gap-2 pr-2"
                        classList={{ "bg-darkSlate-800 pl-2": editableName() }}
                      >
                        <div
                          class="text-lightSlate-50 duration-50 z-10 cursor-pointer text-3xl transition ease-in-out hover:text-green-500 i-hugeicons:tick-02"
                          classList={{
                            hidden: !editableName()
                          }}
                          onClick={() => handleNameChange()}
                        />
                        <div
                          class="text-lightSlate-50 duration-50 z-10 cursor-pointer text-3xl transition ease-in-out hover:text-red-500 i-hugeicons:cancel-01"
                          classList={{
                            hidden: !editableName()
                          }}
                          onClick={() => {
                            if (
                              routeData.instanceDetails.data?.name &&
                              nameRef
                            ) {
                              setNewName(routeData.instanceDetails.data?.name)
                              nameRef.innerHTML =
                                routeData.instanceDetails.data?.name
                            }
                            setEditableName(false)
                          }}
                        />
                      </div>
                    </div>
                    <div
                      ref={innerContainerRef}
                      class="flex cursor-default flex-row justify-between"
                    >
                      <div class="text-lightGray-600 ml-2 mt-2 flex flex-row flex-wrap items-start gap-4">
                        <div
                          class="m-0 flex min-h-6 items-center gap-2"
                          style={{
                            "view-transition-name": `instance-tile-modloader`,
                            contain: "layout"
                          }}
                        >
                          <For
                            each={routeData.instanceDetails.data?.modloaders}
                          >
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
                            routeData.instanceDetails.data?.secondsPlayed !==
                            undefined
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
                      <div class="flex items-center gap-2">
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
                              <Trans key="instance.play" />
                            </Match>
                            <Match when={isRunning()}>
                              <div class="i-hugeicons:stop text-xl" />
                              <Trans key="instance.stop" />
                            </Match>
                          </Switch>
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </header>
      <div class="bg-darkSlate-800 sticky">
        <div
          class="flex justify-center py-0"
          classList={{
            "px-6": !instancePages()[selectedIndex()]?.noPadding
          }}
        >
          <div class="bg-darkSlate-800 w-full">
            <div
              class="bg-darkSlate-800 sticky top-0 z-30 flex h-14 items-center justify-between"
              classList={{
                "px-6": instancePages()[selectedIndex()]?.noPadding
              }}
              ref={(el) => {
                refStickyTabs = el
              }}
            >
              <div class="flex h-full items-center">
                <div
                  class="mr-4 origin-left transition-transform duration-100 ease-in-out"
                  classList={{
                    "scale-x-100": isSticky(),
                    "scale-x-0": !isSticky()
                  }}
                  ref={(el) => {
                    backButtonRef = el
                  }}
                >
                  <Button
                    onClick={() => navigator.navigate("/library")}
                    icon={<div class="i-hugeicons:arrow-left-01 text-2xl" />}
                    size="small"
                    type="secondary"
                  >
                    <Trans key="instance.step_back" />
                  </Button>
                </div>
                <div
                  class="flex h-full origin-left items-center transition-transform duration-100 ease-in-out"
                  style={{
                    transform: `translateX(${tabsTranslate()}px)`
                  }}
                >
                  <Tabs index={selectedIndex()}>
                    <TabList>
                      <For each={instancePages()}>
                        {(page: InstancePage) => (
                          <Tab
                            onClick={() => {
                              navigator.navigate(page.path)
                            }}
                          >
                            {page.label}
                          </Tab>
                        )}
                      </For>
                    </TabList>
                  </Tabs>
                </div>
              </div>
              <div
                class="ml-4 origin-right transition-transform duration-100 ease-in-out"
                classList={{
                  "scale-x-100": isSticky(),
                  "scale-x-0": !isSticky()
                }}
              >
                <Button
                  uppercase
                  size="small"
                  variant={isRunning() && "red"}
                  loading={isPreparing() !== undefined}
                  onClick={handlePlay}
                >
                  <Switch>
                    <Match when={!isRunning()}>
                      <div class="i-hugeicons:play text-base" />
                      <Trans key="instance.play" />
                    </Match>
                    <Match when={isRunning()}>
                      <div class="i-hugeicons:stop text-base" />
                      <Trans key="instance.stop" />
                    </Match>
                  </Switch>
                </Button>
              </div>
            </div>
            <div
              class="px-0"
              classList={{
                "pt-14": isFullScreen(),
                "pt-0":
                  !isFullScreen() && location.pathname.includes("/addons"),
                "pt-4 px-4":
                  !isFullScreen() && !location.pathname.includes("/addons")
              }}
            >
              <Show when={duplicatedMods().length > 0}>
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
                        Duplicated Mods Detected
                      </h3>
                      <p class="m-0 text-sm text-yellow-300/70">
                        {duplicatedMods().length} mod
                        {duplicatedMods().length > 1 ? "s have" : " has"}{" "}
                        multiple versions installed. This may cause conflicts.
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
                          instanceId: parseInt(params.id, 10)
                        }
                      )
                    }}
                  >
                    <div class="i-hugeicons:magic-wand-01" />
                    Fix Now
                  </Button>
                </div>
              </Show>
              <Outlet />
            </div>
          </div>
        </div>
      </div>
    </main>
  )
}

export default Instance
