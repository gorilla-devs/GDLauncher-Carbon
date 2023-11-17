import { Button, Checkbox, Input, Skeleton } from "@gd/ui";
import { For, Show, batch, createMemo, createSignal } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import Mod from "./Mod";
import skull from "/assets/images/icons/skull.png";
import { useParams, useRouteData } from "@solidjs/router";
import { rspc } from "@/utils/rspcClient";
import { createStore, produce, reconcile } from "solid-js/store";
import fetchData from "../../instance.data";
import { Mod as Modtype } from "@gd/core_module/bindings";
import { useGDNavigate } from "@/managers/NavigationManager";

const Mods = () => {
  const [t] = useTransContext();
  const params = useParams();
  const navigate = useGDNavigate();

  const [filter, setFilter] = createSignal("");
  const [selectedMods, setSelectedMods] = createStore<{
    [id: string]: boolean;
  }>({});
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const deleteModMutation = rspc.createMutation(["instance.deleteMod"]);
  const disableModMutation = rspc.createMutation(["instance.disableMod"]);
  const enableModMutation = rspc.createMutation(["instance.enableMod"]);

  const openFolderMutation = rspc.createMutation([
    "instance.openInstanceFolder"
  ]);

  const filteredMods = createMemo(() =>
    filter()
      ? routeData.instanceMods.data?.filter((item) =>
          item.filename.toLowerCase().includes(filter().toLowerCase())
        )
      : routeData.instanceMods.data
  );

  const NoMods = () => {
    return (
      <div class="h-full w-full flex justify-center items-center min-h-90">
        <div class="flex flex-col justify-center items-center text-center">
          <img src={skull} class="w-16 h-16" />
          <p class="text-darkSlate-50 max-w-100">
            <Trans key="instance.no_mods_text" />
          </p>
          <Button
            type="outline"
            size="medium"
            onClick={() => {
              navigate(`/mods?instanceId=${params.id}`);
            }}
          >
            <Trans key="instance.add_mod" />
          </Button>
        </div>
      </div>
    );
  };

  const sortAlphabetically = (a: Modtype, b: Modtype) => {
    if (a.filename < b.filename) return -1;
    if (a.filename > b.filename) return 1;
    return 0;
  };

  const isSelectAllIndeterminate = () => {
    return (
      Object.keys(selectedMods).length > 0 &&
      Object.keys(selectedMods).length !== routeData.instanceMods.data?.length
    );
  };

  return (
    <div>
      <div
        class="flex items-center fixed justify-between bottom-4 h-16 bg-darkSlate-900 mx-auto left-1/2 -translate-x-1/2 rounded-md pr-6 z-50 shadow-md shadow-darkSlate-900 transition-transform duration-100 ease-in-out origin-left w-130 border-darkSlate-700 border-solid border-1"
        classList={{
          "translate-y-24": Object.keys(selectedMods).length === 0
        }}
      >
        <div class="flex items-center h-full">
          <div
            class="flex items-center text-darkSlate-50 hover:text-white h-full px-6 mr-2"
            onClick={() => setSelectedMods(reconcile({}))}
          >
            <div class="i-ri:close-fill text-2xl" />
          </div>
          <div class="text-darkSlate-50">
            <Trans
              key="instance_selected_mods_count"
              options={{
                total: routeData.instanceMods.data?.length,
                selected: Object.keys(selectedMods).length
              }}
            />
          </div>
        </div>
        <div
          class="flex items-center gap-2 cursor-pointer text-darkSlate-50 hover:text-red-500 transition duration-100 ease-in-out"
          onClick={() => {
            Object.keys(selectedMods).forEach((mod) => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: mod
              });
            });
          }}
        >
          <span class="text-2xl i-ri:delete-bin-2-fill" />
          <Trans key="instance.delete_mod" />
        </div>
      </div>

      <div class="flex flex-col bg-darkSlate-800 transition-all duration-100 ease-in-out z-10 sticky top-14 px-6">
        <div class="flex justify-between items-center gap-1 pb-4 flex-wrap">
          <div class="flex items-center gap-4 cursor-pointer">
            <Checkbox
              indeterminate={isSelectAllIndeterminate()}
              checked={Object.keys(selectedMods).length > 0}
              onChange={(checked) => {
                let action = checked;

                if (isSelectAllIndeterminate()) {
                  action = true;
                }

                setSelectedMods(
                  produce((prev) => {
                    for (const mod of routeData.instanceMods.data || []) {
                      prev[mod.id] = action || undefined!;
                    }

                    return prev;
                  })
                );
              }}
            />
            <Input
              onInput={(e) => setFilter(e.target.value)}
              placeholder={t("instance.mods.search")}
              icon={<div class="i-ri:search-line" />}
              class="text-darkSlate-50 rounded-full"
            />
          </div>
          <div class="flex items-center gap-3">
            {/* <p class="text-darkSlate-50">
              <Trans key="instance.sort_by" />
            </p>
            <Dropdown
              options={[
                { label: t("instance.sort_by_asc"), key: "asc" },
                { label: t("instance.sort_by_desc"), key: "desc" }
              ]}
              value={"asc"}
              rounded
            /> */}
            <div
              class="flex items-center gap-2 cursor-pointer duration-100 ease-in-out transition hover:text-white text-darkSlate-50"
              onClick={() => {
                openFolderMutation.mutate({
                  folder: "Mods",
                  instance_id: parseInt(params.id, 10)
                });
              }}
            >
              <span class="text-2xl i-ri:filter-line" />
            </div>
            <Button
              type="outline"
              size="medium"
              onClick={() => {
                navigate(`/mods?instanceId=${params.id}`);
              }}
            >
              <Trans key="instance.add_mod" />
            </Button>
            <div
              class="flex items-center gap-2 cursor-pointer duration-100 ease-in-out transition hover:text-white text-darkSlate-50"
              onClick={() => {
                openFolderMutation.mutate({
                  folder: "Mods",
                  instance_id: parseInt(params.id, 10)
                });
              }}
            >
              <span class="text-2xl i-ri:folder-open-fill" />
            </div>
          </div>
        </div>
      </div>
      <div class="h-full w-full overflow-y-hidden pb-14">
        <Show
          when={
            routeData.instanceMods.data &&
            routeData.instanceMods.data?.length > 0 &&
            !routeData.instanceDetails.isLoading
          }
          fallback={<NoMods />}
        >
          <For
            each={(filteredMods() || []).sort(sortAlphabetically) as Modtype[]}
          >
            {(props) => (
              <Mod
                mod={props}
                setSelectedMods={setSelectedMods}
                selectMods={selectedMods}
              />
            )}
          </For>
        </Show>
        <Show when={routeData.instanceDetails.isLoading}>
          <Skeleton.sidebarInstances />
        </Show>
      </div>
    </div>
  );
};

export default Mods;
