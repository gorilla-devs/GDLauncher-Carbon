import { createEffect, createResource, createSignal } from "solid-js";
import Tile from "../Instance/Tile";
import {
  fetchImage,
  getLaunchState,
  isListInstanceValid,
  isInstancePreparing,
  isProgressKnown,
  isProgressFailed,
} from "@/utils/instances";
import {
  ListInstance,
  TaskId,
  UngroupedInstance,
} from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";

const InstanceTile = (props: {
  instance: UngroupedInstance | ListInstance;
  isSidebarOpened?: boolean;
  selected?: boolean;
}) => {
  const [progress, setProgress] = createSignal(0);
  const [isLoading, setIsLoading] = createSignal(false);
  const [imageResource] = createResource(() => props.instance.id, fetchImage);
  const navigate = useGDNavigate();

  const validInstance = () =>
    isListInstanceValid(props.instance.status)
      ? props.instance.status.Valid
      : null;

  const isPreparingState = () =>
    isListInstanceValid(props.instance.status) &&
    isInstancePreparing(props.instance.status.Valid.state)
      ? (getLaunchState(props.instance.status.Valid.state) as {
          Preparing: TaskId;
        })
      : null;

  const modloader = validInstance()?.modloader;

  const taskId = isPreparingState()?.Preparing;

  if (taskId !== undefined) {
    const task = rspc.createQuery(() => ["vtask.getTask", taskId]);

    createEffect(() => {
      if (task.data) {
        if (isProgressKnown(task.data.progress)) {
          setProgress(task.data.progress.Known);
          setIsLoading(true);
        } else if (isProgressFailed(task.data.progress)) {
          setIsLoading(false);
        }
      }
    });
  }

  const image = () => imageResource();
  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small");
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant();

  return (
    <div>
      <Tile
        onClick={() => navigate(`/library/${props.instance.id}`)}
        title={props.instance.name}
        instanceId={props.instance.id}
        modloader={modloader}
        version={validInstance()?.mc_version}
        invalid={!isListInstanceValid(props.instance.status)}
        variant={type()}
        img={image()}
        selected={props.selected}
        isLoading={isLoading()}
        percentage={progress()}
      />
    </div>
  );
};

export default InstanceTile;
