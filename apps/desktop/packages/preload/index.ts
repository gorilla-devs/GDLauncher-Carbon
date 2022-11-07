import { contextBridge, ipcRenderer, screen } from "electron";
import { Titlebar, Color } from "custom-electron-titlebar";
import { domReady } from "./utils";
import { useLoading } from "./loading";
import napi from "./napi";

const { appendLoading, clearState, fatalError } = useLoading();

(async () => {
  await domReady();
  new Titlebar({
    containerOverflow: "visible",
    backgroundColor: Color.fromHex("#15181E"),
    icon: " ",
  });
  appendLoading();
})();

// --------- Expose some API to the Renderer process. ---------
contextBridge.exposeInMainWorld("clearState", clearState);
contextBridge.exposeInMainWorld("fatalError", fatalError);
contextBridge.exposeInMainWorld("ipcRenderer", withPrototype(ipcRenderer));
contextBridge.exposeInMainWorld("__GDL__", napi);
contextBridge.exposeInMainWorld(
  "getMinimumBounds",
  ipcRenderer.invoke("getMinimumBounds")
);

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
