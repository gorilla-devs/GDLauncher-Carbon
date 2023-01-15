import { createNotification, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [addNotification] = createNotification();
  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-black-lightGray"
      />
      <button onClick={() => addNotification("Notification Added")}>
        Add Notification
      </button>
      Sidebar library
    </SiderbarWrapper>
  );
};

export default Sidebar;
