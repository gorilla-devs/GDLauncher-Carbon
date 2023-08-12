import { FEImportableInstance, FETask } from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";
import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";

export const [isLoading, setIsLoading] = createSignal(false);
export const [selectedInstancesIndexes, setSelectedInstancesIndexes] =
  createStore<{
    [id: number]: boolean;
  }>({});

export const [selectedInstancesNames, setSelectedInstancesNames] = createStore<{
  [id: number]: string;
}>({});
export const [loadingInstances, setLoadingInstances] = createStore<{
  [id: number]: FETask | null | undefined;
}>({});
export const [instances, setInstances] =
  createSignal<CreateQueryResult<FEImportableInstance[], RSPCError>>();
export const [taskId, setTaskId] = createSignal<undefined | number>(undefined);
export const [currentInstanceIndex, setCurrentInstanceIndex] = createSignal(0);
