import core from "@gd/core";
import { createSignal } from "solid-js";

let [moduleLoaded, setModuleLoaded] = createSignal<boolean | Error>(false);

let resolvedAddon: typeof core | undefined;

let addon = (window as any)["__GDL__"] as Promise<
  () => typeof core | undefined
>;

// Potentially use a queue to hold reference to calls before the init
const napi: typeof core = new Proxy({} as any, {
  get(_, prop: keyof typeof resolvedAddon) {
    if (resolvedAddon) {
      return resolvedAddon?.[prop];
    } else {
      console.error(`NAPI not initialized. Called ${prop}`);
    }
  },
});

try {
  console.time("Loading NAPI Module");
  resolvedAddon = (await addon)();
  console.timeEnd("Loading NAPI Module");
  setModuleLoaded(true);
} catch {
  console.error("Failed to load NAPI module");
  setModuleLoaded(new Error("Failed to load NAPI module"));
}

export default napi;
export const isModuleLoaded = moduleLoaded;
