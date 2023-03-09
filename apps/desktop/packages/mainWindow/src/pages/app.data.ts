import { rspc } from "@/utils/rspcClient";
import { AccountType } from "@gd/core_module/bindings";

export type Accounts = {
  username: string;
  uuid: string;
  type_: AccountType;
};

const fetchData = () => {
  const accounts = rspc.createQuery(() => ["account.getAccounts", null]);
  const activeUuid = rspc.createQuery(() => ["account.getActiveUuid", null]);
  const status = rspc.createQuery(() => ["account.enroll.getStatus", null]);

  return { accounts, activeUuid, status };
};

export default fetchData;
