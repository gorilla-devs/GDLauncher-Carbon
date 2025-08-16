import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import {
  Button,
  Progress,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { Match, Show, Switch, createEffect, createSignal } from "solid-js"
import useSearchContext from "./SearchInputContext"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"

interface ModDownloadButtonProps {
  fileId?: number | string
  addon: FEUnifiedSearchResult | undefined
}

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [loading, setLoading] = createSignal(false)
  const [taskId, setTaskId] = createSignal<number | null>(null)
  const [progress, setProgress] = createSignal<number | null>(null)
  const searchContext = useSearchContext()
  const instanceLocked = () =>
    searchContext?.selectedInstance?.data?.modpack?.locked

  const installLatestModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installLatestMod",
    onSuccess(taskId) {
      setTaskId(taskId)
    },
    onMutate() {
      setLoading(true)
    }
  }))

  createEffect(async () => {
    if (taskId() !== null) {
      const task = rspc.createQuery(() => ({
        queryKey: ["vtask.getTask", taskId()]
      }))

      createEffect(() => {
        if (task?.data?.progress.type === "Known") {
          setProgress(Math.round(task?.data?.progress.value * 100))
        } else if (task?.data === null && taskId() !== null) {
          // Task was completed and removed from task manager
          // Refetch the instance mods to update installation status
          searchContext?.selectedInstanceMods?.refetch()
          setLoading(false)
          setTaskId(null)
          setProgress(null)
        }
      })
    }
  })

  const installedMod = () => {
    return searchContext?.selectedInstanceMods?.data?.find((mod) => {
      if (!props.addon) return false

      if (props.addon.platform === "curseforge") {
        return (
          mod.curseforge?.project_id === parseInt(props.addon.id.toString(), 10)
        )
      } else if (props.addon.platform === "modrinth") {
        return mod.modrinth?.project_id === props.addon.id.toString()
      }

      return false
    })
  }

  const installModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installMod"
  }))

  createEffect(() => {
    if (installModMutation.isPending) {
      setLoading(true)
    }

    if (installModMutation.isSuccess) {
      setTaskId(installModMutation.data)
    }
  })

  const latestModInstallObj = () => {
    return props.addon?.platform === "curseforge"
      ? {
          Curseforge: parseInt(props.addon.id.toString(), 10)
        }
      : {
          Modrinth: props.addon!.id.toString()
        }
  }

  const modInstallObj = () => {
    return props.addon?.platform === "curseforge"
      ? {
          Curseforge: {
            project_id: parseInt(props.addon.id.toString(), 10),
            file_id: parseInt(props.fileId!.toString(), 10)
          }
        }
      : {
          Modrinth: {
            project_id: props.addon!.id.toString(),
            version_id: props.fileId!.toString()
          }
        }
  }

  const isInstalled = () => {
    const localInstalledMod = installedMod()

    const addon = props.addon
    if (!localInstalledMod || !addon) return false

    if (!props.fileId) {
      return !!localInstalledMod
    } else {
      if (addon.platform === "curseforge") {
        return (
          localInstalledMod.curseforge!.file_id ===
          parseInt(props.fileId.toString(), 10)
        )
      } else {
        return (
          localInstalledMod.modrinth!.version_id === props.fileId.toString()
        )
      }
    }
  }

  const handleDownload = async () => {
    if (!props.addon) return

    const instanceId = searchContext?.selectedInstance?.data?.id
    if (!instanceId || isInstalled()) return

    if (!props.fileId) {
      await installLatestModMutation.mutateAsync({
        instance_id: instanceId,
        mod_source: latestModInstallObj()
      })
    } else {
      const replacesMod = installedMod()?.id || null

      await installModMutation.mutateAsync({
        mod_source: modInstallObj(),
        instance_id: instanceId,
        install_deps: !replacesMod,
        replaces_mod: replacesMod
      })
    }
  }

  createEffect(() => {
    if (isInstalled()) {
      setLoading(false)
      setTaskId(null)
      setProgress(null)
    }
  })

  return (
    <Tooltip>
      <TooltipTrigger>
        <Button
          uppercase
          variant={isInstalled() ? "green" : "primary"}
          disabled={instanceLocked() && !isInstalled()}
          onClick={handleDownload}
        >
          <Show when={loading()}>
            <Spinner />
            <div
              class="transition-width duration-100 ease-in-out"
              classList={{
                "w-0": progress() === null,
                "w-14": progress() !== null
              }}
            >
              <Progress color="bg-white" value={progress()!} />
            </div>
          </Show>
          <Show when={!loading()}>
            <Switch>
              <Match when={!searchContext?.selectedInstance?.data?.id}>
                <Trans key="instance.no_instance_selected" />
              </Match>
              <Match when={isInstalled()}>
                <Trans key="mod.downloaded" />
              </Match>
              <Match when={instanceLocked()}>
                <Trans key="instance.instance_locked" />
              </Match>
              <Match when={!instanceLocked() && !props.fileId}>
                <Trans key="instance.download" />
              </Match>
              <Match
                when={
                  !instanceLocked() &&
                  props.fileId &&
                  installedMod() &&
                  !isInstalled()
                }
              >
                <Trans key="instance.switch_version" />
              </Match>
              <Match when={!instanceLocked() && props.fileId}>
                <Trans key="instance.download_version" />
              </Match>
            </Switch>
          </Show>
        </Button>
      </TooltipTrigger>
      {instanceLocked() && (
        <TooltipContent>
          <Trans key="instance.locked_cannot_apply_changes" />
        </TooltipContent>
      )}
    </Tooltip>
  )
}

export default ModDownloadButton
