import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  const isModrinth = params.platform === "Modrinth";

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

    return { modpackDetails, modpackDescription, isCurseforge, isModrinth };
  } else {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.modrinthGetProject",
      params.id,
    ]);

    return { modpackDetails, isCurseforge, isModrinth };
  }
};

export default fetchData;
