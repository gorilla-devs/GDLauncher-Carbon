// spawn cargo new command and catch the output
const path = require("path");
const fs = require("fs");
const { spawn } = require("child_process");

const iridium_path = path.join(__dirname, "crates", "iridium");

let dir_ok = false;

try {
  const stat = fs.statSync(iridium_path);
  if (stat.isDirectory() && fs.readdirSync(iridium_path).length) {
    dir_ok = true;
  }
} catch {
  // ignore
}

if (!dir_ok) {
  const cargo = spawn("cargo", ["init", "--lib", "iridium"], {
    cwd: path.join(__dirname, "crates"),
  });

  cargo.on("close", (code) => {
    if (code === 0) {
      console.log("Iridium generated.");
    } else {
      console.log("Uh oh. Cargo new failed. Not good.");
    }
  });
}
