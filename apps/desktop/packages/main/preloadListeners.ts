import { ipcMain, shell, clipboard } from "electron";

ipcMain.handle("openExternalLink", async (_, link) => {
  shell.openExternal(link);
});

ipcMain.handle("copyToClipboard", async (_, text) => {
  clipboard.writeText(text, "clipboard");
});
