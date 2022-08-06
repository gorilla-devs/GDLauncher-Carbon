import core from "@gd/core";

let addon = (window as any)[import.meta.env.VITE_NAPI_ID] as Promise<
  typeof core
>;

// TODO: HANDLE POTENTIAL LOAD ERROR
let resolvedAddon = await addon;

const napi: typeof core = new Proxy({} as any, {
  get(_, prop: keyof typeof resolvedAddon) {
    if (resolvedAddon[prop]) {
      return resolvedAddon[prop];
    }
    throw new Error(`Call to ${prop} failed.`);
  },
});

export default napi;
