import { getInstanceIdFromPath } from "@/utils/routes";
import { queryClient, rspc } from "@/utils/rspcClient";
import { getForgeModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Popover, Switch } from "@gd/ui";
import { useLocation, useParams } from "@solidjs/router";
import { SetStoreFunction, produce } from "solid-js/store";
import { For, Show, createResource } from "solid-js";
import { getModImageUrl, getModpackPlatformIcon } from "@/utils/instances";

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
  const params = useParams();

  const location = useLocation();

  const instanceId = () => getInstanceIdFromPath(location.pathname);

  const enableModMutation = rspc.createMutation(["instance.enableMod"], {
    onMutate: (data) => {
      console.log(data.mod_id);

      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: true
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    }
  });

  const disableModMutation = rspc.createMutation(["instance.disableMod"], {
    onMutate: (data) => {
      console.log(data.mod_id);
      queryClient.setQueryData(
        ["instance.getInstanceMods", data.instance_id],
        (oldData: ModType[] | undefined) => {
          const modIndex = oldData?.findIndex((mod) => mod.id === data.mod_id)!;
          return [
            ...oldData!.slice(0, modIndex),
            {
              ...oldData![modIndex],
              enabled: false
            },
            ...oldData!.slice(modIndex + 1)
          ];
        }
      );
    }
  });

  const deleteModMutation = rspc.createMutation(["instance.deleteMod"]);

  const imagePlatform = () => {
    if (props.mod.curseforge?.has_image) return "curseforge";
    else if (props.mod.modrinth?.has_image) return "modrinth";
    else if (props.mod.metadata?.has_image) return "metadata";
    else return null;
  };

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
                when={
                  props.mod.curseforge?.has_image ||
                  props.mod.modrinth?.has_image ||
                  props.mod.metadata?.has_image
                }
                fallback={
                  <img
                    class="w-full"
                    src={getModpackPlatformIcon(
                      isCurseForge() ? "Curseforge" : "Modrinth"
                    )}
                  />
                }
              >
                <img
                  class="w-full h-full"
                  src={getModImageUrl(params.id, props.mod.id, imagePlatform())}
                />
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
          <Show when={props.mod.curseforge}>CF</Show>
          <Show when={props.mod.modrinth}>MR</Show>
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
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:delete-bin-2-fill transition-color hover:text-red-500"
            onClick={() => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: props.mod.id
              });
            }}
          />

          <Popover
            noPadding
            noTip
            content={
              <div
                onClick={(e) => e.stopPropagation()}
                class="w-40 h-40 bg-red"
              >
                Hi
              </div>
            }
            placement="left-end"
            color="bg-darkSlate-900"
          >
            <div class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:information-fill transition-color hover:text-white" />
          </Popover>
        </span>
      </div>
    </div>
  );
};

export default Mod;
