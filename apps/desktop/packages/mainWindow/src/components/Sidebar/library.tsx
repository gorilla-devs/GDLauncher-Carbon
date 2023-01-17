import { createNotification, Input, Switch } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [addNotification] = createNotification();
  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-shade-0"
      />
      <button onClick={() => addNotification("Notification Added")}>
        Add Notification
      </button>
      Sidebar library
      <Switch />
    </SiderbarWrapper>
  );
};

export default Sidebar;
