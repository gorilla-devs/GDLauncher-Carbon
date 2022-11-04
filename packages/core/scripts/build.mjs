import os from "os";
import { spawnSync } from "child_process";
import targetMapping from "./targetMapping.mjs";

const argPlatform = (process.argv[2] || "").split("-")[0];
const argArch = (process.argv[2] || "").split("-")[1];

spawnSync(
  "npm",
  [
    "run",
    "_build_",
    "--",
    "--target",
    targetMapping[`${argPlatform || os.platform()}-${argArch || os.arch()}`],
  ],
  {
    stdio: "inherit",
    shell: true,
  }
);
