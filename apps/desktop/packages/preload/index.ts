import { contextBridge } from "electron";

import "./loading";
import "./core_module_loader";
import "./minimumBounds";
import "./autoupdate";
import "./openExternal";
import "./os";
import "./overwolf";
import "./runtimePath";
import "./relaunch";

const skipIntroAnimation =
  process.argv
    .find((arg) => arg.startsWith("--skipIntroAnimation="))
    ?.split("=")[1] === "true";

if (skipIntroAnimation) {
  contextBridge.exposeInMainWorld("skipIntroAnimation", skipIntroAnimation);
}
