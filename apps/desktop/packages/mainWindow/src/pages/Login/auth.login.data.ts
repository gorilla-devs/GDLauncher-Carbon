import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const status = rspc.createQuery(() => ["account.enroll.getStatus", null]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid", null]);

  return { status, activeUuid };
};

export default fetchData;
