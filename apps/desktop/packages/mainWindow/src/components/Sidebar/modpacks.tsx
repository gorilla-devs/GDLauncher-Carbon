/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Show, createEffect } from "solid-js";
import { deepTrack } from "@solid-primitives/deep";
import {
  modpacksCategories,
  modLoader,
  setModpacksCategories,
  setModloader,
} from "@/utils/modpackBrowser";
import { produce } from "solid-js/store";

const Sidebar = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (routeData.forgeCategories.data?.data) {
      routeData.forgeCategories.data?.data.forEach((category) => {
        setModpacksCategories((prev) => [
          ...prev,
          { ...category, selected: false },
        ]);
      });
    }
  });

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <div class="h-full w-full py-5 px-4 box-border overflow-y-auto">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                setModloader(val as string);
              }}
              value={modLoader()}
            >
              <Radio name="modloader" value="any">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("vanilla")} />
                  <p class="m-0">Vanilla</p>
                </div>
              </Radio>
              <Radio name="modloader" value="forge">
                <div class="flex items-center gap-2">
                  <img class="h-4 w-4" src={getModloaderIcon("forge")} />
                  <p class="m-0">Forge</p>
                </div>
              </Radio>
            </Radio.group>
          </div>
        </Collapsable>
        <Collapsable title="Categories">
          <div class="flex flex-col gap-3">
            <Show when={modpacksCategories.length > 0}>
              <For each={deepTrack(modpacksCategories)}>
                {(category) => {
                  const selected = () => category.selected;
                  return (
                    <div class="flex items-center gap-3">
                      <Checkbox
                        checked={selected()}
                        onChange={(e) => {
                          setModpacksCategories(
                            (prev) => prev.id === category.id,
                            produce((prev) => {
                              prev.selected = !e;
                            })
                          );
                        }}
                      />
                      <div class="flex items-center gap-2 max-w-32">
                        <img
                          class="h-4 w-4"
                          src={getModloaderIcon("vanilla")}
                        />
                        <p class="m-0">{category.name}</p>
                      </div>
                    </div>
                  );
                }}
              </For>
            </Show>
          </div>
        </Collapsable>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
