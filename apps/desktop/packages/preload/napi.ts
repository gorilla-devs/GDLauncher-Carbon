import path from "path";
import core from "@gd/core";

const nAPIPath =
  import.meta.env.MODE === "development" ? "../../core" : __dirname;

let addon = new Promise<typeof core>((resolve, reject) => {
  import(path.resolve(nAPIPath, "core.node")).then(resolve).catch(reject);
});

export default addon;
