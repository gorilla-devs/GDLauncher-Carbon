import path from "path";
import core from "@gd/core";

const isDev = import.meta.env.MODE === "development";
const nAPIPath = isDev ? "../../core" : __dirname;

let calledOnce = false;
let addon = new Promise<() => typeof core | undefined>((resolve, reject) => {
  import(path.resolve(nAPIPath, "core.node"))
    .then((value: typeof core) => {
      resolve(() => {
        if (calledOnce) {
          return;
        }
        calledOnce = true;
        return value;
      });
    })
    .catch(reject);
});

export default addon;
