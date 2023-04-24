import { createResource } from "solid-js";
import Tile from "../Instance/Tile";
import { fetchImage, isListInstanceValid } from "@/utils/instances";
import { ListInstance, UngroupedInstance } from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";

const InstanceTile = (props: {
  instance: UngroupedInstance | ListInstance;
  isSidebarOpened?: boolean;
}) => {
  const [imageResource] = createResource(() => props.instance.id, fetchImage);
  const navigate = useGDNavigate();

  const validInstance = () =>
    isListInstanceValid(props.instance.status)
      ? props.instance.status.Valid
      : null;

  const modloader = validInstance()?.modloader;

  const image = () => imageResource();
  const variant = () => (props.isSidebarOpened ? "sidebar" : "sidebar-small");
  const type = () =>
    props.isSidebarOpened === undefined ? undefined : variant();

  return (
    <div>
      <Tile
        onClick={() => navigate(`/library/${props.instance.id}`)}
        title={props.instance.name}
        modloader={modloader}
        version={validInstance()?.mc_version}
        invalid={!isListInstanceValid(props.instance.status)}
        variant={type()}
        img={image()}
      />
    </div>
  );
};

export default InstanceTile;
