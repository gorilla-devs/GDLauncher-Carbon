import { getInstanceIdFromPath } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { getForgeModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Switch } from "@gd/ui";
import { useLocation, useParams } from "@solidjs/router";
import { SetStoreFunction, produce } from "solid-js/store";
import { For, Show, createResource } from "solid-js";
import { fetchModImage, getModpackPlatformIcon } from "@/utils/instances";

type Props = {
  mod: ModType;
  setSelectedMods: SetStoreFunction<{
    [id: string]: boolean;
  }>;
  selectMods: {
    [id: string]: boolean;
  };
};

const Mod = (props: Props) => {
  const enableModMutation = rspc.createMutation(["instance.enableMod"]);
  const disableModMutation = rspc.createMutation(["instance.disableMod"]);
  const deleteModMutation = rspc.createMutation(["instance.deleteMod"]);
  const params = useParams();

  const location = useLocation();

  const instanceId = () => getInstanceIdFromPath(location.pathname);

  const imagePlatform = () => {
    if (props.mod.curseforge?.has_image) return "curseforge";
    else if (props.mod.modrinth?.has_image) return "modrinth";
    else if (props.mod.metadata?.has_image) return "metadata";
    else return null;
  };

  const [imageResource] = createResource(
    () => [params.id, props.mod.id, imagePlatform()] as const,
    ([instanceId, modId, platform]) =>
      fetchModImage(instanceId, modId, platform)
  );

  const isCurseForge = () => props.mod.curseforge;

  return (
    <div
      class="w-full flex items-center py-2 px-6 box-border h-14 group hover:bg-darkSlate-700"
      onClick={() => {
        props.setSelectedMods(
          produce((draft) => {
            if (!draft[props.mod.id]) draft[props.mod.id] = true;
            else delete draft[props.mod.id];
          })
        );
      }}
    >
      <div class="flex justify-between items-center w-full gap-4">
        <div class="flex gap-4 justify-between items-center">
          <div
            class="opacity-0 group-hover:opacity-100 transition-opacity duration-100 ease-in-out"
            classList={{
              "opacity-100": props.selectMods[props.mod.id]
            }}
          >
            <Checkbox checked={props.selectMods[props.mod.id]} />
          </div>
          <div class="flex items-center gap-2">
            <div class="flex items-center justify-center h-10 w-10 rounded-xl border-solid overflow-hidden border-darkSlate-500 border">
              <Show
                when={imageResource()}
                fallback={
                  <img
                    class="w-full"
                    src={getModpackPlatformIcon(
                      isCurseForge() ? "Curseforge" : "Modrinth"
                    )}
                  />
                }
              >
                <img class="w-full h-full" src={imageResource()} />
              </Show>
            </div>
            <div class="flex flex-col">
              {props.mod.curseforge?.name ||
                props.mod.metadata?.name ||
                props.mod.filename}
              <div class="flex gap-2">
                <For each={props.mod.metadata?.modloaders}>
                  {(modloader, _) => (
                    <span class="flex gap-2 justify-center items-center">
                      <Show when={modloader}>
                        <img
                          class="w-4 h-4"
                          src={getForgeModloaderIcon(modloader)}
                        />
                      </Show>
                      <p class="m-0 text-darkSlate-500 text-sm">
                        {`${modloader}`}
                      </p>
                    </span>
                  )}
                </For>
                <p class="m-0 text-darkSlate-500 text-sm">
                  {`${props.mod.metadata?.modloaders[0]} ${
                    props.mod.curseforge?.project_id ||
                    props.mod.metadata?.version
                  }`}
                </p>
              </div>
            </div>
          </div>
        </div>
        <span class="flex gap-4 justify-center items-center">
          {/* //TODO: ADD CONFIRMATION MODAL */}
          <Switch
            checked={props.mod.enabled}
            onChange={(e) => {
              if (instanceId() === undefined) return;
              if (e.target.checked) {
                enableModMutation.mutate({
                  instance_id: parseInt(instanceId() as string, 10),
                  mod_id: props.mod.id
                });
              } else {
                disableModMutation.mutate({
                  instance_id: parseInt(instanceId() as string, 10),
                  mod_id: props.mod.id
                });
              }
            }}
          />
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer transition-color hover:text-white i-ri:arrow-left-right-fill"
            onClick={() => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: props.mod.id
              });
            }}
          />
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:delete-bin-2-fill transition-color hover:text-red-500"
            onClick={() => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: props.mod.id
              });
            }}
          />
        </span>
      </div>
    </div>
  );
};

export default Mod;
