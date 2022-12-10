import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-black-lightGray"
      />
      Sidebar library
    </SiderbarWrapper>
  );
};

export default Sidebar;
