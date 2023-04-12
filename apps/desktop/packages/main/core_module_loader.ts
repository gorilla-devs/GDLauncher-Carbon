import path from "path";
import os from "os";
import { spawn } from "child_process";

const isDev = import.meta.env.MODE === "development";

const binaryName =
  os.platform() === "win32" ? "core_module.exe" : "core_module";

const loadCoreModule = () =>
  new Promise((resolve, reject) => {
    if (isDev) {
      resolve(0);
      return;
    }
    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources",
      binaryName
    );

    console.log(`[CORE] Spawning core module: ${coreModulePath}`);

    const coreModule = spawn(coreModulePath, [], {
      shell: false,
      detached: false,
      env: {
        RUST_BACKTRACE: "full",
      },
    });

    coreModule.stdout.on("data", (data) => {
      let dataString = data.toString();
      if (dataString.startsWith("_STATUS_: ")) {
        let port = dataString.split("|")[1];
        resolve(port);
      }
      console.log(`[CORE] Message: ${dataString}`);
    });

    coreModule.stderr.on("data", (data) => {
      console.error(`[CORE] Error: ${data.toString()}`);
    });

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`);
      resolve(0);

      if (code !== 0) {
        reject(new Error(`Core module exited with code ${code}`));
      }

      resolve(0);
    });
  });

export default loadCoreModule;
