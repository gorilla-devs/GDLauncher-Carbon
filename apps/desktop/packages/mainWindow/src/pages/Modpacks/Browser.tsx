import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input } from "@gd/ui";
import { For, Show } from "solid-js";
import BG from "/assets/images/rlccraft_img.png";
import glassBlock from "/assets/images/icons/glassBlock.png";
import Modpack from "./Modpack";
import Tags from "./Tags";
import CurseforgeIcon from "/assets/images/icons/curseforge.png";
import LogoDark from "/assets/images/logo-dark.svg";
import { useModal } from "@/managers/ModalsManager";

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
    <div class="flex justify-center items-center w-full h-full min-h-90">
      <div class="flex justify-center items-center flex-col text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.no_modpacks_text"
            options={{
              defaultValue: "At the moment there is no modpacks.",
            }}
          />
        </p>
      </div>
    </div>
  );
};

export default function Browser() {
  const modalsContext = useModal();
  const [t] = useTransContext();
  return (
    <div class="w-full relative box-border">
      <div class="flex flex-col left-0 right-0 sticky top-0 bg-darkSlate-800 z-10 px-5 pt-5">
        <div class="flex items-center gap-3 justify-between pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full text-darkSlate-50 rounded-full max-w-none flex-1"
            inputClass=""
          />
          <div class="flex items-center gap-3">
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
                { label: t("instance.sort_by_popular"), key: "popular" },
                { label: t("instance.sort_by_featured"), key: "featured" },
                { label: t("instance.sort_by_author"), key: "author" },
                { label: t("instance.sort_by_name"), key: "name" },
                {
                  label: t("instance.sort_by_total_downloads"),
                  key: "downloads",
                },
              ]}
              value={"popular"}
              rounded
            />
          </div>
          <Button
            variant="outline"
            size="medium"
            icon={<div class="rounded-full i-ri:download-2-fill text-md" />}
          >
            <Trans
              key="instance.import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button>
        </div>
        <div class="flex justify-between text-darkSlate-50 z-10 mb-6 max-w-150">
          <Tags />
        </div>
      </div>
      <div class="px-5 flex flex-col pb-5 gap-2 overflow-y-hidden">
        <div class="flex flex-col gap-4 rounded-xl p-5 bg-darkSlate-700">
          <div class="flex justify-between items-center">
            <span class="flex gap-4">
              <div class="flex justify-center items-center rounded-xl bg-darkSlate-900 h-22 w-22">
                <img class="h-14" src={LogoDark} />
              </div>
              <div class="flex flex-col justify-center">
                <div class="flex flex-col gap-2">
                  <h2 class="m-0">
                    <Trans
                      key="instance.create_new_instance_title"
                      options={{
                        defaultValue: "New instance",
                      }}
                    />
                  </h2>
                  <p class="m-0 text-darkSlate-50">
                    <Trans
                      key="instance.create_new_instance_text"
                      options={{
                        defaultValue: "Create your own empty instance",
                      }}
                    />
                  </p>
                </div>
              </div>
            </span>
            <div class="flex gap-3">
              <Dropdown
                options={[
                  { label: "1.16.5", key: "1.16.5" },
                  { label: "1.16.4", key: "1.16.4" },
                  { label: "1.16.3", key: "1.16.3" },
                  { label: "1.16.2", key: "1.16.2" },
                ]}
                icon={<div class="i-ri:price-tag-3-fill" />}
                rounded
                bgColorClass="bg-darkSlate-400"
                value="1.16.2"
              />
              <Button
                variant="glow"
                onClick={() =>
                  modalsContext?.openModal({ name: "instanceCreation" })
                }
              >
                <span class="uppercase">
                  <Trans
                    key="instance.create_instance_btn"
                    options={{
                      defaultValue: "Create",
                    }}
                  />
                </span>
              </Button>
            </div>
          </div>
        </div>
        <Show when={modpacks.length > 0} fallback={<NoModpacks />}>
          <For each={modpacks}>{(props) => <Modpack modpack={props} />}</For>
        </Show>
      </div>
    </div>
  );
}
