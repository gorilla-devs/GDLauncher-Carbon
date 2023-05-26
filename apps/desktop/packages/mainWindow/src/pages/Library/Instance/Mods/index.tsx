import { Button, Checkbox, Dropdown, Input } from "@gd/ui";
import { For, Show } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import { ModloaderType } from "@/utils/sidebar";
import Mod from "./Mod";
import glassBlock from "/assets/images/icons/glassBlock.png";

type ModType = {
  title: string;
  enabled: boolean;
  modloader: ModloaderType;
  mcversion: string;
  modloaderVersion: string;
};

const mods: ModType[] = [
  {
    title: "Mods1",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods2",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods3",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods4",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods5",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods6",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods7",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods8",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods9",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods8",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods9",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods8",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods9",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods8",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "Mods9",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
];

const NoMods = () => {
  return (
    <div class="h-full w-full flex justify-center items-center min-h-90">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.no_mods_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
            }}
          />
        </p>
        <Button type="outline" size="medium">
          <Trans
            key="instance.add_pack"
            options={{
              defaultValue: "+ Add pack",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

const Mods = () => {
  const [t] = useTransContext();
  return (
    <div>
      <div class="flex flex-col bg-darkSlate-800 z-10 transition-all duration-100 ease-in-out pt-10 sticky top-30">
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
          <Button type="outline" size="medium">
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
              <Checkbox checked={true} disabled={false} />
              <Trans
                key="instance.select_all_mods"
                options={{
                  defaultValue: "Select All",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer transition duration-100 ease-in-out hover:text-white">
              <span class="text-2xl i-ri:folder-open-fill" />
              <Trans
                key="instance.open_mods_folder"
                options={{
                  defaultValue: "Open folder",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out">
              <span class="text-2xl i-ri:forbid-line" />
              <Trans
                key="instance.disable_mod"
                options={{
                  defaultValue: "disable",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out">
              <span class="text-2xl i-ri:delete-bin-2-fill" />
              <Trans
                key="instance.delete_mod"
                options={{
                  defaultValue: "delete",
                }}
              />
            </div>
          </div>
          <div>
            {mods.length}
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
        <Show when={mods.length > 0} fallback={<NoMods />}>
          <For each={mods}>{(props) => <Mod mod={props} />}</For>
        </Show>
      </div>
    </div>
  );
};

export default Mods;
