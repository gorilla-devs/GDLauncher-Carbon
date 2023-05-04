import { createEffect, createResource, createSignal } from "solid-js";
import Tile from "../Instance/Tile";
import {
  fetchImage,
  getLaunchState,
  isListInstanceValid,
  isPreparing,
} from "@/utils/instances";
import {
  ListInstance,
  Progress,
  UngroupedInstance,
  VisualTaskId,
} from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";

const InstanceTile = (props: {
  instance: UngroupedInstance | ListInstance;
  isSidebarOpened?: boolean;
  selected?: boolean;
}) => {
  const [progress, setProgress] = createSignal<Progress>();
  const [imageResource] = createResource(() => props.instance.id, fetchImage);
  const navigate = useGDNavigate();

  const validInstance = () =>
    isListInstanceValid(props.instance.status)
      ? props.instance.status.Valid
      : null;

  const isPreparingState = () =>
    isListInstanceValid(props.instance.status) &&
    isPreparing(props.instance.status.Valid.state)
      ? (getLaunchState(props.instance.status.Valid.state) as {
          Preparing: VisualTaskId;
        })
      : null;

  const modloader = validInstance()?.modloader;

  const taskId = isPreparingState()?.Preparing;

  createEffect(() => {
    console.log("taskId", taskId);
  });

  if (taskId !== undefined) {
    const task = rspc.createQuery(() => ["vtask.getTask", taskId]);

    createEffect(() => {
      console.log("task", task.data, taskId);
      if (task.data) {
        setProgress(task.data.progress);
      }
    });
  }

  const image = () => imageResource();
  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small");
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant();

  createEffect(() => {
    console.log("instance", props.instance);
  });

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
      />
    </div>
  );
};

export default InstanceTile;
