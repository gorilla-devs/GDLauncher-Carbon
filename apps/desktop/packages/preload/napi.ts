import path from "path";

const isDev = import.meta.env.MODE === "development";
const nAPIPath = isDev
  ? "../../packages/native_interface"
  : `${__dirname}/../../../`;

import(path.resolve(nAPIPath, "core.node")).catch((err) => {
  console.log(err);
});
