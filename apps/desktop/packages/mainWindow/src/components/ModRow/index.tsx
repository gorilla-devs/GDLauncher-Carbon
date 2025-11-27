import { useGDNavigate } from "@/managers/NavigationManager"
import { formatDownloadCount, truncateText } from "@/utils/helpers"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  toast,
  Popover,
  PopoverTrigger,
  PopoverContent,
  Spinner
} from "@gd/ui"
import { formatDistanceToNowStrict } from "date-fns"
import {
  createSignal,
  getOwner,
  Match,
  mergeProps,
  onCleanup,
  onMount,
  runWithOwner,
  Show,
  Switch
} from "solid-js"
import OverviewPopover from "../OverviewPopover"
import { ModRowProps } from "@/utils/mods"
import Categories from "./Categories"
import Authors from "./Authors"
import ModDownloadButton from "../ModDownloadButton"
import { Modpack } from "@gd/core_module/bindings"

const ModRow = (props: ModRowProps) => {
  const owner = getOwner()
  const [loading, setLoading] = createSignal(false)
  const [isRowSmall, setIsRowSmall] = createSignal(false)
  const [t] = useTransContext()
  const rspcContext = rspc.useContext()

  const mergedProps = mergeProps({ type: "Mod" as const }, props)
  const navigator = useGDNavigate()

  const prepareInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.prepareInstance"],
    onSuccess() {
      setLoading(false)
      toast.success(t("notifications:_trn_instance_created_success"))
    },
    onError() {
      setLoading(false)
      toast.error(t("notifications:_trn_instance_created_error"))
    },
    onSettled() {
      setLoading(false)
      navigator.navigate(`/library`)
    }
  }))

  const instanceId = () => props?.instanceId

  const loadIconMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.loadIconUrl"]
  }))

  const createInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createInstance"],
    onSuccess(instanceId) {
      setLoading(true)
      prepareInstanceMutation.mutate(instanceId)
    },
    onError() {
      setLoading(false)
      toast.error(t("notifications:_trn_modpack_download_error"))
    }
  }))

  const handleExplore = () => {
    navigator.navigate(
      `/${mergedProps.type === "Mod" ? "mods" : "modpacks"}/${props.data.id}/{
        props.data.platform
      }?instanceId=${instanceId()}`
    )
  }

  const instanceCreationObj = (
    fileId?: number | string,
    projectId?: number | string
  ) => {
    return {
      type: props.data.platform,
      value:
        props.data.platform === "curseforge"
          ? {
              file_id:
                (fileId as number) || parseInt(props.data.mainFileId || "0"),
              project_id: (projectId as number) || parseInt(props.data.id)
            }
          : {
              project_id: projectId?.toString() || props.data.id,
              version_id: fileId?.toString()!
            }
    } as Modpack
  }

  let containerRef: HTMLDivElement
  let resizeObserver: ResizeObserver

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      window.requestAnimationFrame(() => {
        for (const entry of entries) {
          const cr = entry.contentRect
          const shouldSetRowSmall = cr.width < 712
          if (isRowSmall() !== shouldSetRowSmall) {
            setIsRowSmall(shouldSetRowSmall)
          }
        }
      })
    })

    resizeObserver.observe(containerRef)
  })

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect()
    }
  })

  const Title = () => {
    return (
      <div class="flex flex-col justify-between">
        <div class="flex w-full justify-between">
          <Popover placement="right-start">
            <PopoverTrigger>
              <h2
                class="mb-1 mt-0 cursor-pointer overflow-hidden text-ellipsis whitespace-nowrap hover:underline"
                onClick={() => handleExplore()}
                classList={{
                  "max-w-140": !isRowSmall(),
                  "max-w-90": isRowSmall()
                }}
              >
                {props.data.title}
              </h2>
            </PopoverTrigger>
            <PopoverContent class="bg-darkSlate-900">
              <OverviewPopover data={props} />
            </PopoverContent>
          </Popover>
          <Categories modProps={props} isRowSmall={isRowSmall()} />
        </div>
        <div class="flex items-center gap-4">
          <div class="text-lightSlate-700 flex items-center gap-2">
            <i class="text-lightSlate-700 i-hugeicons:clock-01" />
            <div class="whitespace-nowrap text-sm">
              {formatDistanceToNowStrict(
                new Date(props.data.releaseDate).getTime()
              )}
            </div>
          </div>
          <div class="text-lightSlate-700 flex items-center gap-2">
            <i class="text-lightSlate-700 i-hugeicons:download-02" />
            <div class="whitespace-nowrap text-sm">
              {formatDownloadCount(props.data.downloadsCount)}
            </div>
          </div>
          <div class="text-lightSlate-700 flex items-center gap-2">
            <i class="text-lightSlate-700 i-hugeicons:user" />
            <Authors modProps={props} isRowSmall={isRowSmall} />
          </div>
        </div>
      </div>
    )
  }

  return (
    <div
      ref={(el) => (containerRef = el)}
      class="bg-darkSlate-700 relative box-border flex h-36 flex-col gap-4 overflow-hidden rounded-2xl p-5"
    >
      <div class="from-darkSlate-700 absolute inset-0 z-10 bg-gradient-to-r from-50%" />
      <div class="from-darkSlate-700 absolute inset-0 z-10 bg-gradient-to-t" />
      <Show when={props.data.imageUrl}>
        <img
          class="absolute bottom-0 right-0 top-0 z-0 w-1/2 select-none"
          src={props.data.imageUrl || ""}
        />
      </Show>
      <div class="flex w-full">
        <div class="flex w-full gap-4">
          <div class="bg-repeat-none z-10 flex w-full flex-col gap-2">
            <Title />
            <div class="flex w-full justify-between">
              <p class="text-lightSlate-700 max-h-15 m-0 max-w-full overflow-hidden text-ellipsis text-sm">
                <Switch>
                  <Match when={isRowSmall()}>
                    {truncateText(props.data.description, 60)}
                  </Match>
                  <Match when={!isRowSmall()}>
                    {truncateText(props.data.description, 120)}
                  </Match>
                </Switch>
              </p>
              <div class="flex w-full items-end justify-end">
                <Switch>
                  <Match
                    when={
                      mergedProps.type === "Mod" &&
                      props.data.type === "modpack"
                    }
                  >
                    <div class="flex items-center gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instances:_trn_explore_modpack" />
                      </Button>
                      <Show when={loading()}>
                        <Button>
                          <Spinner />
                        </Button>
                      </Show>
                      <Show when={!loading()}>
                        <Button
                          size={isRowSmall() ? "small" : "medium"}
                          disabled={loading()}
                          onClick={async () => {
                            runWithOwner(owner, async () => {
                              if (props.data.type !== "modpack") return

                              setLoading(true)

                              const imgUrl = props.data.imageUrl
                              if (imgUrl) loadIconMutation.mutate(imgUrl)

                              let fileVersion = undefined
                              if (props.data.platform !== "curseforge") {
                                const mrVersions =
                                  await rspcContext.client.query([
                                    "modplatforms.modrinth.getProjectVersions",
                                    {
                                      project_id: props.data.id
                                    }
                                  ])

                                fileVersion = mrVersions[0].id
                              }

                              createInstanceMutation.mutate({
                                group: 1,
                                use_loaded_icon: true,
                                notes: "",
                                name: props.data.title,
                                version: {
                                  Modpack: instanceCreationObj(fileVersion)
                                }
                              })
                            })
                          }}
                        >
                          <Show when={loading()}>
                            <Spinner />
                          </Show>
                          <Show when={!loading()}>
                            <Trans key="instances:_trn_download" />
                          </Show>
                        </Button>
                      </Show>
                    </div>
                  </Match>
                  <Match
                    when={
                      mergedProps.type === "Mod" && props.data.type === "mod"
                    }
                  >
                    <div class="flex gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instances:_trn_explore_modpack" />
                      </Button>
                      <ModDownloadButton addon={props.data} />
                    </div>
                  </Match>
                </Switch>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ModRow
