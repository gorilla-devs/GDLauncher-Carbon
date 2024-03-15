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
    .find((arg) => arg.startsWith("--skip-intro-animation="))
    ?.split("=")[1] === "true";

const isDev = import.meta.env.DEV;

if (isDev || skipIntroAnimation) {
  contextBridge.exposeInMainWorld(
    "skipIntroAnimation",
    isDev || skipIntroAnimation
  );
}
