import path from "path";
import fs from "fs";
import { appVersion } from "@gd/config";

const packageJsonPath = path.join("package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

const actualVersion = appVersion;

packageJson.version = actualVersion;

fs.writeFileSync(
  packageJsonPath,
  `${JSON.stringify(packageJson, null, 2)}\n`,
  "utf8"
);
