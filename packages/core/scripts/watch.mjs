import os from "os";
import { spawnSync } from "child_process";
import chokidar from "chokidar";
import targetMapping from "./targetMapping.mjs";

await new Promise((r) => setTimeout(r, 350));

const argPlatform = (process.argv[2] || "").split("-")[0];
const argArch = (process.argv[2] || "").split("-")[1];

function debounce(func, timeout) {
  let timer;
  return (...args) => {
    if (!timer) {
      func.apply(this, args);
    }
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
    }, timeout);
  };
}

let debouncedFn = debounce(() => {
  console.log("Rebuilding native core");
  spawnSync(
    "npm",
    [
      "run",
      "_dev_",
      "--",
      "--target",
      targetMapping[`${argPlatform || os.platform()}-${argArch || os.arch()}`],
    ],
    {
      stdio: "inherit",
      shell: true,
    }
  );
  console.log("Rebuilding native core done");
}, 1000);

chokidar.watch("./src", { ignoreInitial: true }).on("all", (event, path) => {
  debouncedFn();
});
