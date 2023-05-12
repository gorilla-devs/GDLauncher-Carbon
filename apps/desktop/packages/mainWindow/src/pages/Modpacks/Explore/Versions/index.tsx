import { useRouteData } from "@solidjs/router";
import fetchData from "../../modpack.versions";
import { For } from "solid-js";
import VersionRow from "./VersionRow";

const Versions = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div class="flex flex-col">
      <For each={routeData.curseforgeGetModFiles.data?.data}>
        {(modFile) => <VersionRow modVersion={modFile} />}
      </For>
    </div>
  );
};

export default Versions;
