import { Button, Checkbox, Dropdown, Input } from "@gd/ui";
import { For, Show } from "solid-js";
import Mod from "./Mod";
import glassBlock from "/assets/images/icons/glassBlock.png";

const mods = [
  {
    title: "modpack1",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "modpack2",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
  {
    title: "modpack3",
    enabled: true,
    modloader: "forge",
    mcversion: "1.19.2",
    modloaderVersion: "2.1.3",
  },
];

const NoMods = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-shade-0 max-w-100">
          At the moment this modpack does not contain resource packs, but you
          can add packs yourself from your folder
        </p>
        <Button type="outline" size="medium">
          + Add pack
        </Button>
      </div>
    </div>
  );
};

const Mods = () => {
  return (
    <div>
      <div class="flex justify-between items-center pb-4 flex-wrap gap-1">
        <Input
          placeholder="Type Here"
          icon={<div class="i-ri:search-line" />}
          class="w-full rounded-full text-shade-0"
          inputClass=""
        />
        <div class="flex gap-3 items-center">
          <p class="text-shade-0">Sort by: </p>
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
          + Add Mod
        </Button>
      </div>
      <div class="flex justify-between text-shade-0 mb-6">
        <div class="flex gap-4">
          <div class="flex items-center gap-2 cursor-pointer">
            <Checkbox checked={true} disabled={false} />
            Select All
          </div>
          <div class="flex items-center gap-2 cursor-pointer">
            <span class="i-ri:folder-open-fill text-2xl" />
            Open sources
          </div>
          <div class="flex items-center gap-2 cursor-pointer">
            <span class="i-ri:forbid-line text-2xl" />
            Disable
          </div>
          <div class="flex items-center gap-2 cursor-pointer">
            <span class="i-ri:delete-bin-2-fill text-2xl" />
            Delete
          </div>
        </div>
        <div>173 Resource packs</div>
      </div>
      <div class="h-full">
        <Show when={mods.length > 0} fallback={<NoMods />}>
          <div>
            <For each={mods}>{(props) => <Mod mod={props} />}</For>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default Mods;
