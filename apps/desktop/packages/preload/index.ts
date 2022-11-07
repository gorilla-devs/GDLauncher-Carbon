import { contextBridge } from "electron";
import { Titlebar, Color } from "custom-electron-titlebar";
import { domReady } from "./utils";
import { useLoading } from "./loading";
import "./napi";
import "./minimumBounds";

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
