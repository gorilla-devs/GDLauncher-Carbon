import { app, ipcMain } from "electron";
import fss from "fs";
import path from "path";
import fs from "fs/promises";
import fse from "fs-extra";
import coreModule from "./coreModule";

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override";
const RUNTIME_PATH_DEFAULT_NAME = "data";

export let CURRENT_RUNTIME_PATH: string | null = null;

export function initRTPath(override: string | null | undefined) {
  if (override) {
    CURRENT_RUNTIME_PATH = override;
    return;
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
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

  CURRENT_RUNTIME_PATH =
    file_override ||
    path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME);
}

ipcMain.handle("getUserData", async () => {
  return app.getPath("userData");
});

ipcMain.handle("getInitialRuntimePath", async () => {
  return path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME);
});

ipcMain.handle("getRuntimePath", async () => {
  return CURRENT_RUNTIME_PATH;
});

ipcMain.handle("changeRuntimePath", async (_, newPath: string) => {
  if (newPath === CURRENT_RUNTIME_PATH) {
    return;
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
    RUNTIME_PATH_OVERRIDE_NAME
  );

  await fs.mkdir(newPath, { recursive: true });

  (await coreModule).kill();

  // TODO: Copy with progress
  await fse.copy(CURRENT_RUNTIME_PATH!, newPath, {
    overwrite: true,
    errorOnExist: false
  });

  await fs.writeFile(runtimeOverridePath, newPath);

  await fse.remove(CURRENT_RUNTIME_PATH!);

  // TODO: with a bit of work we can change the RTPath without actually restarting the app
  app.relaunch();
  app.exit();
});

ipcMain.handle("validateRuntimePath", async (_, newPath: string | null) => {
  if (!newPath || newPath === CURRENT_RUNTIME_PATH) {
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
