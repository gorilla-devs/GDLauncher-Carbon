import { app, ipcMain } from "electron";
import path from "path";
import fs from "fs/promises";
import fse from "fs-extra";
import os from "os";

export const RUNTIME_PATH_DIR_NAME = "gdlauncher_carbon";
export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override";

export function getInitialRTPath() {
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
    errorOnExist: false,
  });

  await fs.writeFile(runtimeOverridePath, newPath);

  const files = await fs.readdir(currentRTPath);
  for (const file of files) {
    if (file === RUNTIME_PATH_OVERRIDE_NAME) {
      continue;
    }

    try {
      await fs.unlink(path.join(currentRTPath, file));
    } catch (err) {
      console.error(`[RUNTIME] Error unlinking file: ${err}`);
    }
  }

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
