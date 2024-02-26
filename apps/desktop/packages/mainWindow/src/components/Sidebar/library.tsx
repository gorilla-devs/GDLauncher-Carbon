import { Collapsable, Input, Skeleton } from "@gd/ui";
import SiderbarWrapper from "./wrapper";
import {
  For,
  Match,
  Show,
  Suspense,
  Switch,
  createEffect,
  createMemo,
  createSignal
} from "solid-js";
import {
  getCFModloaderIcon,
  isSidebarOpened,
  toggleSidebar
} from "@/utils/sidebar";
import { useLocation, useRouteData } from "@solidjs/router";
import { getInstanceIdFromPath, setLastInstanceOpened } from "@/utils/routes";
import { Trans, useTransContext } from "@gd/i18n";
import fetchData from "@/pages/Library/library.data";
import { createStore, reconcile } from "solid-js/store";
import { InstancesStore } from "@/utils/instances";
import InstanceTile from "../InstanceTile";
import skull from "/assets/images/icons/skull.png";
import { CFFEModLoaderType } from "@gd/core_module/bindings";

const Sidebar = () => {
  const location = useLocation();
  const [t] = useTransContext();

  const instanceId = () => getInstanceIdFromPath(location.pathname);
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const [instances, setInstances] = createStore<InstancesStore>({});
  const [filter, setFilter] = createSignal("");

  createEffect(() => {
    setLastInstanceOpened(instanceId() || "");
  });

  const filteredData = createMemo(() =>
    filter()
      ? instances.data?.filter((item) =>
          item.name.toLowerCase().includes(filter().toLowerCase())
        )
      : instances.data
  );

  createEffect(() => {
    setInstances(reconcile({}));

    if (filteredData()) {
      filteredData()?.forEach((instance) => {
        const validInstance =
          instance.status.status === "valid"
            ? instance.status.value
            : undefined;

        const modloader = validInstance?.modloader || "vanilla";
        if (modloader) {
          setInstances(modloader, (prev) => {
            const filteredPrev = (prev || []).filter(
              (prev) => prev.id !== instance.id
            );
            if (!instance.favorite) return [...filteredPrev, instance];
            else return [...filteredPrev];
          });
        }
      });
    }
  });

  let inputRef: HTMLInputElement | undefined;
  const favoriteInstances = () =>
    instances.data?.filter((inst) => inst.favorite) || [];

  const mapIconToKey = (key: string) => {
    return (
      <Switch>
        <Match when={isSidebarOpened()}>{t(key)}</Match>
        <Match when={!isSidebarOpened()}>
          <img
            class="w-6 h-6"
            src={getCFModloaderIcon(key as CFFEModLoaderType)}
          />
        </Match>
      </Switch>
    );
  };
  return (
    <SiderbarWrapper noPadding>
      <div class="h-full w-full box-border transition-all flex flex-col gap-2 pt-5 pb-5">
        <div class="px-6 max-w-[190px] mt-[calc(2.5rem-1.25rem)] mb-3">
          <Show
            when={isSidebarOpened()}
            fallback={
              <div
                class="flex justify-center items-center cursor-pointer rounded-full bg-darkSlate-700 group w-10 h-10"
                onClick={() => {
                  toggleSidebar();
                  inputRef?.focus();
                }}
              >
                <div class="duration-100 ease-in-out transition text-darkSlate-500 i-ri:search-line group-hover:text-darkSlate-50" />
              </div>
            }
          >
            <Input
              ref={inputRef}
              placeholder={t("general.search")}
              icon={<div class="i-ri:search-line" />}
              class="w-full rounded-full"
              onInput={(e) => setFilter(e.target.value)}
              disabled={(instances?.data || []).length === 0}
            />
          </Show>
        </div>
        <Show when={instances.isInitialLoading}>
          <Skeleton.sidebarInstances />
        </Show>
        <div
          class="h-full box-border overflow-y-auto"
          classList={{
            "scrollbar-hide": !isSidebarOpened()
          }}
        >
          <Show when={favoriteInstances().length > 0}>
            <Collapsable
              title={
                isSidebarOpened() ? (
                  t("favorite")
                ) : (
                  <div class="w-6 h-6 text-yellow-500 i-ri:star-s-fill" />
                )
              }
              size={isSidebarOpened() ? "standard" : "small"}
            >
              <For each={favoriteInstances()}>
                {(instance) => (
                  <Suspense
                    fallback={
                      isSidebarOpened() ? (
                        <Show when={routeData.instances.isLoading}>
                          <Skeleton.sidebarInstance />
                        </Show>
                      ) : (
                        <Show when={instances.isLoading}>
                          <Skeleton.sidebarInstanceSmall />
                        </Show>
                      )
                    }
                  >
                    <InstanceTile
                      size={2}
                      instance={instance}
                      isSidebarOpened={isSidebarOpened()}
                      selected={instanceId() === instance.id.toString()}
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
                title={mapIconToKey(key)}
                size={isSidebarOpened() ? "standard" : "small"}
              >
                <For each={values}>
                  {(instance) => (
                    <Suspense
                      fallback={
                        isSidebarOpened() ? (
                          <Show when={instances.isLoading}>
                            <Skeleton.sidebarInstance />
                          </Show>
                        ) : (
                          <Show when={instances.isLoading}>
                            <Skeleton.sidebarInstanceSmall />
                          </Show>
                        )
                      }
                    >
                      <InstanceTile
                        size={2}
                        instance={instance}
                        isSidebarOpened={isSidebarOpened()}
                        selected={instanceId() === instance.id.toString()}
                      />
                    </Suspense>
                  )}
                </For>
              </Collapsable>
            )}
          </For>
          <Show
            when={(instances?.data || []).length === 0 && !instances.isLoading}
          >
            <div class="w-full h-full flex flex-col justify-center items-center">
              <img
                src={skull}
                classList={{
                  "w-16 h-16": isSidebarOpened(),
                  "w-10 h-10": !isSidebarOpened()
                }}
              />
              <Show when={isSidebarOpened()}>
                <p class="text-darkSlate-50 text-center text-xs max-w-50">
                  <Trans
                    key="instance.no_instances_text"
                    options={{
                      defaultValue:
                        "At the moment there are not instances. Add one to start playing!"
                    }}
                  />
                </p>
              </Show>
            </div>
          </Show>
        </div>
        <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-darkSlate-700 h-20 pointer-events-none" />
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
