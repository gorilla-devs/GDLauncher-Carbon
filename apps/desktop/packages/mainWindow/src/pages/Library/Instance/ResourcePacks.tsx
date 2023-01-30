/* eslint-disable i18next/no-literal-string */
import { Button, Checkbox, Dropdown, Input } from "@gd/ui";
import { For, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import ResourcePack from "./ResourcePack";
import glassBlock from "/assets/images/icons/glassBlock.png";

interface IResourcepack {
  title: string;
  enabled: boolean;
  mcversion: string;
  modloaderVersion: string;
  resourcePackVersion: string;
}

const resourcePacks: IResourcepack[] = [
  {
    title: "Mods1",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods2",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.15",
  },
  {
    title: "Mods3",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.15",
  },
  {
    title: "Mods4",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.14",
  },
  {
    title: "Mods5",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods6",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods7",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",

    resourcePackVersion: "1.17",
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods8",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
  {
    title: "Mods9",
    enabled: true,
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
    resourcePackVersion: "1.17",
  },
];

const NoResourcePacks = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-shade-0 max-w-100">
          <Trans
            key="no_resourcepacks_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
            }}
          />
        </p>
        <Button variant="outline" size="medium">
          <Trans
            key="add_pack"
            options={{
              defaultValue: "+ Add pack",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

const ResourcePacks = () => {
  return (
    <div>
      <div
        id="inline-inner-container"
        class="flex flex-col sticky top-35 bg-shade-8 z-20 pt-10 transition-all duration-100 ease-in-out"
      >
        <div class="flex justify-between items-center pb-4 flex-wrap gap-1">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full rounded-full text-shade-0"
            inputClass=""
          />
          <div class="flex gap-3 items-center">
            <p class="text-shade-0">
              <Trans
                key="sort_by"
                options={{
                  defaultValue: "Sort by:",
                }}
              />
            </p>
            <Dropdown
              options={[
                { label: "A to Z", key: "asc" },
                { label: "Z to A", key: "desc" },
              ]}
              value={"asc"}
              rounded
            />
          </div>
          <Button variant="outline" size="medium">
            <Trans
              key="add_pack"
              options={{
                defaultValue: "+ Add ResourcePack",
              }}
            />
          </Button>
        </div>
        <div class="flex justify-between text-shade-0 mb-6 z-10">
          <div class="flex gap-4">
            <div class="flex items-center gap-2 cursor-pointer">
              <Checkbox checked={true} disabled={false} />
              <Trans
                key="select_all"
                options={{
                  defaultValue: "Select All",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out">
              <span class="i-ri:folder-open-fill text-2xl" />
              <Trans
                key="open_folder"
                options={{
                  defaultValue: "Open folder",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out">
              <span class="i-ri:forbid-line text-2xl" />
              <Trans
                key="disable"
                options={{
                  defaultValue: "disable",
                }}
              />
            </div>
            <div class="flex items-center gap-2 cursor-pointer hover:text-white transition duration-100 ease-in-out">
              <span class="i-ri:delete-bin-2-fill text-2xl" />
              <Trans
                key="delete"
                options={{
                  defaultValue: "delete",
                }}
              />
            </div>
          </div>
          <div>{resourcePacks.length} Resource packs</div>
        </div>
      </div>
      <div class="h-full overflow-y-hidden">
        <Show when={resourcePacks.length > 0} fallback={<NoResourcePacks />}>
          <For each={resourcePacks}>
            {(props) => <ResourcePack resourcePack={props} />}
          </For>
        </Show>
      </div>
    </div>
  );
};

export default ResourcePacks;
