import {
  FEUnifiedSearchResultWithDescription,
  FEUnifiedSearchType,
  InstanceDetails,
  Mod
} from "@gd/core_module/bindings"
import { VersionRowTypeData } from "../InfiniteScrollVersionsQueryWrapper"
import { For, Match, Show, Switch, createMemo } from "solid-js"
import { format } from "date-fns"
import { Badge, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import ModDownloadButton from "../ModDownloadButton"
import ModpackDownloadButton from "../ModpackDownloadButton"

export interface Props {
  modVersion: VersionRowTypeData
  project: FEUnifiedSearchResultWithDescription | undefined
  isCurseforge?: boolean
  instanceId?: number | null
  instanceDetails?: InstanceDetails
  instanceMods?: Mod[]
  installedFile:
    | {
        id: string
        remoteId: string | number
      }
    | undefined
  type: FEUnifiedSearchType | undefined
}

export interface AdditionalProps {
  loading: boolean
  disabled: boolean
  isInstalled?: boolean
}

const formatDownloadCount = (count: number) => {
  if (count >= 1000000) {
    return (count / 1000000).toFixed(1) + "M"
  } else if (count >= 1000) {
    return (count / 1000).toFixed(1) + "K"
  }
  return count.toString()
}

const RowContainer = (props: Props & AdditionalProps) => {
  const fileId = createMemo(() =>
    props.project?.platform === "curseforge"
      ? parseInt(props.modVersion.fileId, 10)
      : props.modVersion.fileId
  )

  return (
    <Switch>
      <Match when={props.modVersion}>
        <div class="contents">
          {/* Version name column */}
          <div class="flex flex-col justify-center py-2 min-w-0">
            <div class="flex items-center gap-2 min-w-0">
              <Show when={props.isInstalled}>
                <div class="i-ri:check-line text-green-400 text-sm" />
              </Show>
              <Tooltip placement="top">
                <TooltipTrigger>
                  <h4 class="text-lightSlate-50 text-sm font-medium leading-tight truncate">
                    {props.modVersion.name.replaceAll(".zip", "")}
                  </h4>
                </TooltipTrigger>
                <TooltipContent>
                  {props.modVersion.name.replaceAll(".zip", "")}
                </TooltipContent>
              </Tooltip>
            </div>
            {/* Game versions - compact display */}
            <div class="mt-1 flex gap-1 overflow-hidden">
              <For each={props.modVersion.gameVersions.slice(0, 2)}>
                {(version) => (
                  <Badge variant="secondary" class="text-xs">
                    {version}
                  </Badge>
                )}
              </For>
              <Show when={props.modVersion.gameVersions.length > 2}>
                <Tooltip placement="top">
                  <TooltipTrigger>
                    <Badge variant="secondary" class="text-xs">
                      +{props.modVersion.gameVersions.length - 2}
                    </Badge>
                  </TooltipTrigger>
                  <TooltipContent>
                    <div class="flex flex-col gap-1">
                      <div class="font-medium text-xs">
                        Additional versions:
                      </div>
                      <div class="flex flex-wrap gap-1 max-w-xs">
                        <For each={props.modVersion.gameVersions.slice(2)}>
                          {(version) => (
                            <Badge variant="secondary" class="text-xs">
                              {version}
                            </Badge>
                          )}
                        </For>
                      </div>
                    </div>
                  </TooltipContent>
                </Tooltip>
              </Show>
            </div>
          </div>

          {/* Published date column */}
          <div class="flex items-center text-lightSlate-300 text-sm">
            {format(new Date(props.modVersion.datePublished), "MMM dd, yyyy")}
          </div>

          {/* Downloads column */}
          <div class="flex items-center text-lightSlate-300 text-sm">
            {formatDownloadCount(props.modVersion.downloads)}
          </div>

          {/* Release type column */}
          <div class="flex items-center gap-2">
            <div
              class="h-2 w-2 rounded-full"
              classList={{
                "bg-green-500":
                  props.modVersion.releaseType === "stable" ||
                  props.modVersion.releaseType === "release",
                "bg-yellow-500": props.modVersion.releaseType === "beta",
                "bg-red-500": props.modVersion.releaseType === "alpha"
              }}
            />
            <span
              class="text-sm font-medium capitalize"
              classList={{
                "text-green-400":
                  props.modVersion.releaseType === "stable" ||
                  props.modVersion.releaseType === "release",
                "text-yellow-400": props.modVersion.releaseType === "beta",
                "text-red-400": props.modVersion.releaseType === "alpha"
              }}
            >
              {props.modVersion.releaseType}
            </span>
          </div>

          {/* Download button column */}
          <div class="flex items-center justify-end">
            <Switch>
              <Match when={props.type === "modpack"}>
                <ModpackDownloadButton
                  addon={props.project}
                  name={props.modVersion.name}
                  fileId={fileId()}
                  size="small"
                />
              </Match>
              <Match when={props.type !== "modpack"}>
                <ModDownloadButton
                  selectedInstanceId={props.instanceId ?? undefined}
                  selectedInstanceMods={props.instanceMods}
                  addon={props.project}
                  fileId={fileId()}
                  size="small"
                />
              </Match>
            </Switch>
          </div>
        </div>
      </Match>
      <Match when={!props.modVersion}>
        <div class="contents">
          <div class="animate-pulse py-3">
            <div class="bg-darkSlate-600 mb-1 h-4 w-3/4 rounded" />
            <div class="flex gap-1">
              <div class="bg-darkSlate-600 h-3 w-12 rounded" />
              <div class="bg-darkSlate-600 h-3 w-12 rounded" />
            </div>
          </div>
          <div class="bg-darkSlate-600 animate-pulse h-4 rounded" />
          <div class="bg-darkSlate-600 animate-pulse h-4 rounded" />
          <div class="bg-darkSlate-600 animate-pulse h-4 rounded" />
          <div class="bg-darkSlate-600 animate-pulse h-8 w-20 rounded" />
        </div>
      </Match>
    </Switch>
  )
}

export default RowContainer
