import { contextBridge } from "electron";
import { Titlebar, Color } from "custom-electron-titlebar";
import { domReady } from "./utils";

let oDiv: HTMLDivElement;
function useLoading() {
  return {
    updateLoading(loaded: number, total: number) {
      oDiv!.innerHTML = `<div>Loaded ${loaded} / ${total} modules</div>`;
    },
    async clearLoading() {
      const root = document.querySelector("#root") as HTMLDivElement;
      oDiv!.style.transform = "translateY(-100%)";
      await new Promise((resolve) => setTimeout(resolve, 400));
      oDiv.remove();
    },
    fatalError(error: string, moduleName?: string) {
      oDiv.id = "appFatalCrashState";
      if (moduleName) {
        const errorText = `<div style="margin-top: 1.5rem; font-size: 1.3rem; font-weight: 400;">${error}</div>`;
        oDiv.innerHTML = `<div><div>Couldn't load module "${moduleName}"</div>${errorText}</div>`;
      } else {
        oDiv.innerHTML = `<div>${error}</div>`;
      }
    },
  };
}

const { clearLoading, fatalError, updateLoading } = useLoading();

(async () => {
  await domReady();
  oDiv = document.querySelector("#appLoadingState")!;
  new Titlebar({
    containerOverflow: "visible",
    backgroundColor: Color.fromHex("#15181E"),
    icon: " ",
  });
})();

// --------- Expose some API to the Renderer process. ---------
contextBridge.exposeInMainWorld("updateLoading", updateLoading);
contextBridge.exposeInMainWorld("clearLoading", clearLoading);
contextBridge.exposeInMainWorld("fatalError", fatalError);
