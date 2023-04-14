import { Button, Collapsable, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { Show, createEffect } from "solid-js";
import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import Tile from "../Instance/Tile";
import { useLocation } from "@solidjs/router";
import { getInstanceIdFromPath, setLastInstanceOpened } from "@/utils/routes";
import { Trans, useTransContext } from "@gd/i18n";
import { useGDNavigate } from "@/managers/NavigationManager";

const Sidebar = () => {
  const navigate = useGDNavigate();
  const location = useLocation();

  const [t] = useTransContext();

  const instanceId = () => getInstanceIdFromPath(location.pathname);

  createEffect(() => {
    setLastInstanceOpened(instanceId() || "");
  });

  // TODO: adapt to real data
  return (
    <SiderbarWrapper noPadding>
      <div class="h-full w-full box-border pt-5 pb-5">
        <div class="px-3 max-w-[190px] mt-[calc(2.5rem-1.25rem)] mb-3">
          <Show
            when={isSidebarOpened()}
            fallback={
              <div
                class="flex justify-center items-center h-10 group w-10 bg-shade-7 rounded-full"
                onClick={() => {
                  toggleSidebar();
                }}
              >
                <div class="transition duration-100 ease-in-out text-shade-5 i-ri:search-line group-hover:text-shade-0" />
              </div>
            }
          >
            <Input
              placeholder={t("general.type_here") || ""}
              icon={<div class="i-ri:search-line" />}
              class="w-full rounded-full"
            />
          </Show>
        </div>
        <Show when={isSidebarOpened()}>
          <Collapsable title="VANILLA">
            <Tile
              isLoading={true}
              percentage={50}
              title={"Instance"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar"
            />
            <Tile
              onClick={() => navigate(`/library/ABDFEAD`)}
              selected={instanceId() === "ABDFEAD"}
              title={"Instance ABDFEAD"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar"
            />
          </Collapsable>
          <Collapsable title="FAVOURITED">
            <Tile
              onClick={() => navigate(`/library/DDAEDF`)}
              selected={instanceId() === "DDAEDF"}
              title={"Instance DDAEDF"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar"
            />
          </Collapsable>
          <Collapsable title="CURSEFORGE">
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar"
            />
          </Collapsable>
        </Show>
        <Show when={!isSidebarOpened()}>
          <div class="h-full w-full flex gap-4 items-center overflow-auto flex-col scrollbar-hide max-h-[calc(100vh-60px-28px-80px-80px)] mt-6">
            <Tile
              // onClick={() => navigate(`/library/${instance.id}`)}
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
            <Tile
              title={"InstanceName"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar-small"
            />
          </div>
        </Show>
        <div class="absolute right-0 w-full flex justify-center left-0 bottom-0 bg-shade-8 py-5">
          <Button
            variant="outline"
            onClick={() => navigate(`/modpacks`)}
            style={{
              ...(isSidebarOpened()
                ? { width: "100%", "max-width": "200px" }
                : { width: "40px", height: "40px", padding: "16px" }),
            }}
          >
            <Show when={isSidebarOpened()} fallback={"+"}>
              <Trans
                key="sidebar.plus_add_instance"
                options={{
                  defaultValue: "+ Add Instance",
                }}
              />
            </Show>
          </Button>
        </div>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
