import { rspc } from "@/utils/rspcClient";
import { createEffect } from "solid-js";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const curseforgeGetModFiles = rspc.createQuery(() => [
      "modplatforms.curseforgeGetModFiles",
      { modId: parseInt(params.id, 10), query: {} },
    ]);

    const curseforgeGetMod = rspc.createQuery(() => [
      "modplatforms.curseforgeGetMod",
      { modId: parseInt(params.id, 10) },
    ]);
    return { curseforgeGetModFiles, curseforgeGetMod, isCurseforge };
  } else {
    const modrinthGetProject = rspc.createQuery(() => [
      "modplatforms.modrinthGetProject",
      params.id,
    ]);

    return { modrinthGetProject, isCurseforge };
  }
};

export default fetchData;
