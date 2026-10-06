import { useGDNavigate } from "@/managers/NavigationManager"
import { resolvePackVersion } from "@/utils/resolvePackVersion"
import { validatedCurseForgeFile } from "@/utils/requestedVersion"
import { TranslatedError, errorDescription } from "@/utils/translatedError"
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
      if (!props.addon || loading()) return
      const addon = { ...props.addon }
      const explicitFileId = props.fileId?.toString()
      const companion = props.serverPackFileId || addon.serverPackFileId
      setLoading(true)

      try {
        let modpackSource: FEServerModpackSource

        if (addon.platform === "curseforge") {
          let serverPackFileId = companion
          if (explicitFileId) {
            // A project-main companion cannot certify a different client release.
            const response = await rspcContext.client.query([
              "modplatforms.curseforge.getModFile",
              { modId: Number(addon.id), fileId: Number(explicitFileId) }
            ])
            if (
              !validatedCurseForgeFile(explicitFileId, addon.id, response.data)
            )
              throw new TranslatedError(
                "notifications:_trn_curseforge_file_unavailable",
                "The selected CurseForge file is unavailable."
              )
            serverPackFileId =
              response.data.serverPackFileId?.toString() ?? null
          }
          if (!serverPackFileId)
            throw new TranslatedError(
              "notifications:_trn_server_pack_unavailable",
              "No server pack is available for the selected release."
            )
          const fileId = Number(explicitFileId || addon.mainFileId)
          const projectId = Number(addon.id)
          const spFileId = Number(serverPackFileId)
          if (
            ![fileId, projectId, spFileId].every(
              (id) => Number.isSafeInteger(id) && id > 0
            )
          )
            throw new TranslatedError(
              "notifications:_trn_server_pack_file_invalid",
              "The selected server pack file is invalid."
            )

          modpackSource = {
            curseforge: {
              project_id: projectId,
              file_id: fileId,
              server_pack_file_id: spFileId
            }
          }
        } else {
          if (!companion)
            throw new TranslatedError(
              "notifications:_trn_server_pack_unavailable",
              "No server pack is available for the selected release."
            )
          const versionId = await resolvePackVersion(
            addon,
            explicitFileId,
            (projectId) =>
              rspcContext.client.query([
                "modplatforms.modrinth.getProjectVersions",
                { project_id: projectId }
              ])
          )

          modpackSource = {
            modrinth: {
              project_id: addon.id,
              version_id: versionId
            }
          }
        }

        createServerMutation.mutate({
          name: addon.title,
          port: null,
          group: null,
          modpackSource: modpackSource,
          iconUrl: addon.imageUrl ?? null
        })
      } catch (error) {
        setLoading(false)
        console.error("[ServerPackDownloadButton] No pack version:", error)
        toast.error(t("notifications:_trn_pack_version_failed"), {
          description: errorDescription(
            t,
            error,
            "notifications:_trn_pack_version_retry"
          )
        })
      }
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
