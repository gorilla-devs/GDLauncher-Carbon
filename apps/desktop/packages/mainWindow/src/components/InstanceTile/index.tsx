import { createEffect, createResource, createSignal } from "solid-js";
import Tile from "../Instance/Tile";
import {
  fetchImage,
  isListInstanceValid,
  isProgressKnown,
  isProgressFailed,
  getValideInstance,
  getPreparingState,
  getRunningState,
  getInValideInstance,
  getInactiveState,
} from "@/utils/instances";
import {
  ListInstance,
  UngroupedInstance,
  Subtask,
  Task,
} from "@gd/core_module/bindings";
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
  subTasks: Subtask[] | undefined;
};

const InstanceTile = (props: {
  instance: UngroupedInstance | ListInstance;
  isSidebarOpened?: boolean;
  selected?: boolean;
}) => {
  const [isLoading, setIsLoading] = createSignal(false);
  const [failError, setFailError] = createSignal("");
  const [progress, setProgress] = createStore<InstanceDownloadProgress>({
    totalDownload: 0,
    downloaded: 0,
    percentage: 0,
    subTasks: undefined,
  });
  const [imageResource] = createResource(() => props.instance.id, fetchImage);
  const navigate = useGDNavigate();

  const validInstance = () => getValideInstance(props.instance.status);
  const invalidInstance = () => getInValideInstance(props.instance.status);
  const inactiveState = () => getInactiveState(props.instance.status);
  const failedTaskId = () => inactiveState();
  const isPreparingState = () => getPreparingState(props.instance.status);

  const modloader = () => validInstance()?.modloader;

  const taskId = () => isPreparingState();

  const isRunning = () => getRunningState(props.instance.status);
  const dismissTaskMutation = rspc.createMutation(["vtask.dismissTask"]);

  // const task = useConditionalQuery("vtask.getTask", taskId());

  const [task, setTask] = createSignal<CreateQueryResult<
    Task | null,
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
      const data = (task() as CreateQueryResult<Task | null, RSPCError>)
        .data as Task;
      setProgress("totalDownload", data.download_total);
      setProgress("downloaded", data.downloaded);
      if (isProgressKnown(data.progress)) {
        setProgress("subTasks", data.active_subtasks);
        setProgress("percentage", data.progress.Known);
        setIsLoading(true);
      } else if (isProgressFailed(data.progress)) {
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
    () => ["vtask.getTask", failedTaskId() as number],
    { enabled: false }
  );

  createEffect(() => {
    if (failedTaskId() !== null && failedTaskId() !== undefined) {
      failedTask.refetch();
    }
  });

  createEffect(() => {
    if (failedTask.data && isProgressFailed(failedTask.data.progress)) {
      setFailError(failedTask.data.progress.Failed.cause[0].display);
    }
  });

  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small");
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant();

  return (
    <Tile
      onClick={() => navigate(`/library/${props.instance.id}`)}
      title={props.instance.name}
      instanceId={props.instance.id}
      modloader={modloader()}
      version={validInstance()?.mc_version}
      isInvalid={!isListInstanceValid(props.instance.status)}
      failError={failError()}
      isRunning={!!isRunning()}
      isPreparing={isPreparingState() !== undefined}
      variant={type()}
      img={imageResource()}
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
