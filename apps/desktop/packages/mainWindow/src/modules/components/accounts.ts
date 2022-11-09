import { Accounts, DeviceCodeObject } from "@gd/core";
import { createStore } from "solid-js/store";
import napi from "../napi";

const [accounts, setAccounts] = createStore<Accounts>({
  accounts: [],
  selectedAccount: undefined,
});

export const login = async (callback: (res: DeviceCodeObject) => void) => {
  const authToken = await napi.auth(({ userCode, link, expiresAt }) =>
    callback({ userCode, link, expiresAt })
  );
  // TODO: check how to add the account
  // setAccounts()
};

export const init = async () => {
  const res = await napi.initAccounts();
  setAccounts(res);
};

export default accounts;
