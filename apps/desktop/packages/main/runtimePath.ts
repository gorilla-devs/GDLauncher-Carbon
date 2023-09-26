import { app, ipcMain } from "electron";
import path from "path";
import fs from "fs/promises";
import fse from "fs-extra";

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override";

export let INITIAL_RUNTIME_PATH = { value: "" };

ipcMain.handle("changeRuntimePath", async (_, newPath: string) => {
  const currentPath = app.getPath("userData");
  const runtimeOverridePath = path.join(
    currentPath,
    RUNTIME_PATH_OVERRIDE_NAME
  );

  await fs.mkdir(newPath, { recursive: true });

  await fse.copy(currentPath, newPath, {
    overwrite: true,
    filter: (src) => {
      return !src.includes(RUNTIME_PATH_OVERRIDE_NAME);
    },
    errorOnExist: false,
  });

  await fs.writeFile(runtimeOverridePath, newPath);

  const files = await fs.readdir(currentPath);
  for (const file of files) {
    if (file === RUNTIME_PATH_OVERRIDE_NAME) {
      continue;
    }

    try {
      await fs.unlink(path.join(currentPath, file));
    } catch (err) {
      console.error(`[RUNTIME] Error unlinking file: ${err}`);
    }
  }

  app.relaunch();
});
