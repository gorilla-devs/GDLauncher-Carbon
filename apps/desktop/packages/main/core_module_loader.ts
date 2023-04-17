import path from "path";
import os from "os";
import { spawn } from "child_process";

const isDev = import.meta.env.MODE === "development";

const binaryName =
  os.platform() === "win32" ? "core_module.exe" : "core_module";

type CoreModule = () => Promise<number>;

type Log = {
  type: "info" | "error";
  message: string;
};

type CoreModuleError = {
  exitCode: number | null;
  message: string;
  logs: Log[];
};

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

    const coreModule = spawn(coreModulePath, [], {
      shell: false,
      detached: false,
      env: {
        RUST_BACKTRACE: "full",
      },
    });
    let logs: Log[] = [];

    coreModule.stdout.on("data", (data) => {
      let dataString = data.toString();
      logs.push({
        type: "info",
        message: dataString,
      });
      if (dataString.startsWith("[_GDL_STATUS_]: ")) {
        let port = dataString.split("|")[1];
        resolve(port);
      }
      console.log(`[CORE] Message: ${dataString}`);
    });

    coreModule.stderr.on("data", (data) => {
      logs.push({
        type: "error",
        message: data.toString(),
      });
      console.error(`[CORE] Error: ${data.toString()}`);
    });

    coreModule.on("exit", (code) => {
      console.log(`[CORE] Exit with code: ${code}`);

      if (code !== 0) {
        reject({
          exitCode: code,
          message: "Core module failed to start",
          logs,
        } as CoreModuleError);
      }

      resolve(0);
    });

    setTimeout(() => {
      reject({
        message: "Core module failed to start",
        exitCode: null,
        logs,
      } as CoreModuleError);
    }, 10000);
  });

export default loadCoreModule;
