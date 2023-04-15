import CurseforgeIcon from "/assets/images/icons/curseforge.png";
import { For } from "solid-js";
import { Button, Tag } from "@gd/ui";
import { Trans } from "@gd/i18n";
const tags = [
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
];

const Tags = () => {
  return (
    <div class="flex gap-2 max-w-full items-center">
      <div class="scrollbar-hide flex flex-1 gap-2 max-w-full overflow-x-auto">
        <For each={tags}>{(tag) => <Tag name={tag.name} img={tag.img} />}</For>
      </div>
      <Button
        class="h-8"
        variant="secondary"
        textColor="text-red-500"
        rounded={false}
      >
        <Trans
          key="instance.clear_filters_modpacks"
          options={{
            defaultValue: "Clear filters",
          }}
        />
      </Button>
    </div>
  );
};

export default Tags;
