import fs from "fs";
import path from "path";
import { contextBridge, ipcRenderer } from "electron";
import { domReady } from "./utils";
import { useLoading } from "./loading";

const nAPIPath =
  import.meta.env.MODE === "development" ? "./packages/napi" : __dirname;

const addon = require(path.resolve(nAPIPath, "napi.node"));

const { appendLoading, removeLoading } = useLoading();

(async () => {
  await domReady();

  appendLoading();
})();

// --------- Expose some API to the Renderer process. ---------
contextBridge.exposeInMainWorld("fs", fs);
contextBridge.exposeInMainWorld("removeLoading", removeLoading);
contextBridge.exposeInMainWorld("ipcRenderer", withPrototype(ipcRenderer));
contextBridge.exposeInMainWorld("computeFibonacci", computeFibonacci);

console.log("INITTAL", addon);

function computeFibonacci() {
  const res = addon.fibonacci(45);
  console.log(res);
  return res;
}

// `exposeInMainWorld` can't detect attributes and methods of `prototype`, manually patching it.
function withPrototype(obj: Record<string, any>) {
  const protos = Object.getPrototypeOf(obj);

  for (const [key, value] of Object.entries(protos)) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) continue;

    if (typeof value === "function") {
      // Some native APIs, like `NodeJS.EventEmitter['on']`, don't work in the Renderer process. Wrapping them into a function.
      obj[key] = function (...args: any) {
        return value.call(obj, ...args);
      };
    } else {
      obj[key] = value;
    }
  }
  return obj;
}
