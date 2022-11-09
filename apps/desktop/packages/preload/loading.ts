import { contextBridge } from "electron";
import { Titlebar, Color } from "custom-electron-titlebar";
import { domReady } from "./utils";

function useLoading() {
  const oDiv = document.createElement("div");
  oDiv.style.position = "fixed";
  oDiv.style.top = "0";
  oDiv.style.left = "0";
  oDiv.style.width = "100%";
  oDiv.style.height = "100%";
  oDiv.style.zIndex = "999999";
  oDiv.style.backgroundColor = "#1D2028";
  oDiv.style.color = "#fff";
  oDiv.style.display = "flex";
  oDiv.style.justifyContent = "center";
  oDiv.style.alignItems = "center";
  oDiv.style.fontSize = "1.7rem";
  oDiv.style.textAlign = "center";

  return {
    appendLoading() {
      oDiv.innerHTML = `<div></div>`;
      oDiv.className = "appLoadingState";
      oDiv.style.fontWeight = "600";
      document.body.appendChild(oDiv);
    },
    updateLoading(loaded: number, total: number) {
      oDiv.innerHTML = `<div>Loaded ${loaded}/${total} modules</div>`;
    },
    async clearLoading() {
      document.body.removeChild(oDiv);
    },
    fatalError(error: string, moduleName?: string) {
      oDiv.style.fontWeight = "600";
      oDiv.className = "appFatalCrashState";
      if (moduleName) {
        const errorText = `<div style="margin-top: 1.5rem; font-size: 1.3rem; font-weight: 400;">${error}</div>`;
        oDiv.innerHTML = `<div><div>Couldn't load module "${moduleName}"</div>${errorText}</div>`;
      } else {
        oDiv.innerHTML = `<div>${error}</div>`;
      }
    },
  };
}

const { appendLoading, clearLoading, fatalError, updateLoading } = useLoading();
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
contextBridge.exposeInMainWorld("updateLoading", updateLoading);
contextBridge.exposeInMainWorld("clearLoading", clearLoading);
contextBridge.exposeInMainWorld("fatalError", fatalError);
