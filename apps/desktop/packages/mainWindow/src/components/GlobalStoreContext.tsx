import { rspc } from "@/utils/rspcClient";
import {
  AccountEntry,
  FEGDLAccount,
  FESettings,
  ListGroup,
  ListInstance
} from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";
import { JSX, createContext, useContext } from "solid-js";

type Context = {
  instances: CreateQueryResult<ListInstance[], RSPCError>;
  instanceGroups: CreateQueryResult<ListGroup[], RSPCError>;
  settings: CreateQueryResult<FESettings, RSPCError>;
  accounts: CreateQueryResult<AccountEntry[], RSPCError>;
  currentlySelectedAccount: () => AccountEntry | null;
  currentlySelectedAccountUuid: CreateQueryResult<string | null, RSPCError>;
  gdlAccount: CreateQueryResult<FEGDLAccount | null, RSPCError>;
};

const GlobalStoreContext = createContext();

export const GlobalStoreProvider = (props: { children: JSX.Element }) => {
  const instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }));

  const groups = rspc.createQuery(() => ({
    queryKey: ["instance.getGroups"]
  }));

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const accounts = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }));

  const currentlySelectedAccountUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));

  const gdlAccount = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount"]
  }));

  const currentlySelectedAccount = () => {
    const uuid = currentlySelectedAccountUuid.data;
    if (!uuid) return null;

    return accounts.data?.find((account) => account.uuid === uuid) || null;
  };

  const store: Context = {
    instances,
    instanceGroups: groups,
    settings,
    accounts,
    currentlySelectedAccountUuid,
    currentlySelectedAccount,
    gdlAccount
  };

  return (
    <GlobalStoreContext.Provider value={store}>
      {props.children}
    </GlobalStoreContext.Provider>
  );
};

export const useGlobalStore = (): Context => {
  return useContext(GlobalStoreContext) as Context;
};
