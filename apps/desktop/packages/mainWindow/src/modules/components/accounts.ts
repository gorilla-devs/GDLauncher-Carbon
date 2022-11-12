import { Account, Accounts, DeviceCodeObject } from "@gd/core";
import { createStore } from "solid-js/store";
import napi from "../napi";

interface IAccounts extends Accounts {
  currentAccount: Account | undefined;
}

export const [accounts, setAccounts] = createStore<IAccounts>({
  accounts: [],
  selectedAccountId: undefined,
  get currentAccount() {
    return this.accounts.find(
      (account: Account) => account.id === this.selectedAccountId
    );
  },
});

export const login = async (callback: (res: DeviceCodeObject) => void) => {
  const accounts = await napi.auth(({ userCode, link, expiresAt }) =>
    callback({ userCode, link, expiresAt })
  );
  // TODO: check how to add the account
  setAccounts("selectedAccountId", accounts.id);
};

export const init = async () => {
  const res = await napi.initAccounts();
  setAccounts(res);
};

export default accounts;
