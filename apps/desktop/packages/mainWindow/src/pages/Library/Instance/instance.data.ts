import { port, rspc } from "@/utils/rspcClient";

//@ts-ignore
const fetchData = ({ params }) => {
  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10),
  ]);

  const image = `http://localhost:${port}/instance/instanceIcon?id=${params.id}`;

  return { image, instanceDetails };
};

export default fetchData;
