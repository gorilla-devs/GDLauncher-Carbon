import { rspc } from "@/utils/rspcClient";

const fetchData = ({ params }: { params: any }) => {
  const isCurseforge = params.platform === "curseforge";
  const isModrinth = params.platform === "Modrinth";
  if (isCurseforge) {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.curseforge.getMod",
      { modId: parseInt(params.id, 10) },
    ]);

    const modpackDescription = rspc.createQuery(() => [
      "modplatforms.curseforge.getModDescription",
      { modId: parseInt(params.id, 10) },
    ]);

    return { modpackDetails, modpackDescription, isCurseforge, isModrinth };
  } else {
    const modpackDetails = rspc.createQuery(() => [
      "modplatforms.modrinth.getProject",
      params.id,
    ]);

    return { modpackDetails, isCurseforge, isModrinth };
  }
};

export default fetchData;
