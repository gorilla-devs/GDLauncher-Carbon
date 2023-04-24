import { Button, Collapsable, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import { For, Show, Suspense, createEffect } from "solid-js";
import { isSidebarOpened, toggleSidebar } from "@/utils/sidebar";
import { useLocation, useRouteData } from "@solidjs/router";
import { getInstanceIdFromPath, setLastInstanceOpened } from "@/utils/routes";
import { Trans, useTransContext } from "@gd/i18n";
import fetchData from "@/pages/Library/library.data";
import { createStore, produce } from "solid-js/store";
import { InstancesStore, isListInstanceValid } from "@/utils/instances";
import { useModal } from "@/managers/ModalsManager";
import { ListInstance } from "@gd/core_module/bindings";
import InstanceTile from "../InstanceTile";

const Sidebar = () => {
  const location = useLocation();

  const [t] = useTransContext();
  const modalsContext = useModal();

  const instanceId = () => getInstanceIdFromPath(location.pathname);
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [instances, setInstances] = createStore<InstancesStore>({});
  const [favoriteInstances, setFavoriteInstances] = createStore<ListInstance[]>(
    []
  );

  createEffect(() => {
    setLastInstanceOpened(instanceId() || "");
  });

  createEffect(() => {
    if (routeData.groups.data) {
      const favorites = routeData.groups.data.find(
        (group) => group.name === "localize➽favorites"
      )?.instances;

      if (favorites) {
        favorites.forEach((instance) => {
          setFavoriteInstances((prev) => [...prev, instance]);
        });
      }
    }
  });

  createEffect(() => {
    if (routeData.instancesUngrouped.data) {
      routeData.instancesUngrouped.data.forEach((instance) => {
        const validInstance = isListInstanceValid(instance.status)
          ? instance.status.Valid
          : null;

        const modloader = validInstance?.modloader;

        if (modloader) {
          setInstances(
            produce((prev) => {
              prev[modloader] = [...(prev[modloader] || []), instance];
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
          <div
            class="mt-4 overflow-y-auto h-[calc(100%-84px-40px)]"
            classList={{
              "scrollbar-hide": !isSidebarOpened(),
            }}
          >
            <Show when={favoriteInstances.length > 0}>
              <Collapsable
                title={"Favorites"}
                size={isSidebarOpened() ? "standard" : "small"}
              >
                <For each={favoriteInstances}>
                  {(instance) => (
                    //TODO: SKELETON
                    <Suspense fallback={<></>}>
                      <InstanceTile
                        instance={instance}
                        isSidebarOpened={isSidebarOpened()}
                      />
                    </Suspense>
                  )}
                </For>
              </Collapsable>
            </Show>
            <For
              each={Object.entries(instances).filter(
                (group) => group[1].length > 0
              )}
            >
              {([key, values]) => (
                <Collapsable
                  title={key}
                  size={isSidebarOpened() ? "standard" : "small"}
                >
                  <For each={values}>
                    {(instance) => (
                      //TODO: SKELETON
                      <Suspense fallback={<></>}>
                        <InstanceTile
                          instance={instance}
                          isSidebarOpened={isSidebarOpened()}
                        />
                      </Suspense>
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
            onClick={() => {
              modalsContext?.openModal({
                name: "instanceCreation",
                url: "/modpacks",
              });
            }}
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
