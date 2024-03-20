import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));
  const status = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"]
  }));
  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));
  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));

  return { status, activeUuid, accounts, settings };
};

export default fetchData;
