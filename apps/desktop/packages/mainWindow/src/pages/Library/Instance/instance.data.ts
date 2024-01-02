import { rspc } from "@/utils/rspcClient";
import { Mod } from "@gd/core_module/bindings";
import { createMemo } from "solid-js";
import { createStore, reconcile } from "solid-js/store";

//@ts-ignore
const fetchData = ({ params }) => {
  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(params.id, 10)
  ]);

  const modpackInfo = rspc.createQuery(() => [
    "instance.getModpackInfo",
    parseInt(params.id, 10)
  ]);

  const instancesUngrouped = rspc.createQuery(() => [
    "instance.getInstancesUngrouped"
  ]);

  const _instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    parseInt(params.id, 10)
  ]);

  const [instanceMods, setInstanceMods] = createStore({
    mods: [] as Mod[]
  });

  createMemo(() => {
    const mods = _instanceMods.data;
    setInstanceMods("mods", reconcile(mods || []));
  });

  const totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);

  return {
    instanceDetails,
    modpackInfo,
    instanceMods: instanceMods.mods,
    instancesUngrouped,
    totalRam
  };
};

export default fetchData;
