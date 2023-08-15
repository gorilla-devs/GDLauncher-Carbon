import path from "path";
import os from "os";
import { spawn } from "child_process";
import type { ChildProcessWithoutNullStreams } from "child_process";
import { app, ipcMain } from "electron";

const isDev = import.meta.env.MODE === "development";

const binaryName =
  os.platform() === "win32" ? "core_module.exe" : "core_module";

type CoreModule = () => Promise<{
  port: number;
  kill: () => void;
}>;

const loadCoreModule: CoreModule = () =>
  new Promise((resolve, reject) => {
    if (isDev) {
      resolve({
        port: 4650,
        kill: () => {},
      });
      return;
    }

    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources/binaries",
      binaryName
    );

    console.log(`[CORE] Spawning core module: ${coreModulePath}`);
    let coreModule: ChildProcessWithoutNullStreams | null = null;

    try {
      coreModule = spawn(
        coreModulePath,
        ["--runtime_path", app.getPath("userData")],
        {
          shell: false,
          detached: false,
          stdio: "pipe",
          env: {
            ...process.env,
            RUST_BACKTRACE: "full",
          },
        }
      );
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
          const port: number = row.split("|")[1];
          console.log(`[CORE] Port: ${port}`);
          resolve({
            port,
            kill: () => coreModule?.kill(),
          });
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

      resolve({
        port: 0,
        kill: () => coreModule?.kill(),
      });
    });
  });

const coreModule = loadCoreModule();

ipcMain.handle("getCoreModulePort", async () => {
  return (await coreModule).port;
});

export default coreModule;
