/* eslint-disable solid/no-innerhtml */
import {
  InvalidListInstance,
  LaunchState,
  ListInstanceStatus,
  FESubtask,
  ListInstance,
  ValidListInstance
} from "@gd/core_module/bindings"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { Switch, Match, createSignal } from "solid-js"
import { sanitizeSvgIcon } from "@/utils/modplatformDescriptionConverter"
import { apiUrl } from "./rspcClient"

export const isListInstanceInvalid = (status: ListInstanceStatus) => {
  return "Invalid" in status
}

export const getLaunchState = (launchState: LaunchState | undefined) => {
  if (!launchState) return undefined

  if (
    launchState.state === "queued" ||
    launchState.state === "preparing" ||
    launchState.state === "running"
  ) {
    return launchState.value
  }
  return undefined
}

export const getQueuedState = (status: LaunchState | undefined) => {
  if (!status) return undefined

  if (status.state === "queued") {
    return status.value
  }
}

export const getPreparingState = (status: LaunchState | undefined) => {
  if (!status) return undefined

  if (status.state === "preparing") {
    return status.value
  }
}

export const getInactiveState = (status: LaunchState | undefined) => {
  if (!status) return undefined

  if (status.state === "inactive") {
    return status.value.failed_task
  }
}

export const isSubTaskDownload = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "download" in input
}

export const isSubTaskItem = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "item" in input
}

export const getRunningState = (status: LaunchState | undefined) => {
  if (!status) return undefined

  if (status.state === "running") {
    return status.value
  }
}

export const isInstanceDeleting = (status: LaunchState | undefined) => {
  if (!status) return false

  return status.state === "deleting"
}

export const isInstanceQueued = (launchState: LaunchState) => {
  return launchState.state === "queued"
}

export const isInstancePreparing = (launchState: LaunchState) => {
  return launchState.state === "preparing"
}

export const isInstanceRunning = (launchState: LaunchState) => {
  return launchState.state === "running"
}

export const getInstanceImageUrl = (
  instanceId: string | number,
  rev: string | number
) => {
  return apiUrl(`/instance/instanceIcon?id=${instanceId}&rev=${rev}`)
}

export const getServerImageUrl = (
  serverId: string | number,
  rev: string | number
) => {
  return apiUrl(`/server/serverIcon?id=${serverId}&rev=${rev}`)
}

export const getModImageUrl = (
  instanceId: string,
  modId: string,
  platform: string | null
) => {
  return apiUrl(
    `/instance/modIcon?instance_id=${instanceId}&mod_id=${modId}&platform=${platform}`
  )
}

export const getServerModImageUrl = (
  serverId: string,
  modId: string,
  platform: string
) => {
  return apiUrl(
    `/server/serverModIcon?server_id=${serverId}&mod_id=${modId}&platform=${platform}`
  )
}

export const getUrlType = (url: string) => {
  return /^\/(modpacks|mods)\/[a-zA-Z0-9]+\/(curseforge|modrinth)(\/[^/]+)*$/.exec(
    url
  )
    ? /mods/.exec(url)
      ? "mods"
      : "modpacks"
    : null
}

export interface InvalidInstanceType extends Omit<ListInstance, "status"> {
  error?: InvalidListInstance
}

export interface ValidInstanceType extends ValidListInstance, ListInstance {
  error?: undefined
  img: string
}

export type Instance = InvalidInstanceType | ValidInstanceType

export type InstancesStore = Record<string, ListInstance[]>

export const getModpackPlatformIcon = (
  modpackType: "curseforge" | "modrinth" | null | undefined
) => {
  switch (modpackType) {
    case "curseforge":
      return CurseforgeLogo
    case "modrinth":
      return ModrinthLogo
    default:
      return CurseforgeLogo
  }
}

export const CategoryIcon = (props: {
  type: "embedded" | "url" | undefined
  value: string | undefined
}) => {
  return (
    <Switch>
      <Match when={props.type === "embedded"}>
        <div class="h-4 w-4" innerHTML={sanitizeSvgIcon(props.value)} />
      </Match>
      <Match when={props.type === "url"}>
        <img class="h-4 w-4" src={props.value} />
      </Match>
    </Switch>
  )
}

export const PlatformIcon = (props: { modpack: "curseforge" | "modrinth" }) => {
  return <img class="h-4 w-4" src={getModpackPlatformIcon(props.modpack)} />
}

export const [importedInstances, setImportedInstances] = createSignal<number[]>(
  []
)
