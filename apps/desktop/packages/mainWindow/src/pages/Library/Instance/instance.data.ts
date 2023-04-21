import { fetchImage } from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";
import { createResource } from "solid-js";

const fetchData = ({ params }) => {
  const [image] = createResource(() => params.id, fetchImage);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10),
  ]);

  return { image, instanceDetails };
};

export default fetchData;
