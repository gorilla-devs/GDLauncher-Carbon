import { initNAPI } from "./napi";
import config from "@gd/config";

let moduleNames = config.moduleNames;

export type ModuleStatus = Record<
  typeof moduleNames[number],
  {
    loaded: boolean;
    error?: Error;
  }
>;

const modulesStatus = moduleNames.reduce((acc, module) => {
  acc[module] = {
    loaded: false,
    error: undefined,
  };
  return acc;
}, {} as ModuleStatus);

export const initModules = async () => {
  let loadedModules = 0;

  try {
    console.time("Loading NAPI Module");
    await initNAPI();
    console.timeEnd("Loading NAPI Module");
  } catch (err) {
    console.error(err);
    window.fatalError(err as string, "NAPI");
    return;
  }

  let errored = false;

  try {
    await Promise.all(
      moduleNames.map(async (moduleName) => {
        try {
          console.time(`Loading Module - ${moduleName}`);
          const module = await import(`./components/${moduleName}.ts`);
          await module.init();
          if (errored) return;
          loadedModules++;
          window.updateLoading(loadedModules, moduleNames.length);
          modulesStatus[moduleName].loaded = true;
          console.timeEnd(`Loading Module - ${moduleName}`);
        } catch (err) {
          errored = true;
          modulesStatus[moduleName].error = err as Error;
          console.error(err);
          throw err;
        }
      })
    );

    await new Promise((resolve) => setTimeout(resolve, 200));
    window.clearLoading();
  } catch (e) {
    const erroredModule = moduleNames.find(
      (moduleName) => modulesStatus[moduleName].error
    );

    window.fatalError(e as string, erroredModule);
  }

  // Recover from errors?
};
