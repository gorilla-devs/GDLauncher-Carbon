import { Trans } from "@gd/i18n";
import { Button, Dropdown, Input } from "@gd/ui";
import { For, Show } from "solid-js";
import BG from "/assets/images/rlccraft_img.png";
import glassBlock from "/assets/images/icons/glassBlock.png";
import Modpack from "./Modpack";
import Tags from "./Tags";
import CurseforgeIcon from "/assets/images/icons/curseforge.png";

const modpacks = [
  {
    img: BG,
    name: "RLC Craft",
    tags: [
      { name: "curseforge", img: CurseforgeIcon },
      { name: "curseforge", img: CurseforgeIcon },
    ],
    description:
      "Minecraft Forge is a handy place to store, sort, and keep tabs on all your mods. If you’re after more inspiration, our guide to the best Minecraft shaders and Minecraft texture packs.",
    author: "ATMTeam",
    download: 1400000,
    lastUpdate: "2023-02-28T09:45:43.029Z",
  },
  {
    img: BG,
    name: "RLC Craft",
    tags: [
      { name: "curseforge", img: CurseforgeIcon },
      { name: "curseforge", img: CurseforgeIcon },
    ],
    description:
      "Minecraft Forge is a handy place to store, sort, and keep tabs on all your mods. If you’re after more inspiration, our guide to the best Minecraft shaders and Minecraft texture packs.",
    author: "ATMTeam",
    download: 1400000,
    lastUpdate: "2023-02-28T09:45:43.029Z",
  },
  {
    img: BG,
    name: "RLC Craft",
    tags: [
      { name: "curseforge", img: CurseforgeIcon },
      { name: "curseforge", img: CurseforgeIcon },
    ],
    description:
      "Minecraft Forge is a handy place to store, sort, and keep tabs on all your mods. If you’re after more inspiration, our guide to the best Minecraft shaders and Minecraft texture packs.",
    author: "ATMTeam",
    download: 1400000,
    lastUpdate: "2023-02-28T09:45:43.029Z",
  },
  {
    img: BG,
    name: "RLC Craft",
    tags: [
      { name: "curseforge", img: CurseforgeIcon },
      { name: "curseforge", img: CurseforgeIcon },
    ],
    description:
      "Minecraft Forge is a handy place to store, sort, and keep tabs on all your mods. If you’re after more inspiration, our guide to the best Minecraft shaders and Minecraft texture packs.",
    author: "ATMTeam",
    download: 1400000,
    lastUpdate: "2023-02-28T09:45:43.029Z",
  },
  {
    img: BG,
    name: "RLC Craft",
    tags: [
      { name: "curseforge", img: CurseforgeIcon },
      { name: "curseforge", img: CurseforgeIcon },
    ],
    description:
      "Minecraft Forge is a handy place to store, sort, and keep tabs on all your mods. If you’re after more inspiration, our guide to the best Minecraft shaders and Minecraft texture packs.",
    author: "ATMTeam",
    download: 1400000,
    lastUpdate: "2023-02-28T09:45:43.029Z",
  },
];

const NoModpacks = () => {
  return (
    <div class="h-full w-full flex justify-center items-center min-h-90">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-shade-0 max-w-100">
          <Trans
            key="no_modpacks_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
            }}
          />
        </p>
      </div>
    </div>
  );
};

export default function Browser() {
  return (
    <div class="relative w-full box-border">
      <div class="sticky top-0 left-0 right-0 flex flex-col bg-shade-8 z-10 px-5 pt-5">
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
                { label: "Popular", key: "popular" },
                { label: "Featured", key: "featured" },
                { label: "Author", key: "author" },
                { label: "Name", key: "name" },
                { label: "Total downloads", key: "downloads" },
              ]}
              value={"popular"}
              rounded
            />
          </div>
          <Button
            variant="outline"
            size="medium"
            icon={<div class="i-ri:download-2-fill text-lg" />}
          >
            <Trans
              key="import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button>
        </div>
        <div class="max-w-150 flex justify-between text-shade-0 z-10 mb-6">
          <Tags />
        </div>
      </div>
      <div class="overflow-y-hidden px-5 pb-5 flex flex-col gap-2">
        <Show when={modpacks.length > 0} fallback={<NoModpacks />}>
          <For each={modpacks}>{(props) => <Modpack modpack={props} />}</For>
        </Show>
      </div>
    </div>
  );
}
