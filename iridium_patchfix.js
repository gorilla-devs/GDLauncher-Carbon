// spawn cargo new command and catch the output
const path = require("path");
const fs = require("fs");
const { spawn } = require("child_process");

const iridium_path = path.join(__dirname, "crates", "iridium");

if (!fs.existsSync(iridium_path)) {
  const cargo = spawn("cargo", ["new", "--lib", "iridium"], {
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
