import os from "os";
import { spawnSync } from "child_process";
import targetMapping from "./targetMapping.mjs";

const argPlatform = (process.argv[2] || "").split("-")[0];
const argArch = (process.argv[2] || "").split("-")[1];

const target =
  targetMapping[`${argPlatform || os.platform()}-${argArch || os.arch()}`];

spawnSync(
  "npm",
  [
    "run",
    "cross-env-shell",
    `GOOS=${target.GOOS} GOARCH=${target.GOARCH}`,
    "go build",
  ],
  {
    stdio: "inherit",
    shell: true,
  }
);
