import path from "path";
import core from "@gd/core";
import { contextBridge } from "electron";

const isDev = import.meta.env.MODE === "development";
const nAPIPath = isDev ? "../../packages/core" : `${__dirname}/../../../`;

let calledOnce = false;
let addon = new Promise<() => typeof core | undefined>((resolve, reject) => {
  import(path.resolve(nAPIPath, "core.node"))
    .then((value: typeof core) => {
      resolve(() => {
        if (calledOnce) {
          return;
        }
        calledOnce = true;
        return value;
      });
    })
    .catch(reject);
});

contextBridge.exposeInMainWorld(import.meta.env.VITE_NAPI_ID, addon);