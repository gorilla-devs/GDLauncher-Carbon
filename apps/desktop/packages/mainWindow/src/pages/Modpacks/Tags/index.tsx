import CurseforgeIcon from "/assets/images/icons/curseforge.png";
import { For } from "solid-js";
import Tag from "./Tag";
const tags = [
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
  { name: "curseforge", img: CurseforgeIcon },
];

const Tags = () => {
  return (
    <div class="flex gap-2">
      <For each={tags}>{(tag) => <Tag name={tag.name} img={tag.img} />}</For>
    </div>
  );
};

export default Tags;
