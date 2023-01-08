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
      <button
        onClick={() => {
          console.log("Notification Added");
          addNotification("Notification Added");
        }}
      >
        ADD
      </button>
      Sidebar library
    </SiderbarWrapper>
  );
};

export default Sidebar;
