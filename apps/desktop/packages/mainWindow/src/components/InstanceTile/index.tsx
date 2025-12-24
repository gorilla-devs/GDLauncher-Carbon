import { createEffect, createSignal, createMemo } from "solid-js"
import Tile from "../Instance/Tile"
import {
  getPreparingState,
  getRunningState,
  getInactiveState,
  getInstanceImageUrl,
  isInstanceDeleting
} from "@/utils/instances"
import { ListInstance, FESubtask } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import { createStore } from "solid-js/store"
import { bytesToMB } from "@/utils/helpers"
import { useGlobalStore } from "../GlobalStoreContext"

interface InstanceDownloadProgress {
  totalDownload: number
  downloaded: number
  percentage: number
  subTasks: FESubtask[] | undefined
}

export const [clickedInstanceId, setClickedInstanceId] = createSignal<
  string | undefined
>(undefined)

const InstanceTile = (props: {
  instance: ListInstance
  isSidebarOpened?: boolean
  identifier: string
  selected?: boolean
  size: 1 | 2 | 3 | 4 | 5
}) => {
  const [isLoading, setIsLoading] = createSignal(false)
  const [failError, setFailError] = createSignal("")
  const [progress, setProgress] = createStore<InstanceDownloadProgress>({
    totalDownload: 0,
    downloaded: 0,
    percentage: 0,
    subTasks: undefined
  })

  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  const validInstance = () =>
    props.instance.status.status === "valid"
      ? props.instance.status.value
      : undefined

  const invalidInstance = () =>
    props.instance.status.status === "invalid"
      ? props.instance.status.value
      : undefined

  const inactiveState = () => getInactiveState(validInstance()?.state)
  const isPreparingState = () => getPreparingState(validInstance()?.state)
  const isDeleting = () => isInstanceDeleting(validInstance()?.state)

  const modloader = () => validInstance()?.modloader

  const taskId = () => isPreparingState()

  const isRunning = () => getRunningState(validInstance()?.state)
  const dismissTaskMutation = rspc.createMutation(() => ({
    mutationKey: ["vtask.dismissTask"]
  }))

  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId() || null]
  }))

  createEffect(() => {
    setFailError("")

    if (task?.data) {
      const data = task.data
      setProgress("totalDownload", data.download_total)
      setProgress("downloaded", data.downloaded)
      if (data.progress.type === "Known") {
        setProgress("subTasks", data.active_subtasks)
        setProgress("percentage", data.progress.value)
        setIsLoading(true)
      } else if (data.progress.type === "Failed") {
        setIsLoading(false)
      } else {
        setIsLoading(false)
      }
    } else {
      setIsLoading(false)
      setProgress({
        totalDownload: 0,
        downloaded: 0,
        percentage: 0,
        subTasks: undefined
      })
    }
  })

  createEffect(() => {
    if ((validInstance() || invalidInstance()) && taskId === undefined) {
      dismissTaskMutation.mutate(taskId)
    }
  })

  const failedTask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", inactiveState()!],
    enabled: false
  }))

  createEffect(() => {
    if (inactiveState() !== null && inactiveState() !== undefined) {
      failedTask.refetch()
    }
  })

  createEffect(() => {
    if (failedTask.data && failedTask.data.progress.type === "Failed") {
      if (taskId()) dismissTaskMutation.mutate(taskId()!)
      setFailError(failedTask.data.progress.value.cause[0].display)
    }
  })

  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small")
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant()

  const instanceImageUrl = createMemo(() =>
    props.instance.icon_revision
      ? getInstanceImageUrl(props.instance.id, props.instance.icon_revision)
      : undefined
  )

  // Only show NEW badge on completely installed, valid instances
  const isNew = () => {
    // Instance must be valid
    if (props.instance.status.status !== "valid") return false

    // Instance must be in inactive state (not preparing, running, or deleting)
    const state = validInstance()?.state
    if (!state || state.state !== "inactive") return false

    // Instance must not have a failed task (installation didn't error)
    if (inactiveState()) return false

    return globalStore.isNewInstance(props.instance.id)
  }

  return (
    <Tile
      onClick={() => {
        globalStore.markInstanceAsSeen(props.instance.id)
        setClickedInstanceId(props.identifier)

        requestAnimationFrame(() => {
          navigator.navigate(`/library/${props.instance.id}`)
        })
      }}
      onHover={() => globalStore.markInstanceAsSeen(props.instance.id)}
      isNew={isNew()}
      shouldSetViewTransition={clickedInstanceId() === props.identifier}
      identifier={props.identifier}
      instance={props.instance}
      modloader={modloader()}
      version={validInstance()?.mc_version}
      isInvalid={props.instance.status.status === "invalid"}
      failError={failError()}
      isRunning={!!isRunning()}
      isPreparing={isPreparingState() !== undefined}
      isDeleting={isDeleting()}
      variant={type()}
      size={props.size}
      img={instanceImageUrl()}
      selected={props.selected}
      isLoading={isLoading()}
      percentage={progress.percentage * 100}
      totalDownload={bytesToMB(progress.totalDownload)}
      downloaded={bytesToMB(progress.downloaded)}
      subTasks={progress.subTasks}
    />
  )
}

export default InstanceTile
