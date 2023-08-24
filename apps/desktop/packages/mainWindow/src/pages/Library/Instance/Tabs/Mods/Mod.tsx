import { getInstanceIdFromPath } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { getForgeModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Switch } from "@gd/ui";
import { useLocation, useParams } from "@solidjs/router";
import { SetStoreFunction } from "solid-js/store";
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

  const [imageResource] = createResource(
    () => [params.id, props.mod.id] as const,
    ([instanceId, modId]) => fetchModImage(instanceId, modId)
  );

  const isCurseForge = () => props.mod.curseforge;

  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <Checkbox
            checked={props.selectMods[props.mod.id]}
            onChange={(e) => {
              props.setSelectedMods(props.mod.id, e);
            }}
          />
          <div class="flex items-center gap-2">
            <div class="h-10 w-10 rounded-xl border border-solid border-darkSlate-500 overflow-hidden flex items-center justify-center">
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
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer transition-color hover:text-red-500 i-ri:delete-bin-2-fill"
            onClick={() => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: props.mod.id,
              });
            }}
          />
          <Switch
            checked={props.mod.enabled}
            onChange={(e) => {
              if (instanceId() === undefined) return;
              if (e.target.checked) {
                enableModMutation.mutate({
                  instance_id: parseInt(instanceId() as string, 10),
                  mod_id: props.mod.id,
                });
              } else {
                disableModMutation.mutate({
                  instance_id: parseInt(instanceId() as string, 10),
                  mod_id: props.mod.id,
                });
              }
            }}
          />
        </span>
      </div>
    </div>
  );
};

export default Mod;
