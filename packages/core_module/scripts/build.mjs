import { spawnSync } from "child_process";

const target = process.argv[2];

const args = ["build", "--release"];

if (target) {
  args.push("--target", target);
}

spawnSync("./node_modules/.bin/napi", args, {
  stdio: "inherit",
  shell: true,
});
