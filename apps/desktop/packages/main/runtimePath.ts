import { app, ipcMain } from "electron";
import fss from "fs";
import path from "path";
import fs from "fs/promises";
import fse from "fs-extra";
import os from "os";
import coreModule from "./coreModule";

export const RUNTIME_PATH_DIR_NAME = "gdlauncher_carbon";
export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override";

let INITIAL_RUNTIME_PATH_OVERRIDE: string | null = null;

export function initRTPath(override: string | null | undefined) {
  if (override) {
    INITIAL_RUNTIME_PATH_OVERRIDE = override;
    setCurrentRTPath(override);
    return;
  }

  const initialRTPath = getInitialRTPath();

  const runtimeOverridePath = path.join(
    initialRTPath,
    RUNTIME_PATH_OVERRIDE_NAME
  );

  let file_override: string | null = null;
  try {
    const tmp_path = fss.readFileSync(runtimeOverridePath).toString();
    fse.ensureDirSync(tmp_path);
    file_override = tmp_path;
  } catch {
    // ignore
  }

  setCurrentRTPath(file_override || initialRTPath);
}

export function getInitialRTPath() {
  if (import.meta.env.RUNTIME_PATH) {
    return import.meta.env.RUNTIME_PATH;
  } else if (INITIAL_RUNTIME_PATH_OVERRIDE) {
    return INITIAL_RUNTIME_PATH_OVERRIDE;
  }

  let runtimePath = null;

  if (os.platform() !== "linux") {
    runtimePath = app.getPath("appData");
  } else {
    // monkey patch linux since it defaults to .config instead of .local/share
    const xdgDataHome = process.env.XDG_DATA_HOME;
    if (xdgDataHome) {
      runtimePath = xdgDataHome;
    } else {
      const homeDir = os.homedir();
      runtimePath = path.join(homeDir, ".local/share");
    }
  }

  return path.join(runtimePath, RUNTIME_PATH_DIR_NAME);
}

export function getCurrentRTPath() {
  return app.getPath("userData");
}

export function setCurrentRTPath(newPath: string) {
  // userData is where electron stores all of its data
  app.setPath("userData", newPath);
}

ipcMain.handle("getInitialRuntimePath", async () => {
  return getInitialRTPath();
});

ipcMain.handle("getRuntimePath", async () => {
  return getCurrentRTPath();
});

ipcMain.handle("changeRuntimePath", async (_, _newPath: string | null) => {
  const initialRTPath = getInitialRTPath();
  const newPath = _newPath || initialRTPath;

  const currentRTPath = getCurrentRTPath();

  if (newPath === currentRTPath) {
    return;
  }

  const runtimeOverridePath = path.join(
    initialRTPath,
    RUNTIME_PATH_OVERRIDE_NAME
  );

  await fs.mkdir(newPath, { recursive: true });

  await fse.copy(currentRTPath, newPath, {
    overwrite: true,
    filter: (src) => {
      return !src.includes(RUNTIME_PATH_OVERRIDE_NAME);
    },
    errorOnExist: false
  });

  await fs.writeFile(runtimeOverridePath, newPath);

  try {
    let _coreModule = await coreModule;
    _coreModule.kill();
  } catch {
    // No op
  }

  await fse.remove(path.join(currentRTPath, "data"));

  app.relaunch();
});

ipcMain.handle("validateRuntimePath", async (_, newPath: string | null) => {
  if (!newPath || newPath === getCurrentRTPath()) {
    return false;
  }

  const pathExists = await fse.pathExists(newPath);
  if (!pathExists) {
    return true;
  }

  const newPathStat = await fs.stat(newPath);
  if (!newPathStat.isDirectory()) {
    return false;
  }

  const files = await fs.readdir(newPath);
  if (files.length > 0) {
    return false;
  }

  return true;
});
