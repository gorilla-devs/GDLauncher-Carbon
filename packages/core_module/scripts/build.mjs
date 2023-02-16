import { spawnSync } from "child_process";
import path from "path";

const target = process.argv[2];

const args = ["build", "--release"];

if (target) {
  args.push("--target", target);
}

spawnSync(path.join(process.cwd(), "node_modules", ".bin", "napi"), args, {
  stdio: "inherit",
  shell: true,
});
