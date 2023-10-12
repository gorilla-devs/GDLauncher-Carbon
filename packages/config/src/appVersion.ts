import fs from "fs";
import path from "path";
import { execSync } from "child_process";
const gitCommitDate = execSync("git log -1 --format=%ct").toString().trim();

const VERSION_JSON = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../", "version.json")).toString()
);
const channel = VERSION_JSON.channel
  ? "-" + VERSION_JSON.channel + "." + gitCommitDate
  : "";

const version = VERSION_JSON.version;

export const COMPLETE_VERSION = `${version}${channel}`;
