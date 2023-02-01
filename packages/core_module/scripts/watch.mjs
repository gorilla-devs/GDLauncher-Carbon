import os from "os";
import { spawnSync } from "child_process";
import chokidar from "chokidar";
import targetMapping from "./targetMapping.mjs";

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
  spawnSync("./node_modules/.bin/napi", ["build"], {
    stdio: "inherit",
    shell: true,
  });
  console.log("Rebuilding native core done");
}, 1000);

//! IMPORTANT !//
// This only works on Unix systems, as in windows you cannot delete a file while it's being used (core.node).
// The workaround for windows would be to build the native core to a different file, and then reload the app
// targetting the new file.
chokidar
  .watch(["./src", "../../crates"], { ignoreInitial: false })
  .on("all", (event, path) => {
    debouncedFn();
  });
