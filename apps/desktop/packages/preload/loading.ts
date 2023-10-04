import { contextBridge } from "electron";
import { domReady } from "./utils";

let oDiv: HTMLDivElement;
function useLoading() {
  return {
    fatalError(error: string, moduleName?: string) {
      oDiv.classList.add("appFatalCrash");
      if (moduleName) {
        const errorText = `<div style="margin-top: 1.5rem; font-size: 1.3rem; font-weight: 400;">${error}</div>`;
        oDiv.innerHTML = `<div><div>Couldn't load module "${moduleName}"</div>${errorText}</div>`;
      } else {
        oDiv.innerHTML = `<div>${error}</div>`;
      }
    }
  };
}

const { fatalError } = useLoading();

(async () => {
  await domReady();
  oDiv = document.querySelector("#appFatalCrash")!;
})();

// --------- Expose some API to the Renderer process. ---------
contextBridge.exposeInMainWorld("fatalError", fatalError);
