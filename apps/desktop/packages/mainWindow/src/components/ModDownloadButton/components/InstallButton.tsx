import { Show, Switch, Match, Accessor } from "solid-js"
import { Trans } from "@gd/i18n"
import {
  Button,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  Spinner,
  Progress
} from "@gd/ui"

interface InstallButtonProps {
  loading: Accessor<boolean>
  progress: Accessor<number | null>
  isInstalled: () => boolean
  instanceLocked: () => boolean
  fileId?: number | string
  installedMod: () => any
  onDownload: () => void
  size?: "small" | "medium" | "large"
}

export const InstallButton = (props: InstallButtonProps) => {
  return (
    <Tooltip>
      <TooltipTrigger>
        <Button
          uppercase
          variant={props.isInstalled() ? "green" : "primary"}
          disabled={props.instanceLocked() && !props.isInstalled()}
          size={props.size || "medium"}
          onClick={props.onDownload}
        >
          <Show when={props.loading()}>
            <Spinner />
            <div
              class="transition-width duration-100 ease-in-out"
              classList={{
                "w-0": props.progress() === null,
                "w-14": props.progress() !== null
              }}
            >
              <Progress color="bg-white" value={props.progress()!} />
            </div>
          </Show>
          <Show when={!props.loading()}>
            <Switch>
              <Match when={props.isInstalled()}>
                <Trans key="content:_trn_mod.downloaded" />
              </Match>
              <Match when={props.instanceLocked()}>
                <Trans key="instances:_trn_instance_locked" />
              </Match>
              <Match when={!props.instanceLocked() && !props.fileId}>
                <div class="flex items-center gap-1.5">
                  <div class="i-hugeicons:download-02 h-5 w-5" />
                  <Trans key="instances:_trn_download" />
                </div>
              </Match>
              <Match
                when={
                  !props.instanceLocked() &&
                  props.fileId &&
                  props.installedMod() &&
                  !props.isInstalled()
                }
              >
                <div class="flex items-center gap-1.5">
                  <div class="i-hugeicons:download-02 h-5 w-5" />
                  <Trans key="instances:_trn_switch_version" />
                </div>
              </Match>
              <Match when={!props.instanceLocked() && props.fileId}>
                <div class="flex items-center gap-1.5">
                  <div class="i-hugeicons:download-02 h-5 w-5" />
                  <Trans key="instances:_trn_download_version" />
                </div>
              </Match>
            </Switch>
          </Show>
        </Button>
      </TooltipTrigger>
      {props.instanceLocked() && (
        <TooltipContent>
          <Trans key="instances:_trn_locked_cannot_apply_changes" />
        </TooltipContent>
      )}
    </Tooltip>
  )
}
