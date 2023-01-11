import { settingsItem } from ".";

interface Props {
  item: settingsItem;
}

const ListItem = (props: Props) => {
  return <div class="w-full py-2">{props.item.name}</div>;
};

export default ListItem;
