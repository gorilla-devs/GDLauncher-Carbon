import { useRouteData } from "@solidjs/router";
import fetchData from "../modpack.data";
import { For } from "solid-js";
import { Tag } from "@gd/ui";
import { Trans } from "@gd/i18n";
import html from "solid-js/html";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div class="flex flex-col h-[800px]">
      <div class="flex flex-col gap-4">
        <h1>
          <Trans
            key="modpack.overview_categories"
            options={{
              defaultValue: "Categories",
            }}
          />
        </h1>
        <div class="flex gap-2 overflow-x-auto scrollbar-hide">
          <For each={routeData.modpackDetails.data?.data.categories}>
            {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
          </For>
        </div>
        {/* <div innerHTML={routeData.modpackDescription.data?.data} /> */}
      </div>
    </div>
  );
};

export default Overview;
