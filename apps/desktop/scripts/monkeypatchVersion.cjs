const path = require("path");
const fs = require("fs");

const packageJsonPath = path.join(__dirname, "..", "package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

const actualVersion = require("@gd/config").appVersion;

packageJson.version = actualVersion;

fs.writeFileSync(
  packageJsonPath,
  `${JSON.stringify(packageJson, null, 2)}\n`,
  "utf8"
);
