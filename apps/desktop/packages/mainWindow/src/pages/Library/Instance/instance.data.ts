import { fetchImage } from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";
import { createEffect, createResource } from "solid-js";

//@ts-ignore
const fetchData = ({ params }) => {
  const [image] = createResource(() => params.id, fetchImage);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10),
  ]);
  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    parseInt(params.id, 10),
  ]);

  return { image, instanceDetails, instanceMods };
};

export default fetchData;
