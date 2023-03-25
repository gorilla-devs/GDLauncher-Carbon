const fs = require("fs");
const path = require("path");
const execSync = require("child_process").execSync;
const gitHash = execSync("git rev-parse --short HEAD").toString().trim();

const VERSION_JSON = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../", "version.json"))
);

export const COMPLETE_VERSION = `${VERSION_JSON.version}${VERSION_JSON.channel}+${gitHash}`;
