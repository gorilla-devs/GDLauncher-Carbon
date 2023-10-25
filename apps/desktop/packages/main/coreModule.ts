import path from "path";
import os from "os";
import { spawn } from "child_process";
import type { ChildProcessWithoutNullStreams } from "child_process";
import { app, ipcMain } from "electron";

export enum KnownError {
  // eslint-disable-next-line no-unused-vars
  MigrationFailed = "MigrationFailed"
}

export type Log = {
  type: "info" | "error";
  message: string;
};

export type CoreModuleError = {
  logs: Log[];
  knownError: KnownError | null;
};

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
        kill: () => {}
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
            RUST_BACKTRACE: "full"
          }
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

    let logs: Log[] = [];
    coreModule.stdout.on("data", (data) => {
      let dataString = data.toString();
      let rows = dataString.split(/\r?\n|\r|\n/g);

      logs.push({
        type: "info",
        message: dataString
      });

      for (let row of rows) {
        if (row.startsWith("_STATUS_:")) {
          const port: number = row.split("|")[1];
          console.log(`[CORE] Port: ${port}`);
          resolve({
            port,
            kill: () => coreModule?.kill()
          });
        }
      }
      console.log(`[CORE] Message: ${dataString}`);
    });

    coreModule.stderr.on("data", (data) => {
      logs.push({
        type: "error",
        message: data.toString()
      });
      console.error(`[CORE] Error: ${data.toString()}`);
    });

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`);

      if (code !== 0) {
        reject({
          logs,
          knownError: mapLogsToKnownError(logs)
        } as CoreModuleError);
      }

      resolve({
        port: 0,
        kill: () => coreModule?.kill()
      });
    });
  });

function mapLogsToKnownError(logs: Log[]): KnownError | null {
  if (logs.some((log) => log.message.includes("[_GDL_DB_MIGRATION_FAILED_]"))) {
    return KnownError.MigrationFailed;
  }

  return null;
}

const coreModule = loadCoreModule();

ipcMain.handle("getCoreModulePort", async () => {
  return (await coreModule).port;
});

export default coreModule;
