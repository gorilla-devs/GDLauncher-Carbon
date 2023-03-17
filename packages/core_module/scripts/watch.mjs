import { spawnSync } from "child_process";
import chokidar from "chokidar";
import path from "path";

// function debounce(func, timeout) {
//   let timer;
//   return (...args) => {
//     if (!timer) {
//       func.apply(this, args);
//     }
//     clearTimeout(timer);
//     timer = setTimeout(() => {
//       timer = undefined;
//     }, timeout);
//   };
// }

// let debouncedFn = debounce(() => {
//   console.log("Rebuilding native core");
//   const spawnHandler = spawnSync("cargo", ["build"], {
//     stdio: "inherit",
//     shell: true,
//   });
//   if (spawnHandler && spawnHandler.status !== 0) {
//     console.log("Native core build failed!");
//   } else {
//     console.log("Rebuilding native core done");
//   }
// }, 1000);

//! IMPORTANT !//
// This only works on Unix systems, as in windows you cannot delete a file while it's being used (core.node).
// The workaround for windows would be to build the native core to a different file, and then reload the app
// targetting the new file.
// chokidar
//   .watch(["./src", "../../crates"], { ignoreInitial: false })
//   .on("all", (event, path) => {
//     debouncedFn();
//   });

spawnSync("cargo", ["run"], {
  stdio: "inherit",
  shell: true,
});
