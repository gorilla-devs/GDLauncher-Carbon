import { rspc } from "@/utils/rspcClient";
import { FESettings, ListGroup, ListInstance } from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";
import { JSX, createContext, useContext } from "solid-js";

type Context = {
  instances: CreateQueryResult<ListInstance[], RSPCError>;
  instanceGroups: CreateQueryResult<ListGroup[], RSPCError>;
  settings: CreateQueryResult<FESettings, RSPCError>;
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

  const store: Context = {
    instances: instances,
    instanceGroups: groups,
    settings: settings
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
