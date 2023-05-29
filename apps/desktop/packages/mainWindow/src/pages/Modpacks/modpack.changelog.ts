import { rspc } from "@/utils/rspcClient";

// @ts-ignore
const fetchData = ({ params, data }) => {
  const lastFile = data.modpackDetails?.data?.data.latestFiles.reverse()[0];

  if (lastFile) {
    const modpackChangelog = rspc.createQuery(() => [
      "modplatforms.curseforgeGetModFileChangelog",
      { modId: parseInt(params.id, 10), fileId: lastFile?.id || 0 },
    ]);

    return { modpackChangelog };
  } else return { modpackChangelog: null };
};

export default fetchData;
