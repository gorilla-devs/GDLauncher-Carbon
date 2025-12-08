import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import { FEUnifiedSearchResult, Modpack } from "@gd/core_module/bindings"
import { Trans, useTransContext } from "@gd/i18n"
import { Button, toast, Spinner } from "@gd/ui"
import { Show, createSignal, getOwner, runWithOwner } from "solid-js"

interface ModDownloadButtonProps {
  fileId?: number | string
  name?: string
  addon: FEUnifiedSearchResult | undefined
  size?: "small" | "medium" | "large"
  iconOnly?: boolean
}

const ModpackDownloadButton = (props: ModDownloadButtonProps) => {
  const owner = getOwner()
  const [loading, setLoading] = createSignal(false)
  const [t] = useTransContext()
  const rspcContext = rspc.useContext()

  const navigator = useGDNavigate()

  const prepareInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.prepareInstance"],
    async onSuccess() {
      setLoading(false)
      toast.success(t("notifications:_trn_instance_created_success"))

      navigator.navigate(`/library`)
    },
    onError() {
      setLoading(false)
      toast.error(t("notifications:_trn_instance_created_error"))
    }
  }))

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

  const instanceCreationObj = (
    fileId?: number | string,
    projectId?: number | string
  ) => {
    if (!props.addon?.mainFileId) {
      throw new Error("No main file ID found")
    }

    const request =
      props.addon.platform === "curseforge"
        ? ({
            type: "curseforge",
            value: {
              file_id:
                Number.parseInt(fileId?.toString() || "", 10) ||
                Number.parseInt(props.addon.mainFileId),
              project_id:
                Number.parseInt(projectId?.toString() || "", 10) ||
                Number.parseInt(props.addon.id)
            }
          } satisfies Modpack)
        : ({
            type: "modrinth",
            value: {
              project_id: projectId?.toString() || props.addon.id,
              version_id: fileId?.toString()! || props.addon.mainFileId
            }
          } satisfies Modpack)

    return request
  }

  function handleDownload() {
    runWithOwner(owner, async () => {
      setLoading(true)

      const imgUrl = props.addon?.imageUrl
      if (imgUrl) loadIconMutation.mutate(imgUrl)

      let fileVersion = props.fileId
      if (!fileVersion && props.addon?.platform === "modrinth") {
        const mrVersions = await rspcContext.client.query([
          "modplatforms.modrinth.getProjectVersions",
          {
            project_id: props.addon.id.toString()
          }
        ])
        fileVersion = mrVersions[0].id
      }

      createInstanceMutation.mutate({
        use_loaded_icon: true,
        notes: "",
        name: props.name || props.addon?.title!,
        version: {
          Modpack: instanceCreationObj(fileVersion, props.addon?.id)
        }
      })
    })
  }

  return (
    <div class="relative">
      <Button
        disabled={loading()}
        size={props.size || "medium"}
        onClick={handleDownload}
      >
        <Show when={loading()}>
          <Spinner />
        </Show>
        <Show when={!loading()}>
          <Show
            when={props.iconOnly}
            fallback={
              <div class="flex items-center gap-1.5">
                <div class="i-hugeicons:download-02" />
                <Trans key="instances:_trn_download" />
              </div>
            }
          >
            <div class="i-hugeicons:download-02 text-xl" />
          </Show>
        </Show>
      </Button>
    </div>
  )
}

export default ModpackDownloadButton
