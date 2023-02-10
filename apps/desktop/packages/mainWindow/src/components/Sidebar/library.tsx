/* eslint-disable i18next/no-literal-string */
import { Button, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { Show } from "solid-js";
import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import Collapsable from "./collapsable";
import Tile from "../Instance/Tile";
import Style from "./style.module.scss";
import { useLocation, useNavigate } from "@solidjs/router";
import { getTabIndex, routesLastTab } from "@/utils/routes";

const Sidebar = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const instanceTab = () =>
    location.pathname.match(/^.*\/library\/([^\\/]+)\/([^\\/]+)(?=\/*$)/);

  // TODO: adapt to real data
  return (
    <SiderbarWrapper noPadding>
      <div class="h-full w-full pt-5 pb-5 box-border">
        <div class="max-w-[190px] mt-[calc(2.5rem-1.25rem)] mb-3 px-3">
          <Show
            when={isSidebarOpened()}
            fallback={
              <div
                class="group w-10 h-10 bg-shade-7 rounded-full flex justify-center items-center"
                onClick={() => {
                  toggleSidebar();
                }}
              >
                <div class="i-ri:search-line text-shade-5 group-hover:text-shade-0 transition duration-100 ease-in-out" />
              </div>
            }
          >
            <Input
              placeholder="Type Here"
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
              onClick={() =>
                navigate(
                  `/library/ABDFEAD/${getTabIndex(routesLastTab["ABDFEAD"])}`
                )
              }
              selected={instanceTab()?.[1] === "ABDFEAD"}
              title={"Instance ABDFEAD"}
              modloader={"forge"}
              version={"1.19.2"}
              variant="sidebar"
            />
          </Collapsable>
          <Collapsable title="FAVOURITED">
            <Tile
              onClick={() =>
                navigate(
                  `/library/DDAEDF/${getTabIndex(routesLastTab["DDAEDF"])}`
                )
              }
              selected={instanceTab()?.[1] === "DDAEDF"}
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
          <div
            class={`${Style.scrollbarHide} h-full w-full max-h-[calc(100vh-60px-28px-80px-80px)] overflow-auto flex flex-col gap-4 items-center mt-6`}
          >
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
        <div class="absolute left-0 right-0 bottom-0 py-5 w-full flex justify-center bg-shade-8">
          <Button
            variant="outline"
            style={{
              ...(isSidebarOpened()
                ? { width: "100%", "max-width": "200px" }
                : { width: "40px", height: "40px", padding: "16px" }),
            }}
          >
            <Show when={isSidebarOpened()} fallback={"+"}>
              + Add Instance
            </Show>
          </Button>
        </div>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
