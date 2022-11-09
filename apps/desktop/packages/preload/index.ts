import { contextBridge } from "electron";
import { Titlebar, Color } from "custom-electron-titlebar";
import { domReady } from "./utils";
import "./loading";
import "./napi";
import "./minimumBounds";

(async () => {
  await domReady();
  new Titlebar({
    containerOverflow: "visible",
    backgroundColor: Color.fromHex("#15181E"),
    icon: " ",
  });
})();

