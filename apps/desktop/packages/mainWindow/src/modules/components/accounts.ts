import { Account, DeviceCodeObject } from "@gd/core";
import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import napi from "../napi";

const [accounts, setAccounts] = createStore<Account[]>([]);
export const [selectedAccount, setSelectedAccount] = createSignal<Account | undefined>(undefined);

export const login = async (callback: (res: DeviceCodeObject) => void) => {
  const account = await napi.auth(({ userCode, link, expiresAt }) =>
    callback({ userCode, link, expiresAt })
  );
  // TODO: check how to add the account
  setSelectedAccount(account);
};

export const init = async () => {
  const res = await napi.initAccounts();
  setAccounts(res.accounts);
  setSelectedAccount(res.selectedAccount);
};

export default accounts;
