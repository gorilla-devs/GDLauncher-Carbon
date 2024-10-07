// Intentionally putting this on top to catch any potential error in dependencies as well

process.on("uncaughtException", handleUncaughtException);

import {
  app,
  BrowserWindow,
  dialog,
  Display,
  ipcMain,
  OpenDialogOptions,
  SaveDialogOptions,
  screen,
  session,
  shell
} from "electron";
import os, { platform, release } from "os";
import path, { join, resolve } from "path";
import fs from "fs/promises";
import fss from "fs";
import fse from "fs-extra";
import type { ChildProcessWithoutNullStreams } from "child_process";
import { spawn } from "child_process";
import crypto from "crypto";
import log from "electron-log/main";
import * as Sentry from "@sentry/electron/main";
import "./preloadListeners";
import getAdSize from "./adSize";
import handleUncaughtException from "./handleUncaughtException";
import initAutoUpdater from "./autoUpdater";
import "./appMenu";
import { FELauncherActionOnGameLaunch } from "@gd/core_module/bindings";

const timeStart = Date.now();
let isPotatoPcModeSet = false;

export const RUNTIME_PATH_OVERRIDE_NAME = "runtime_path_override";
const RUNTIME_PATH_DEFAULT_NAME = "data";

export let CURRENT_RUNTIME_PATH: string | null = null;

let win: BrowserWindow | null = null;

let isGameRunning = false;
let showAppCloseWarning = true;

export function initRTPath(override: string | null | undefined) {
  if (override) {
    CURRENT_RUNTIME_PATH = override;
    return;
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
    RUNTIME_PATH_OVERRIDE_NAME
  );

  let file_override: string | null = null;
  try {
    const tmp_path = fss.readFileSync(runtimeOverridePath).toString();
    fse.ensureDirSync(tmp_path);
    file_override = tmp_path;
  } catch {
    // ignore
  }

  CURRENT_RUNTIME_PATH =
    file_override ||
    path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME);
}

const args = process.argv.slice(1);

type Argument = {
  argument: string;
  value: string | null;
};

function validateArgument(arg: string): Argument | null {
  const hasValue =
    args.includes(arg) && !args[args.indexOf(arg) + 1]?.startsWith("--");

  if (hasValue) {
    return {
      argument: arg,
      value: args[args.indexOf(arg) + 1]
    };
  }

  if (args.includes(arg)) {
    return {
      argument: arg,
      value: null
    };
  }

  return null;
}

export function getPatchedUserData() {
  let appData = null;

  if (os.platform() === "darwin" || os.platform() === "win32") {
    appData = app.getPath("appData");
  } else {
    // monkey patch linux since it defaults to .config instead of .local/share
    const xdgDataHome = process.env.XDG_DATA_HOME;
    if (xdgDataHome) {
      appData = xdgDataHome;
    } else {
      const homeDir = os.homedir();
      appData = path.join(homeDir, ".local/share");
    }
  }

  return path.join(appData, "gdlauncher_carbon");
}

const skipIntroAnimation = fss.existsSync(getPatchedUserData());

app.setPath("userData", getPatchedUserData());

if (!process.env.CI) {
  Object.assign(console, log.functions);
}

log.transports.file.resolvePathFn = (variables) =>
  path.join(getPatchedUserData(), variables.fileName!);
log.initialize();
log.eventLogger.startLogging();

if (app.isPackaged) {
  const overrideCLIDataPath = validateArgument("--runtime_path");
  const overrideEnvDataPath = process.env.GDL_RUNTIME_PATH;

  initRTPath(overrideCLIDataPath?.value || overrideEnvDataPath);
} else {
  const rtPath = import.meta.env.RUNTIME_PATH;
  if (!rtPath) {
    throw new Error("Missing runtime path");
  }
  initRTPath(rtPath);
}

console.log("Userdata path:", app.getPath("userData"));
console.log("Runtime path:", CURRENT_RUNTIME_PATH);

const sentrySessionId = crypto.randomUUID();

console.log("SENTRY SESSION ID", sentrySessionId);

const allowMultipleInstances = validateArgument(
  "--gdl_allow_multiple_instances"
);

const overrideBaseApi = validateArgument("--gdl_override_base_api");

if (!allowMultipleInstances) {
  if (!app.requestSingleInstanceLock()) {
    app.quit();
    process.exit(0);
  }
}

const disableSentry = validateArgument("--gdl_disable_sentry");

if (!disableSentry) {
  if (import.meta.env.VITE_MAIN_DSN) {
    // @ts-ignore
    process.removeListener("uncaughtException", handleUncaughtException);

    Sentry.init({
      dsn: import.meta.env.VITE_MAIN_DSN,
      release: __APP_VERSION__,
      dist: os.platform()
    });

    Sentry.setContext("session", {
      gdl_session_id: sentrySessionId
    });
  }
}

function maybeDisableGPU(override: boolean) {
  if (app.isReady()) {
    console.error("App is ready, cannot disable GPU");
    return;
  }

  const disableGPU = validateArgument("--disable-gpu") || override;

  if (disableGPU) {
    app.commandLine.appendSwitch("no-sandbox");
    app.commandLine.appendSwitch("disable-gpu");
    app.commandLine.appendSwitch("disable-software-rasterizer");
    app.commandLine.appendSwitch("disable-gpu-compositing");
    app.commandLine.appendSwitch("disable-gpu-rasterization");
    app.commandLine.appendSwitch("disable-gpu-sandbox");
    app.commandLine.appendSwitch("--no-sandbox");
  }

  // Disable GPU Acceleration for Windows 7
  if (disableGPU || (release().startsWith("6.1") && platform() === "win32")) {
    app.disableHardwareAcceleration();
  }
}

maybeDisableGPU(false);

export type Log = {
  type: "info" | "error";
  message: string;
};

const isDev = import.meta.env.MODE === "development";

const binaryName =
  os.platform() === "win32" ? "core_module.exe" : "core_module";

export type CoreModule = () => Promise<
  | {
      type: "success";
      result: {
        port: number;
        kill: () => void;
      };
    }
  | {
      type: "error";
      logs: Log[];
    }
>;

const loadCoreModule: CoreModule = () =>
  new Promise((resolve, _) => {
    if (isDev) {
      resolve({
        type: "success",
        result: {
          port: 4650,
          kill: () => {}
        }
      });
      return;
    }

    let started = false;

    const coreModulePath = path.resolve(
      __dirname,
      "../../../../resources/binaries",
      binaryName
    );

    console.log(`[CORE] Spawning core module: ${coreModulePath}`);
    let coreModule: ChildProcessWithoutNullStreams | null = null;
    let logs: Log[] = [];

    const args = ["--runtime_path", CURRENT_RUNTIME_PATH!];

    if (overrideBaseApi?.value) {
      args.push("--base_api", overrideBaseApi.value!);
    }

    try {
      coreModule = spawn(coreModulePath, args, {
        shell: false,
        detached: false,
        stdio: "pipe",
        env: {
          ...process.env,
          RUST_BACKTRACE: "full"
        }
      });
    } catch (err) {
      console.error(`[CORE] Spawn error: ${err}`);

      logs.push({
        type: "error",
        message: (err as unknown as string).toString()
      });

      resolve({
        type: "error",
        logs
      });

      return;
    }

    coreModule.on("error", function (err) {
      console.error(`[CORE] Spawn error: ${err}`);

      logs.push({
        type: "error",
        message: err.toString()
      });

      resolve({
        type: "error",
        logs
      });

      return;
    });

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

          started = true;

          resolve({
            type: "success",
            result: {
              port,
              kill: () => coreModule?.kill()
            }
          });
        } else if (row.startsWith("_INSTANCE_STATE_:")) {
          const rightPart = row.split(":")[1];
          const event = rightPart.split("|")[0];
          const action: FELauncherActionOnGameLaunch = rightPart.split("|")[1];

          if (event === "GAME_LAUNCHED") {
            isGameRunning = true;
            switch (action) {
              case "closeWindow":
                win?.close();
                win = null;
                break;
              case "hideWindow":
                win?.hide();
                break;
              case "minimizeWindow":
                win?.minimize();
                break;
              case "none":
                break;
              case "quitApp":
                showAppCloseWarning = false;
                app.quit();
                break;
            }
          } else if (event === "GAME_CLOSED") {
            isGameRunning = false;
            switch (action) {
              case "closeWindow":
                if (!win || win.isDestroyed()) {
                  createWindow();
                }
                break;
              case "hideWindow":
              case "minimizeWindow":
                if (win && !win.isDestroyed()) {
                  win?.show();
                  win?.focus();
                } else {
                  createWindow();
                }
                break;
              case "none":
                break;
              case "quitApp":
                // There's nothing we can do
                break;
            }
          }
        } else if (row.startsWith("_SHOW_APP_CLOSE_WARNING_:")) {
          const rightPart = row.split(":")[1];
          showAppCloseWarning = rightPart === "true";
        } else if (row.startsWith("_POTATO_PC_MODE_:")) {
          isPotatoPcModeSet = true;
          const rightPart = row.split(":")[1];
          if (rightPart === "true") {
            maybeDisableGPU(true);
          }
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
        resolve({
          type: "error",
          logs
        });
      }

      resolve({
        type: "success",
        result: {
          port: 0,
          kill: () => coreModule?.kill()
        }
      });
    });

    setTimeout(
      () => {
        if (coreModule?.killed || started) {
          return;
        }

        console.error(`[CORE] Took too long to start`);

        Sentry.captureException(
          new Error("Core module took too long to start")
        );

        resolve({
          type: "error",
          logs
        });
      },
      60 * 5 * 1000
    );
  });

const coreModule = loadCoreModule();

if ((app as any).overwolf) {
  (app as any).overwolf.disableAnonymousAnalytics();
}

// Set application name for Windows 10+ notifications
if (process.platform === "win32") app.setAppUserModelId(app.getName());

if (process.defaultApp) {
  if (process.argv.length >= 2) {
    app.setAsDefaultProtocolClient("gdlauncher", process.execPath, [
      resolve(process.argv[1])
    ]);
  }
} else {
  app.setAsDefaultProtocolClient("gdlauncher");
}

let lastDisplay: Display | null = null;

let isSpawningWindow = false;

async function createWindow(): Promise<BrowserWindow> {
  if (isSpawningWindow) {
    return win!;
  }

  isSpawningWindow = true;

  const currentDisplay = screen.getPrimaryDisplay();
  lastDisplay = currentDisplay;
  const { minWidth, minHeight, height, width } = getAdSize(currentDisplay);

  if (!win || win.isDestroyed()) {
    win?.close();
    win?.destroy();
    win = null;
  }

  win = new BrowserWindow({
    title: "GDLauncher Carbon",
    minHeight,
    height,
    minWidth,
    width,
    titleBarStyle: "default",
    frame: true,
    show: true,
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, "../preload/index.cjs"),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: app.isPackaged,
      webSecurity: true,
      additionalArguments: [`--skip-intro-animation=${skipIntroAnimation}`]
    }
  });

  win.on("move", () => {
    const bounds = win?.getBounds();

    if (!bounds) {
      return;
    }

    const currentDisplay = screen.getDisplayMatching(bounds);
    if (lastDisplay?.id === currentDisplay?.id) {
      return;
    }

    lastDisplay = currentDisplay;
    const { minWidth, minHeight, adSize } = getAdSize(currentDisplay);
    win?.setMinimumSize(minWidth, minHeight);
    win?.setSize(minWidth, minHeight);
    win?.webContents?.send("adSizeChanged", adSize);
  });

  win.on("close", (e) => {
    if (!isGameRunning) {
      return;
    }

    if (showAppCloseWarning) {
      e.preventDefault();
      win?.webContents.send("showAppCloseWarning");
    }
  });

  win.webContents.on("will-navigate", (e, url) => {
    if (win && !win.isDestroyed() && url !== win.webContents.getURL()) {
      e.preventDefault();
      shell.openExternal(url);
    }
  });

  if (app.isPackaged) {
    win.loadFile(join(__dirname, "../mainWindow/index.html"));
  } else {
    const url = `http://${import.meta.env.VITE_DEV_SERVER_HOST}:${
      import.meta.env.VITE_DEV_MAIN_WINDOW_PORT
    }`;

    win.loadURL(url, {
      userAgent: "GDLauncher Carbon"
    });
  }

  win.webContents.on("before-input-event", (event, input) => {
    if (input.alt && input.shift && input.code === "KeyI") {
      event.preventDefault();
      console.log("dev tools open:", win?.webContents.isDevToolsOpened());
      win?.webContents.toggleDevTools();
    }
  });

  win.on("ready-to-show", () => {
    isSpawningWindow = false;

    coreModule.finally(() => {
      win?.show();
    });

    function upsertKeyValue(obj: any, keyToChange: string, value: any) {
      const keyToChangeLower = keyToChange.toLowerCase();
      for (const key of Object.keys(obj)) {
        if (key.toLowerCase() === keyToChangeLower) {
          return;
        }
      }
      // Insert at end instead
      obj[keyToChange] = value;
    }

    win?.webContents.session.webRequest.onBeforeSendHeaders(
      (details, callback) => {
        const { requestHeaders } = details;
        upsertKeyValue(requestHeaders, "Access-Control-Allow-Origin", ["*"]);
        callback({ requestHeaders });
      }
    );

    win?.webContents.session.webRequest.onHeadersReceived(
      (details, callback) => {
        const { responseHeaders } = details;
        upsertKeyValue(responseHeaders, "Access-Control-Allow-Origin", ["*"]);
        upsertKeyValue(responseHeaders, "Access-Control-Allow-Headers", ["*"]);
        callback({
          responseHeaders
        });
      }
    );

    if (import.meta.env.DEV) {
      win?.webContents.openDevTools();
    }
  });

  // Make all links open with the browser, not with the application
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith("https:")) shell.openExternal(url);
    return { action: "deny" };
  });

  return win;
}

// Handlers
ipcMain.handle("relaunch", async () => {
  console.info("relaunching app...");

  try {
    let _coreModule = await coreModule;
    if (_coreModule.type === "success") {
      _coreModule.result.kill();
    }
  } catch {
    // No op
  }

  app.relaunch();
  app.exit();
});

ipcMain.handle("getAdSize", async () => {
  const currentDisplay = screen.getDisplayMatching(win?.getBounds()!);
  return getAdSize(currentDisplay).adSize;
});

ipcMain.handle("openFileDialog", async (_, opts: OpenDialogOptions) => {
  return dialog.showOpenDialog(opts);
});

ipcMain.handle("showSaveDialog", async (_, opts: SaveDialogOptions) => {
  return dialog.showSaveDialog(opts);
});

ipcMain.handle("getCurrentOS", async () => {
  return { platform: os.platform(), arch: os.arch() };
});

ipcMain.handle("openFolder", async (_, path) => {
  shell.showItemInFolder(path);
});

ipcMain.handle("openCMPWindow", async () => {
  // @ts-ignore
  if (app.overwolf.openCMPWindow) {
    // @ts-ignore
    app.overwolf.openCMPWindow();
  }
});

ipcMain.handle("closeWindow", async () => {
  win?.close();
  win?.destroy();
});

ipcMain.handle("getUserData", async () => {
  return app.getPath("userData");
});

ipcMain.handle("getInitialRuntimePath", async () => {
  return path.join(app.getPath("userData"), RUNTIME_PATH_DEFAULT_NAME);
});

ipcMain.handle("getRuntimePath", async () => {
  return CURRENT_RUNTIME_PATH;
});

ipcMain.handle("changeRuntimePath", async (_, newPath: string) => {
  if (newPath === CURRENT_RUNTIME_PATH) {
    return;
  }

  const runtimeOverridePath = path.join(
    app.getPath("userData"),
    RUNTIME_PATH_OVERRIDE_NAME
  );

  await fs.mkdir(newPath, { recursive: true });

  try {
    const cm = await coreModule;
    if (cm.type === "success") {
      cm.result.kill();
    }
  } catch {
    // No op
  }

  // TODO: Copy with progress
  await fse.copy(CURRENT_RUNTIME_PATH!, newPath, {
    overwrite: true,
    errorOnExist: false
  });

  await fs.writeFile(runtimeOverridePath, newPath);

  await fse.remove(CURRENT_RUNTIME_PATH!);

  // TODO: with a bit of work we can change the RTPath without actually restarting the app
  app.relaunch();
  app.exit();
});

ipcMain.handle("validateRuntimePath", async (_, newPath: string | null) => {
  if (!newPath || newPath === CURRENT_RUNTIME_PATH) {
    return false;
  }

  const pathExists = await fse.pathExists(newPath);
  if (!pathExists) {
    return true;
  }

  const newPathStat = await fs.stat(newPath);
  if (!newPathStat.isDirectory()) {
    return false;
  }

  const files = await fs.readdir(newPath);
  if (files.length > 0) {
    return false;
  }

  return true;
});

ipcMain.handle("getCoreModule", async () => {
  // we can assume this promise never rejects
  const cm = await coreModule;

  return {
    type: cm.type,
    logs: cm.type === "error" ? cm.logs : undefined,
    port: cm.type === "success" ? cm.result.port : undefined
  };
});

app.whenReady().then(async () => {
  const accessibility = validateArgument("--enable-accessibility");

  if (accessibility) {
    app.setAccessibilitySupportEnabled(true);
  }

  console.log("OVERWOLF APP ID", process.env.OVERWOLF_APP_UID);
  session.defaultSession.webRequest.onBeforeSendHeaders(
    {
      urls: ["http://*/*", "https://*/*"]
    },
    (details, callback) => {
      details.requestHeaders["Origin"] = "https://app.gdlauncher.com";
      callback({ requestHeaders: details.requestHeaders });
    }
  );

  session.defaultSession.webRequest.onHeadersReceived(
    {
      urls: ["http://*/*", "https://*/*"]
    },
    (details, callback) => {
      // eslint-disable-next-line
      delete details.responseHeaders!["Access-Control-Allow-Origin"];
      // eslint-disable-next-line
      delete details.responseHeaders!["access-control-allow-origin"];
      details.responseHeaders!["Access-Control-Allow-Origin"] = ["*"];
      callback({
        cancel: false,
        responseHeaders: details.responseHeaders
      });
    }
  );

  app.on("second-instance", (_e, _argv) => {
    if (win && !win.isDestroyed()) {
      // Focus on the main window if the user tried to open another
      if (win.isMinimized()) win.restore();
      win.focus();
    } else {
      createWindow();
    }
  });

  app.on("activate", () => {
    if (!win || win.isDestroyed()) {
      createWindow();
    }
  });

  await createWindow();

  screen.addListener(
    "display-metrics-changed",
    (_, display, changedMetrics) => {
      const bounds = win?.getBounds();

      if (!bounds) {
        return;
      }

      const currentDisplay = screen.getDisplayMatching(bounds);
      if (lastDisplay?.id === currentDisplay?.id) {
        return;
      }

      lastDisplay = currentDisplay;

      const { minWidth, minHeight } = getAdSize(currentDisplay);
      if (changedMetrics.includes("workArea")) {
        win?.setMinimumSize(minWidth, minHeight);
        win?.setSize(minWidth, minHeight);
        win?.webContents.send("adSizeChanged", getAdSize().adSize);
      }
    }
  );

  initAutoUpdater(win);
});

app.on("window-all-closed", async () => {
  if (isSpawningWindow) {
    return;
  }

  try {
    let _coreModule = await coreModule;
    if (_coreModule.type === "success") {
      _coreModule.result.kill();
    }
  } catch {
    // No op
  }

  if (win && !win.isDestroyed()) {
    win.close();
    win.destroy();
  }

  win = null;
  app.quit();
});

app.on("before-quit", async () => {
  try {
    let _coreModule = await coreModule;
    if (_coreModule.type === "success") {
      _coreModule.result.kill();
    }
  } catch {
    // No op
  }
});

app.on("render-process-gone", (event, webContents, detailed) => {
  console.error("render-process-gone", detailed);
  webContents.reload();
});

app.on("open-url", (event, url) => {
  dialog.showErrorBox("Welcome Back", `You arrived from: ${url}`);
});

const LOOP_TIMEOUT = 4000;

// keep event loop busy until potato pc mode is set or timeout is reached
if (!isPotatoPcModeSet) {
  let timeEnd = Date.now();
  while (!isPotatoPcModeSet && timeEnd - timeStart < LOOP_TIMEOUT) {
    timeEnd = Date.now();
  }

  // DO NOT REMOVE THIS CONSOLE LOG as V8 optimizes the loop away
  console.log("First event loop tick done in ", timeEnd - timeStart);
}
