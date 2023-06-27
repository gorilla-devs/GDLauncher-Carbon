import path from "path";
import os from "os";
import { spawn } from "child_process";
import { ipcMain } from "electron";
import {
  initDRPC,
  shutdownDRPC,
  stopActivityDRPC,
  updateActivityDRPC,
} from "./discordRPC";

const isDev = import.meta.env.MODE === "development";

const binaryName =
  os.platform() === "win32" ? "core_module.exe" : "core_module";

type CoreModule = () => Promise<number>;

const loadCoreModule: CoreModule = () =>
  new Promise((resolve, reject) => {
    if (isDev) {
      resolve(4650);
      return;
    }

    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources",
      binaryName
    );

    console.log(`[CORE] Spawning core module: ${coreModulePath}`);
    let coreModule;

    try {
      coreModule = spawn(coreModulePath, [], {
        shell: false,
        detached: false,
        env: {
          RUST_BACKTRACE: "full",
        },
      });
    } catch (err) {
      console.error(`[CORE] Spawn error: ${err}`);
      return reject(err);
    }

    coreModule.on("error", function (err) {
      console.error(`[CORE] Spawn error: ${err}`);
      reject(err);
    });

    coreModule.stdout.on("data", (data) => {
      let dataString = data.toString();
      let rows = dataString.split(/\r?\n|\r|\n/g);

      for (let row of rows) {
        if (row.startsWith("_STATUS_:")) {
          const port = row.split("|")[1];
          console.log(`[CORE] Port: ${port}`);
          resolve(port);
        } else if (row.startsWith("_DRPC_:")) {
          const [eventName, value] = row.split(":")[1].split("|");
          switch (eventName) {
            case "INIT":
              initDRPC();
              break;
            case "SHUTDOWN":
              shutdownDRPC();
              break;
            case "STOP_ACTIVITY":
              stopActivityDRPC();
              break;
            case "UPDATE_ACTIVITY":
              updateActivityDRPC(value);
              break;
          }
        }
      }
      console.log(`[CORE] Message: ${dataString}`);
    });

    coreModule.stderr.on("data", (data) => {
      console.error(`[CORE] Error: ${data.toString()}`);
    });

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`);

      if (code !== 0) {
        reject(new Error(`Core module exited with code ${code}`));
      }

      resolve(0);
    });
  });

const coreModule = loadCoreModule();

ipcMain.handle("getCoreModulePort", async () => {
  return coreModule;
});

export default coreModule;
