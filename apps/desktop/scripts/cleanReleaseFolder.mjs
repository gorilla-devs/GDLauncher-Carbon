import fs from "fs";
import path from "path";

const files = fs.readdirSync("release");

for (const file of files) {
  if (!(file.endsWith(".zip") || file.endsWith(".png"))) {
    fs.rmSync(path.join("release", file), {
      recursive: true,
      force: true
    });
  }
}
