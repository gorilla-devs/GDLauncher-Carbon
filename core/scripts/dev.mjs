import os from "os";
import { spawnSync } from "child_process";

const targetMapping = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const argPlatform = (process.argv[2] || "").split("-")[0];
const argArch = (process.argv[2] || "").split("-")[1];

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
