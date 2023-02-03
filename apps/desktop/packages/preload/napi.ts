import { contextBridge } from "electron";
import path from "path";

const isDev = import.meta.env.MODE === "development";

// const prodPath = process.platform === "darwin" ? "../../../" : "../../";

const nAPIPath = isDev ? "../../../../packages/core_module" : "../../../";

let napiLoaded = new Promise((resolve, reject) => {
  import(path.resolve(__dirname, nAPIPath, "core.node"))
    .then(() => {
      resolve(null);
    })
    .catch((err) => {
      console.log(err);
      reject(err);
    });
});

contextBridge.exposeInMainWorld("napiLoaded", napiLoaded);
