import { createEffect, createSignal } from "solid-js";
import Tile from "../Instance/Tile";
import {
  isListInstanceValid,
  getValideInstance,
  getPreparingState,
  getRunningState,
  getInValideInstance,
  getInactiveState,
  getInstanceImageUrl,
  getDeletingState
} from "@/utils/instances";
import { ListInstance, FESubtask, FETask } from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import { bytesToMB } from "@/utils/helpers";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";

type InstanceDownloadProgress = {
  totalDownload: number;
  downloaded: number;
  percentage: number;
  subTasks: FESubtask[] | undefined;
};

const InstanceTile = (props: {
  instance: ListInstance;
  isSidebarOpened?: boolean;
  selected?: boolean;
  size: 1 | 2 | 3 | 4 | 5;
}) => {
  const [isLoading, setIsLoading] = createSignal(false);
  const [failError, setFailError] = createSignal("");
  const [progress, setProgress] = createStore<InstanceDownloadProgress>({
    totalDownload: 0,
    downloaded: 0,
    percentage: 0,
    subTasks: undefined
  });

  const navigate = useGDNavigate();

  const validInstance = () => getValideInstance(props.instance.status);
  const invalidInstance = () => getInValideInstance(props.instance.status);
  const inactiveState = () => getInactiveState(props.instance.status);
  const isPreparingState = () => getPreparingState(props.instance.status);
  const isDeleting = () => getDeletingState(props.instance.status) as boolean;

  const modloader = () => validInstance()?.modloader;

  const taskId = () => isPreparingState();

  const isRunning = () => getRunningState(props.instance.status);
  const dismissTaskMutation = rspc.createMutation(["vtask.dismissTask"]);

  const [task, setTask] = createSignal<CreateQueryResult<
    FETask | null,
    RSPCError
  > | null>(null);

  createEffect(() => {
    if (taskId() !== undefined) {
      setTask(rspc.createQuery(() => ["vtask.getTask", taskId() as number]));
    }
  });

  createEffect(() => {
    setFailError("");
    if (task() !== null && task()?.data) {
      const data = (task() as CreateQueryResult<FETask | null, RSPCError>)
        .data as FETask;
      setProgress("totalDownload", data.download_total);
      setProgress("downloaded", data.downloaded);
      if (data.progress.type === "Known") {
        setProgress("subTasks", data.active_subtasks);
        setProgress("percentage", data.progress.value);
        setIsLoading(true);
      } else if (data.progress.type === "Failed") {
        setIsLoading(false);
      } else {
        setIsLoading(false);
      }
    }
  });

  createEffect(() => {
    if ((validInstance() || invalidInstance()) && taskId === undefined) {
      dismissTaskMutation.mutate(taskId);
    }
  });

  const failedTask = rspc.createQuery(
    () => ["vtask.getTask", inactiveState() as number],
    { enabled: false }
  );

  createEffect(() => {
    if (inactiveState() !== null && inactiveState() !== undefined) {
      failedTask.refetch();
    }
  });

  createEffect(() => {
    if (failedTask.data && failedTask.data.progress.type === "Failed") {
      if (taskId()) dismissTaskMutation.mutate(taskId() as number);
      setFailError(failedTask.data.progress.value.cause[0].display);
    }
  });

  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small");
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant();

  return (
    <Tile
      onClick={() => navigate(`/library/${props.instance.id}`)}
      instance={props.instance}
      modloader={modloader()}
      version={validInstance()?.mc_version}
      isInvalid={!isListInstanceValid(props.instance.status)}
      failError={failError()}
      isRunning={!!isRunning()}
      isPreparing={isPreparingState() !== undefined}
      isDeleting={isDeleting()}
      variant={type()}
      size={props.size}
      img={
        props.instance.icon_revision
          ? getInstanceImageUrl(props.instance.id, props.instance.icon_revision)
          : undefined
      }
      selected={props.selected}
      isLoading={isLoading()}
      percentage={progress.percentage * 100}
      totalDownload={bytesToMB(progress.totalDownload)}
      downloaded={bytesToMB(progress.downloaded)}
      subTasks={progress.subTasks}
    />
  );
};

export default InstanceTile;
