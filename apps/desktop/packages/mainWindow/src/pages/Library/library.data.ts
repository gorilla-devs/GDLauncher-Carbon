import { rspc } from "@/utils/rspcClient";
import { FESettings, ListInstance } from "@gd/core_module/bindings";
import { createEffect } from "solid-js";
import { createStore } from "solid-js/store";

const fetchData = () => {
  const _instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }));
  const groups = rspc.createQuery(() => ({
    queryKey: ["instance.getGroups"]
  }));
  const _settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const [settings, _setSettings] = createStore<{
    data: FESettings | null;
    isLoading: boolean;
  }>({
    data: null,
    isLoading: _settings.isLoading || _settings.isInitialLoading
  });

  createEffect(() => {
    _setSettings({
      data: _settings.data,
      isLoading: _settings.isLoading || _settings.isInitialLoading
    });
  });

  const [instances, _setInstances] = createStore<{
    data: ListInstance[] | null;
    isLoading: boolean;
  }>({
    data: null,
    isLoading: _instances.isLoading || _instances.isInitialLoading
  });

  createEffect(() => {
    _setInstances({
      data: _instances.data,
      isLoading: _instances.isLoading || _instances.isInitialLoading
    });
  });

  return { settings, instances, groups };
};

export default fetchData;
