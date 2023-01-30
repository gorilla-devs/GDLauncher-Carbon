import path from "path";

const isDev = import.meta.env.MODE === "development";
const nAPIPath = isDev ? "../../../../packages/native_interface" : "../../../";
import(path.resolve(__dirname, nAPIPath, "core.node")).catch((err) => {
  console.log(err);
});
