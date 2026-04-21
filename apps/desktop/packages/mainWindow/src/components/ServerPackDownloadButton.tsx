import { useGDNavigate } from "@/managers/NavigationManager"
import { queryClient, rspc } from "@/utils/rspcClient"
import {
  FEServerModpackSource,
  FEUnifiedSearchResult
} from "@gd/core_module/bindings"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  toast,
  Spinner,
  Tooltip,
  TooltipTrigger,
  TooltipContent
} from "@gd/ui"
import { Show, createSignal, getOwner, runWithOwner } from "solid-js"

interface ServerPackDownloadButtonProps {
  addon: FEUnifiedSearchResult | undefined
  size?: "small" | "medium" | "large"
  fileId?: number | string
  serverPackFileId?: string | null
  splitPosition?: "left" | "right"
}

const ServerPackDownloadButton = (props: ServerPackDownloadButtonProps) => {
  const owner = getOwner()
  const [loading, setLoading] = createSignal(false)
  const [t] = useTransContext()
  const rspcContext = rspc.useContext()

  const navigator = useGDNavigate()

  const createServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.createServerFromModpack"],
    onSuccess(serverId) {
      console.log(
        `[ServerPackDownloadButton] createServerFromModpack succeeded, serverId=`,
        serverId
      )
      setLoading(false)
      queryClient.invalidateQueries({ queryKey: ["server.getAllServers"] })
      queryClient.invalidateQueries({ queryKey: ["server.getGroups"] })
      toast.success(t("notifications:_trn_server_from_modpack_success"))
      navigator.navigate(`/library?mode=servers`)
    },
    onError(err) {
      console.error(
        `[ServerPackDownloadButton] createServerFromModpack failed:`,
        err
      )
      setLoading(false)
      toast.error(t("notifications:_trn_server_from_modpack_error"))
    }
  }))

  function handleDownload() {
    runWithOwner(owner, async () => {
      if (!props.addon) return
      setLoading(true)

      const serverPackFileId =
        props.serverPackFileId || props.addon.serverPackFileId
      if (!serverPackFileId) {
        setLoading(false)
        return
      }

      let modpackSource: FEServerModpackSource

      if (props.addon.platform === "curseforge") {
        const fileId = props.fileId
          ? Number.parseInt(props.fileId.toString(), 10)
          : Number.parseInt(props.addon.mainFileId || "0")
        const projectId = Number.parseInt(props.addon.id)
        const spFileId = Number.parseInt(serverPackFileId)

        modpackSource = {
          curseforge: {
            project_id: projectId,
            file_id: fileId,
            server_pack_file_id: spFileId
          }
        }
      } else {
        // Modrinth
        let versionId = props.fileId?.toString() || props.addon.mainFileId

        if (!versionId) {
          const mrVersions = await rspcContext.client.query([
            "modplatforms.modrinth.getProjectVersions",
            { project_id: props.addon.id.toString() }
          ])
          versionId = mrVersions[0].id
        }

        modpackSource = {
          modrinth: {
            project_id: props.addon.id,
            version_id: versionId
          }
        }
      }

      createServerMutation.mutate({
        name: props.addon.title,
        port: null,
        group: null,
        modpackSource: modpackSource,
        iconUrl: props.addon.imageUrl ?? null
      })
    })
  }

  return (
    <Tooltip>
      <TooltipTrigger>
        <div class="relative">
          <Button
            disabled={loading()}
            size={props.size || "medium"}
            onClick={handleDownload}
            class={
              props.splitPosition === "left"
                ? "!rounded-r-none"
                : props.splitPosition === "right"
                  ? "!rounded-l-none"
                  : ""
            }
          >
            <Show
              when={loading()}
              fallback={<div class="i-hugeicons:server-stack-01 text-xl" />}
            >
              <Spinner />
            </Show>
          </Button>
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <Trans key="instances:_trn_download_server" />
      </TooltipContent>
    </Tooltip>
  )
}

export default ServerPackDownloadButton
