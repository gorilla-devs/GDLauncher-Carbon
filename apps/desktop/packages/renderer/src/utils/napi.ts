import core from "@gd/core";

let resolvedAddon: typeof core | undefined;

let addon = (window as any)[import.meta.env.VITE_NAPI_ID] as Promise<
  () => typeof core | undefined
>;

// Potentially use a queue to hold reference to calls before the init
const napi: typeof core = new Proxy({} as any, {
  get(_, prop: keyof typeof resolvedAddon) {
    if (resolvedAddon) {
      return resolvedAddon?.[prop];
    } else {
      console.error(`NAPI not initialized yet. Calling ${prop} too early.`);
    }
  },
});

console.time("Loading NAPI Module");
// TODO: Handle failure
resolvedAddon = (await addon)();
console.timeEnd("Loading NAPI Module");

export default napi;
