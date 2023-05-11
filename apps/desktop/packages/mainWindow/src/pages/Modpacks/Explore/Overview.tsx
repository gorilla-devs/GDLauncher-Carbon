import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.data";
import { For } from "solid-js";
import { Tag } from "@gd/ui";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div class="flex flex-col h-[800px]">
      <div class="flex gap-2 overflow-x-auto max-w-100 scrollbar-hide">
        <For each={routeData.modpackDetails.data?.data.categories}>
          {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
        </For>
      </div>
    </div>
  );
};

export default Overview;
