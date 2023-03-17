import { spawnSync } from "child_process";
import path from "path";

const target = process.argv[2];

const args = ["build", "--release"];

if (target) {
  args.push("--target", target);
}

spawnSync("cargo", args, {
  stdio: "inherit",
  shell: true,
});
