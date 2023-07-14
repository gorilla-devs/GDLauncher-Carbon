import { rspc } from "@/utils/rspcClient";
import { FESearchAPI } from "@gd/core_module/bindings";
import { createEffect } from "solid-js";

type Params = {
  id: string;
  platform: FESearchAPI;
};

const fetchData = ({ params }: { params: Params }) => {
  const isCurseforge = params.platform === "curseforge";
  if (isCurseforge) {
    const numericId = parseInt(params.id, 10);
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.curseforgeGetMod",
      { modId: numericId },
    ]);

    const modpackDescription = rspc.createQuery(() => [
      "modplatforms.curseforgeGetModDescription",
      { modId: numericId },
    ]);

    return { modpackDetails, modpackDescription, isCurseforge };
  } else {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.modrinthGetProject",
      params.id,
    ]);

    return { modpackDetails, isCurseforge };
  }
};

export default fetchData;
