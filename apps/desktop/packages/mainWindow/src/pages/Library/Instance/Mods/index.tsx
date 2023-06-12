import { Button, Checkbox, Dropdown, Input, Skeleton } from "@gd/ui";
import { For, Show } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import Mod from "./Mod";
import skull from "/assets/images/icons/skull.png";
import { useParams, useRouteData } from "@solidjs/router";
import { rspc } from "@/utils/rspcClient";
import { useModal } from "@/managers/ModalsManager";
import { createStore } from "solid-js/store";
import fetchData from "../instance.data";

const Mods = () => {
  const [t] = useTransContext();
  const params = useParams();
  const modalsContext = useModal();
  const [selectedMods, setSelectedMods] = createStore<{
    [id: string]: boolean;
  }>({});
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const deleteModMutation = rspc.createMutation(["instance.deleteMod"]);
  const disableModMutation = rspc.createMutation(["instance.disableMod"]);
  const enableModMutation = rspc.createMutation(["instance.enableMod"]);

  const openFolderMutation = rspc.createMutation([
    "instance.openInstanceFolder",
  ]);

  const NoMods = () => {
    return (
      <div class="h-full w-full flex justify-center items-center min-h-90">
        <div class="flex flex-col justify-center items-center text-center">
          <img src={skull} class="w-16 h-16" />
          <p class="text-darkSlate-50 max-w-100">
            <Trans
              key="instance.no_mods_text"
              options={{
                defaultValue:
                  "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
              }}
            />
          </p>
          <Button
            type="outline"
            size="medium"
            onClick={() => {
              modalsContext?.openModal(
                { name: "addMod" },
                routeData.instanceDetails.data?.version
              );
            }}
          >
            <Trans
              key="instance.add_mod"
              options={{
                defaultValue: "+ Add Mod",
              }}
            />
          </Button>
        </div>
      </div>
    );
  };

  return (
    <div>
      <div class="flex flex-col bg-darkSlate-800 z-10 transition-all duration-100 ease-in-out sticky pt-10 top-30">
        <div class="flex justify-between items-center gap-1 pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full rounded-full text-darkSlate-50"
            inputClass=""
          />
          <div class="flex gap-3 items-center">
            <p class="text-darkSlate-50">
              <Trans
                key="instance.sort_by"
                options={{
                  defaultValue: "Sort by:",
                }}
              />
            </p>
            <Dropdown
              options={[
                { label: t("instance.sort_by_asc"), key: "asc" },
                { label: t("instance.sort_by_desc"), key: "desc" },
              ]}
              value={"asc"}
              rounded
            />
          </div>
          <Button
            type="outline"
            size="medium"
            onClick={() => {
              modalsContext?.openModal(
                { name: "addMod" },
                routeData.instanceDetails.data?.version
              );
            }}
          >
            <Trans
              key="instance.add_mod"
              options={{
                defaultValue: "+ Add Mod",
              }}
            />
          </Button>
        </div>
        <div class="flex justify-between text-darkSlate-50 z-10 mb-6">
          <div class="flex gap-4">
            <div class="flex items-center gap-2 cursor-pointer">
              <Checkbox
                onChange={(checked) => {
                  routeData.instanceDetails.data?.mods.forEach((mod) => {
                    if (checked) {
                      setSelectedMods((prev) => ({ ...prev, [mod.id]: true }));
                    } else
                      setSelectedMods((prev) => ({ ...prev, [mod.id]: false }));
                  });
                }}
              />
              <Trans
                key="instance.select_all_mods"
                options={{
                  defaultValue: "Select All",
                }}
              />
            </div>
            <div
              class="flex items-center gap-2 cursor-pointer transition duration-100 ease-in-out hover:text-white"
              onClick={() => {
                openFolderMutation.mutate({
                  folder: "Mods",
                  instance_id: parseInt(params.id, 10),
                });
              }}
            >
              <span class="text-2xl i-ri:folder-open-fill" />
              <Trans
                key="instance.open_mods_folder"
                options={{
                  defaultValue: "Open folder",
                }}
              />
            </div>

            <div
              class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out"
              onClick={() => {
                Object.keys(selectedMods).forEach((mod) => {
                  deleteModMutation.mutate({
                    instance_id: parseInt(params.id, 10),
                    mod_id: mod,
                  });
                });
              }}
            >
              <span class="text-2xl i-ri:delete-bin-2-fill" />
              <Trans
                key="instance.delete_mod"
                options={{
                  defaultValue: "delete",
                }}
              />
            </div>
            <Show when={Object.keys(selectedMods).length > 0}>
              <div
                class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out"
                onClick={() => {
                  const areSelectedEnabled =
                    routeData.instanceDetails.data?.mods
                      .filter((mod) => selectedMods[mod.id])
                      .every((mod) => mod.enabled);

                  routeData.instanceDetails.data?.mods
                    .filter((mod) => selectedMods[mod.id])
                    .forEach((mod) => {
                      if (areSelectedEnabled) {
                        disableModMutation.mutate({
                          instance_id: parseInt(params.id, 10),
                          mod_id: mod.id,
                        });
                      } else {
                        enableModMutation.mutate({
                          instance_id: parseInt(params.id, 10),
                          mod_id: mod.id,
                        });
                      }
                    });
                }}
              >
                <Show
                  when={routeData.instanceDetails.data?.mods
                    .filter((mod) => selectedMods[mod.id])
                    .every((mod) => mod.enabled)}
                  fallback={
                    <Trans
                      key="instance.enable_all_selected_mod"
                      options={{
                        defaultValue: "Enable selected",
                      }}
                    />
                  }
                >
                  <Trans
                    key="instance.disable_all_selected_mod"
                    options={{
                      defaultValue: "Disable selected",
                    }}
                  />
                </Show>
              </div>
            </Show>
          </div>
          <div class="flex gap-1">
            <span>{routeData.instanceDetails.data?.mods.length}</span>

            <Trans
              key="instance.mods"
              options={{
                defaultValue: "Mods",
              }}
            />
          </div>
        </div>
      </div>
      <div class="h-full overflow-y-hidden">
        <Show
          when={
            routeData.instanceDetails.data?.mods &&
            routeData.instanceDetails.data?.mods.length > 0 &&
            !routeData.instanceDetails.isLoading
          }
          fallback={<NoMods />}
        >
          <For each={routeData.instanceDetails.data?.mods}>
            {(props) => (
              <Mod
                mod={props}
                setSelectedMods={setSelectedMods}
                selectedMods={selectedMods}
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
