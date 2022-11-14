import { contextBridge } from "electron";
import path from "path";

contextBridge.exposeInMainWorld("nodePath", {
  join: path.join,
  resolve: path.resolve,
});
