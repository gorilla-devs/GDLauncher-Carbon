import { useMatch, useNavigate } from "@solidjs/router";
import { settingsItem } from ".";

interface Props {
  item: settingsItem;
}

const ListItem = (props: Props) => {
  const navigate = useNavigate();
  const match = useMatch(() => props.item.path);

  return (
    <div
      class="w-full py-2 cursor-pointer hover:bg-shade-6 pl-2"
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
