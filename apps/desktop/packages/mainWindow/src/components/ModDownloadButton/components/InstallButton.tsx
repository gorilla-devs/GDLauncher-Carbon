import { Show, Switch, Match, Accessor } from "solid-js"
import { Trans } from "@gd/i18n"
import type { worldVersionCopy } from "../worldVersionCopy"
import type { InstalledArtifact } from "@/utils/installedAddon"
import { LinkedFileMismatchNotice } from "./LinkedFileMismatchNotice"
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
  installedMod: () => InstalledArtifact | undefined
  onDownload: () => void
  size?: "small" | "medium" | "large"
  iconOnly?: boolean
  staged?: boolean
  copyPresentation?: ReturnType<typeof worldVersionCopy>
  /** Set when a link's file does not fit this instance: the game versions it
   *  lists, shown as "26.3 · NeoForge" (or a generic notice when they name
   *  none). The instance's own version is installed instead, and the tooltip
   *  says so. */
  linkedFileMismatch?: readonly string[]
  testId?: string
}

export const InstallButton = (props: InstallButtonProps) => {
  return (
    <Tooltip>
      <TooltipTrigger>
        <Button
          data-testid={props.testId}
          uppercase={!props.iconOnly}
          variant={props.isInstalled() ? "green" : "primary"}
          disabled={props.instanceLocked() && !props.isInstalled()}
          size={props.size || "medium"}
          onClick={props.onDownload}
        >
          <Show when={props.loading()}>
            <Spinner />
            <Show when={!props.iconOnly}>
              <div
                class="transition-width duration-100 ease-spring"
                classList={{
                  "w-0": props.progress() === null,
                  "w-14": props.progress() !== null
                }}
              >
                <Progress color="bg-white" value={props.progress()!} />
              </div>
            </Show>
          </Show>
          <Show when={!props.loading()}>
            <Show
              when={props.iconOnly}
              fallback={
                <Switch>
                  <Match when={props.isInstalled()}>
                    <Show
                      when={props.staged}
                      fallback={<Trans key="content:_trn_mod.downloaded" />}
                    >
                      <Trans key="content:_trn_datapack_downloaded" />
                    </Show>
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
                      <Show
                        when={props.copyPresentation}
                        fallback={<Trans key="instances:_trn_switch_version" />}
                      >
                        {(copy) => <Trans key={copy().action} />}
                      </Show>
                    </div>
                  </Match>
                  <Match when={!props.instanceLocked() && props.fileId}>
                    <div class="flex items-center gap-1.5">
                      <div class="i-hugeicons:download-02 h-5 w-5" />
                      <Trans key="instances:_trn_download_version" />
                    </div>
                  </Match>
                </Switch>
              }
            >
              <Switch>
                <Match when={props.isInstalled()}>
                  <div class="i-hugeicons:tick-02 text-xl" />
                </Match>
                <Match when={!props.isInstalled()}>
                  <div class="i-hugeicons:download-02 text-xl" />
                </Match>
              </Switch>
            </Show>
          </Show>
        </Button>
      </TooltipTrigger>
      <Show
        when={
          !props.instanceLocked() &&
          !props.isInstalled() &&
          props.copyPresentation
        }
      >
        {(copy) => (
          <TooltipContent>
            <Trans key={copy().disclosure} />
          </TooltipContent>
        )}
      </Show>
      <Show
        when={
          !props.instanceLocked() &&
          !props.isInstalled() &&
          props.linkedFileMismatch !== undefined
        }
      >
        <TooltipContent>
          <LinkedFileMismatchNotice gameVersions={props.linkedFileMismatch!} />
        </TooltipContent>
      </Show>
      {props.instanceLocked() && (
        <TooltipContent>
          <Trans key="instances:_trn_locked_cannot_apply_changes" />
        </TooltipContent>
      )}
    </Tooltip>
  )
}
