import { Button, Collapsable, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { For, Show, createEffect } from "solid-js";
import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import Tile from "../Instance/Tile";
import { useLocation, useRouteData } from "@solidjs/router";
import { getInstanceIdFromPath, setLastInstanceOpened } from "@/utils/routes";
import { Trans, useTransContext } from "@gd/i18n";
import { useGDNavigate } from "@/managers/NavigationManager";
import fetchData from "@/pages/Library/library.data";
import { createStore, produce } from "solid-js/store";
import {
  InvalidListInstance,
  UngroupedInstance,
  ValidListInstance,
} from "@gd/core_module/bindings";
import {
  InvalidInstanceType,
  ValidInstanceType,
  isListInstanceValid,
} from "@/utils/instances";

export interface InstancesStore {
  [modloader: string]: (InvalidInstanceType | ValidInstanceType)[];
}

const Sidebar = () => {
  const [instances, setInstances] = createStore<InstancesStore>({});
  const navigate = useGDNavigate();
  const location = useLocation();

  const [t] = useTransContext();

  const instanceId = () => getInstanceIdFromPath(location.pathname);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    setLastInstanceOpened(instanceId() || "");
  });

  createEffect(() => {
    if (routeData.instancesUngrouped.data) {
      routeData.instancesUngrouped.data.forEach((instance) => {
        const validInstance = isListInstanceValid(instance.status)
          ? instance.status.Valid
          : null;
        const InvalidInstance = !isListInstanceValid(instance.status)
          ? instance.status.Invalid
          : null;
        const modloader = validInstance?.modloader;

        if (validInstance && modloader) {
          const mappedInstance: InvalidInstanceType | ValidInstanceType = {
            id: instance.id,
            name: instance.name,
            favorite: instance.favorite,
            ...(validInstance && { mc_version: validInstance.mc_version }),
            ...(validInstance && {
              modpack_platform: validInstance.modpack_platform,
            }),
            ...(validInstance && { img: "" }),
            ...(validInstance && { modloader }),
            ...(InvalidInstance && { error: InvalidInstance }),
          };

          setInstances(
            produce((prev) => {
              prev[modloader] = [...(prev[modloader] || []), mappedInstance];
              return prev;
            })
          );
        }
      });
    }
  });

  return (
    <SiderbarWrapper noPadding>
      <div class="h-full w-full box-border pt-5 pb-5">
        <div class="px-3 max-w-[190px] mt-[calc(2.5rem-1.25rem)] mb-3">
          <Show
            when={isSidebarOpened()}
            fallback={
              <div
                class="flex justify-center items-center group w-10 h-10 rounded-full bg-darkSlate-700"
                onClick={() => {
                  toggleSidebar();
                }}
              >
                <div class="transition duration-100 ease-in-out i-ri:search-line text-darkSlate-500 group-hover:text-darkSlate-50" />
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
        <Show when={Object.entries(instances).length > 0}>
          <div class="mt-4">
            <For each={Object.entries(instances)}>
              {([key, values]) => (
                <Collapsable
                  title={key}
                  size={isSidebarOpened() ? "standard" : "small"}
                >
                  <For each={values}>
                    {(instance) => (
                      <Tile
                        onClick={() => navigate(`/library/${instance.id}`)}
                        title={instance.name}
                        modloader={
                          "modloader" in instance ? instance?.modloader : null
                        }
                        version={
                          "mc_version" in instance ? instance?.mc_version : null
                        }
                        invalid={"error" in instance}
                        variant={
                          isSidebarOpened() ? "sidebar" : "sidebar-small"
                        }
                        img={"mc_version" in instance ? instance?.img : null}
                      />
                    )}
                  </For>
                </Collapsable>
              )}
            </For>
          </div>
        </Show>
        <div class="absolute left-0 right-0 bottom-0 w-full flex justify-center bg-darkSlate-800 py-5">
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
