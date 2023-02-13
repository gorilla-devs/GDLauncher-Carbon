/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox } from "@gd/ui";
import Collapsable from "./Collapsable";

const Sidebar = () => {
  return (
    <SiderbarWrapper collapsable={false}>
      <div class="h-full w-full pt-5 pb-5 box-border">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <div class="flex items-center gap-3">
              <Checkbox checked={true} disabled={false} />
              <div class="flex items-center gap-2">
                <img class="w-4 h-4" src={getModloaderIcon("vanilla")} />
                <p class="m-0">Vanilla</p>
              </div>
            </div>
            <div class="flex items-center gap-3">
              <Checkbox checked={true} disabled={false} />
              <div class="flex items-center gap-2">
                <img class="w-4 h-4" src={getModloaderIcon("forge")} />
                <p class="m-0">Forge</p>
              </div>
            </div>
          </div>
        </Collapsable>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
