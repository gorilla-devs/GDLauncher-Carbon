import { createEffect, createResource, createSignal } from "solid-js";
import Tile from "../Instance/Tile";
import {
  fetchImage,
  isListInstanceValid,
  isProgressKnown,
  isProgressFailed,
  getValideInstance,
  getPreparingState,
} from "@/utils/instances";
import { ListInstance, UngroupedInstance } from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import { bytesToMB } from "@/utils/helpers";

type InstanceDownloadProgress = {
  totalDownload: number;
  downloaded: number;
  percentage: number;
};

const InstanceTile = (props: {
  instance: UngroupedInstance | ListInstance;
  isSidebarOpened?: boolean;
  selected?: boolean;
}) => {
  const [isLoading, setIsLoading] = createSignal(false);
  const [progress, setProgress] = createStore<InstanceDownloadProgress>({
    totalDownload: 0,
    downloaded: 0,
    percentage: 0,
  });
  const [imageResource] = createResource(() => props.instance.id, fetchImage);
  const navigate = useGDNavigate();

  const validInstance = () => getValideInstance(props.instance.status);

  const isPreparingState = () => getPreparingState(props.instance.status);

  const modloader = validInstance()?.modloader;

  const taskId = isPreparingState();

  if (taskId !== undefined) {
    const task = rspc.createQuery(() => ["vtask.getTask", taskId]);

    createEffect(() => {
      if (task.data) {
        setProgress("totalDownload", task.data.download_total);
        setProgress("downloaded", task.data.downloaded);
        if (isProgressKnown(task.data.progress)) {
          setProgress("percentage", task.data.progress.Known);
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
        type={type()}
        img={image()}
        selected={props.selected}
        isLoading={isLoading()}
        percentage={progress.percentage * 100}
        totalDownload={bytesToMB(progress.totalDownload)}
        downloaded={bytesToMB(progress.downloaded)}
      />
    </div>
  );
};

export default InstanceTile;
