import core from "@gd/native_interface";

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
      console.error(`NAPI not initialized. Called ${prop}`);
    }
  },
});

export const initNAPI = async () => {
  resolvedAddon = (await addon)();
};

export default napi;
