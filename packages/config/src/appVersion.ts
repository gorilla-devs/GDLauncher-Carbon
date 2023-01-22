const fs = require("fs");
const path = require("path");
const execSync = require("child_process").execSync;
const gitHash = execSync("git rev-parse --short HEAD").toString().trim();

const BASE_VERSION = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../", "version.json"))
).version;

export const COMPLETE_VERSION = `${BASE_VERSION}+${gitHash}`;
