import { Accounts } from "@gd/core";
import { createStore } from "solid-js/store";
import napi from "../napi";

const [accounts, setAccounts] = createStore<Accounts>({
  accounts: [],
  selectedAccount: undefined,
});

export const init = async () => {
  const res = await napi.initAccounts();
  setAccounts(res);
};

export default accounts;