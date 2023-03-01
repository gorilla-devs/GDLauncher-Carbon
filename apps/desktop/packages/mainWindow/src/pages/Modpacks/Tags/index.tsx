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
      <div class="flex flex-1 gap-2 max-w-full overflow-x-auto">
        <For each={tags}>{(tag) => <Tag name={tag.name} img={tag.img} />}</For>
      </div>
      <Button variant="secondary" textColor="text-red" rounded={false}>
        <Trans
          key="clear_filters"
          options={{
            defaultValue: "Clear filters",
          }}
        />
      </Button>
    </div>
  );
};

export default Tags;
