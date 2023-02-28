import { useMatch } from "@solidjs/router";
import { settingsItem } from ".";
import { useGDNavigate } from "@/managers/NavigationManager";

interface Props {
  item: settingsItem;
}

const ListItem = (props: Props) => {
  const navigate = useGDNavigate();
  const match = useMatch(() => props.item.path);

  return (
    <div
      class="w-full cursor-pointer py-2 hover:bg-shade-6 pl-2"
      classList={{
        "bg-shade-6": !!match(),
      }}
      onClick={() => {
        navigate(props.item.path);
      }}
    >
      {props.item.name}
    </div>
  );
};

export default ListItem;
