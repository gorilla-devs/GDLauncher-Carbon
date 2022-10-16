import fs from "fs";
import path from "path";
import packageJson from "../package.json" assert { type: "json" };

const files = fs.readdirSync(path.join("release", packageJson.version));

for (const file of files) {
  if (!file.endsWith(".zip") && !file.endsWith(".png")) {
    fs.rmSync(path.join("release", packageJson.version, file), {
      recursive: true,
      force: true,
    });
  }
}
