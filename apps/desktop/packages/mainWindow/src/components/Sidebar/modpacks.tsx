/* eslint-disable i18next/no-literal-string */
import { getModloaderIcon } from "@/utils/sidebar";
import SiderbarWrapper from "./wrapper";
import { Checkbox, Collapsable, Radio } from "@gd/ui";
import fetchData from "@/pages/Modpacks/browser.data";
import { useRouteData } from "@solidjs/router";
import { For, Show } from "solid-js";
import { FECategory } from "@gd/core_module/bindings";
import { modLoader, setModloader } from "@/utils/modpackBrowser";

const Sidebar = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <SiderbarWrapper collapsable={false}>
      <div class="h-full w-full pt-5 pb-5 box-border overflow-y-auto">
        <Collapsable title="Modloader">
          <div class="flex flex-col gap-3">
            <Radio.group
              onChange={(val) => {
                setModloader(val as string);
              }}
              value={"any"}
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
            <Show when={routeData.forgeCategories.data?.data}>
              <For each={routeData.forgeCategories.data?.data as FECategory[]}>
                {(category) => (
                  <div class="flex items-center gap-3">
                    <Checkbox checked={true} disabled={false} />
                    <div class="flex items-center gap-2">
                      <img class="h-4 w-4" src={getModloaderIcon("vanilla")} />
                      <p class="m-0">{category.name}</p>
                    </div>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </Collapsable>
      </div>
    </SiderbarWrapper>
  );
};

export default Sidebar;
