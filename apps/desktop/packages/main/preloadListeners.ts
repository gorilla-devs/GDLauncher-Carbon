import { ipcMain, shell } from "electron";

ipcMain.handle("openExternalLink", async (_, link) => {
  shell.openExternal(link);
});
