import {
  FEUnifiedSearchResultWithDescription,
  FEUnifiedSearchType,
  InstanceDetails,
  Mod
} from "@gd/core_module/bindings"
import { VersionRowTypeData } from "../InfiniteScrollVersionsQueryWrapper"
import { For, Match, Show, Switch, createSignal } from "solid-js"
import { Trans } from "@gd/i18n"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { format } from "date-fns"
import CopyIcon from "../CopyIcon"
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

const CopiableEntity = (props: {
  text: string | undefined | null | number
}) => {
  return (
    <div class="text-lightSlate-200 flex w-60 items-center">
      <div class="truncate">
        <Tooltip>
          <TooltipTrigger>{props.text || "-"}</TooltipTrigger>
          <TooltipContent>
            <div class="max-w-110 break-all">{props.text || "-"}</div>
          </TooltipContent>
        </Tooltip>
      </div>
      <Show when={props.text}>
        <div class="ml-2 shrink-0">
          <CopyIcon text={props.text} />
        </div>
      </Show>
    </div>
  )
}

const RowContainer = (props: Props & AdditionalProps) => {
  const [isHoveringInfoCard, setIsHoveringInfoCard] = createSignal(false)

  return (
    <Switch>
      <Match when={props.modVersion}>
        <div class="flex flex-col justify-center py-2">
          <h4 class="text-md m-0 pb-2 font-medium">
            {props.modVersion.name.replaceAll(".zip", "")}
          </h4>
          <div class="divide-darkSlate-500 text-lightGray-800 divide-x-1 flex gap-2 text-sm">
            <Trans key="explore_versions.tags" />
            <For each={props.modVersion.gameVersions}>
              {(version) => <div>{version}</div>}
            </For>
          </div>
        </div>
        <div class="flex items-center">
          {format(new Date(props.modVersion.datePublished), "dd-MM-yyyy")}
        </div>
        <div class="flex items-center">{props.modVersion.downloads}</div>
        <div
          class="flex items-center"
          classList={{
            "text-green-500":
              props.modVersion.releaseType === "stable" ||
              props.modVersion.releaseType === "release",
            "text-yellow-500": props.modVersion.releaseType === "beta",
            "text-red-500": props.modVersion.releaseType === "alpha"
          }}
        >
          {props.modVersion.releaseType}
        </div>
        <div class="flex items-center">
          <div onClick={(e) => e.stopPropagation()}>
            <Popover
              placement="left"
              onOpenChange={(open) => {
                if (open) setIsHoveringInfoCard(true)
                else setIsHoveringInfoCard(false)
              }}
            >
              <PopoverTrigger>
                <div
                  class="hover:text-lightSlate-50 text-lightSlate-700 i-ri:information-fill transition-color cursor-pointer text-2xl duration-100 ease-in-out"
                  classList={{
                    "text-lightSlate-50": isHoveringInfoCard()
                  }}
                />
              </PopoverTrigger>
              <PopoverContent class="border-none p-0">
                <div
                  class="bg-darkSlate-900 text-lightSlate-700 border-darkSlate-700 border-1 shadow-darkSlate-90 w-110 rounded-lg border-solid p-4 shadow-md"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div class="text-lightSlate-50 mb-4 text-xl font-bold">
                    <Trans
                      key="addons_versions.technical_info_for"
                      options={{
                        addon_name: props.modVersion.name
                      }}
                    >
                      {""}
                      <span class="italic">{""}</span>
                    </Trans>
                  </div>
                  <div class="flex w-full flex-col">
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.project_id" />
                      </div>
                      <CopiableEntity text={props.modVersion.id} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.file_id" />
                      </div>
                      <CopiableEntity text={props.modVersion.fileId} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.file_name" />
                      </div>
                      <CopiableEntity text={props.modVersion.fileName} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.file_size" />
                      </div>
                      <CopiableEntity text={props.modVersion.size} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.hash" />
                      </div>
                      <CopiableEntity text={props.modVersion.hash} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.status" />
                      </div>
                      <CopiableEntity text={props.modVersion.status} />
                    </div>
                    <div class="flex w-full justify-between text-sm">
                      <div class="w-50">
                        <Trans key="addons_versions.release_type" />
                      </div>
                      <CopiableEntity text={props.modVersion.releaseType} />
                    </div>
                  </div>
                </div>
              </PopoverContent>
            </Popover>
          </div>
        </div>
        <div class="flex items-center">
          <Switch>
            <Match when={props.type === "modpack"}>
              <ModpackDownloadButton
                addon={props.project}
                name={props.modVersion.name}
                fileId={
                  props.project?.platform === "curseforge"
                    ? parseInt(props.modVersion.fileId, 10)
                    : props.modVersion.fileId
                }
              />
            </Match>
            <Match when={props.type !== "modpack"}>
              <ModDownloadButton
                selectedInstanceId={props.instanceId ?? undefined}
                selectedInstanceMods={props.instanceMods}
                addon={props.project}
                fileId={
                  props.project?.platform === "curseforge"
                    ? parseInt(props.modVersion.fileId, 10)
                    : props.modVersion.fileId
                }
              />
            </Match>
          </Switch>
        </div>
      </Match>
      <Match when={!props.modVersion}>
        <Trans key="loading" />
      </Match>
    </Switch>
  )
}

export default RowContainer
