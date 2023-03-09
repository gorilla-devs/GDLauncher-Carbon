import { rspc } from "@/utils/rspcClient";
import { AccountType } from "@gd/core_module/bindings";

export type Accounts = {
  username: string;
  uuid: string;
  type_: AccountType;
};

const fetchData = () => {
  let data = rspc.createQuery(() => ["account.getAccounts", null], {
    onError(err) {
      console.log("DDDDD", err);
    },
  });
  return data;
};

export default fetchData;
