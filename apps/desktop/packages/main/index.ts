// Intentionally putting this on top to catch any potential error in dependencies as well

process.on("uncaughtException", handleUncaughtException);

import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  OpenDialogOptions,
  screen,
  session,
  shell
} from "electron";
import os, { platform, release } from "os";
import { join, resolve } from "path";
import "./runtimePath";
import "./cli"; // THIS MUST BE BEFORE "coreModule" IMPORT!
import coreModule from "./coreModule";
import "./preloadListeners";
import getAdSize from "./adSize";
import handleUncaughtException from "./handleUncaughtException";
import initAutoUpdater from "./autoUpdater";
import "./appMenu";

if ((app as any).overwolf) {
  (app as any).overwolf.disableAnonymousAnalytics();
}

// Disable GPU Acceleration for Windows 7
if (release().startsWith("6.1") && platform() === "win32") {
  app.disableHardwareAcceleration();
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

let win: BrowserWindow | null = null;

const menu = Menu.buildFromTemplate([]);
Menu.setApplicationMenu(menu);

async function createWindow() {
  const { minWidth, minHeight, width, height } = getAdSize();

  win = new BrowserWindow({
    title: "GDLauncher Carbon",
    minHeight,
    height,
    minWidth,
    width,
    titleBarStyle: "default",
    frame: true,
    show: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, "../preload/index.cjs"),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: app.isPackaged
    }
  });

  initAutoUpdater(win);

  screen.addListener(
    "display-metrics-changed",
    (_, display, changedMetrics) => {
      const { minWidth, minHeight } = getAdSize();
      if (changedMetrics.includes("workArea")) {
        win?.setMinimumSize(minWidth, minHeight);
        win?.setSize(minWidth, minHeight);
        win?.webContents.send("adSizeChanged", getAdSize().adSize);
      }
    }
  );

  // Handlers
  ipcMain.handle("getAdSize", async () => {
    return getAdSize().adSize;
  });

  ipcMain.handle("openFileDialog", async (_, opts: OpenDialogOptions) => {
    return dialog.showOpenDialog(opts);
  });

  ipcMain.handle("getCurrentOS", async () => {
    return { platform: os.platform(), arch: os.arch() };
  });

  ipcMain.handle("openFolder", async (_, path) => {
    shell.openPath(path);
  });

  ipcMain.handle("openCMPWindow", async () => {
    // @ts-ignore
    app.overwolf.openCMPWindow();
  });

  win.webContents.on("will-navigate", (e, url) => {
    if (win && url !== win.webContents.getURL()) {
      e.preventDefault();
      shell.openExternal(url);
    }
  });

  win.webContents.on("render-process-gone", (event, detailed) => {
    console.log("render-process-gone", detailed);
    if (detailed.reason === "crashed") {
      win?.webContents.reload();
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

  ipcMain.handle("relaunch", () => {
    console.info("relaunching app...");

    app.relaunch();
    app.exit();
  });
}

app.whenReady().then(() => {
  // Expose chrome's accessibility tree by default
  app.setAccessibilitySupportEnabled(true);

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
  createWindow();
});

app.on("window-all-closed", async () => {
  try {
    let _coreModule = await coreModule;
    _coreModule.kill();
  } catch {
    // No op
  }
  win = null;
  app.quit();
});

app.on("second-instance", (_e, _argv) => {
  if (win) {
    // Focus on the main window if the user tried to open another
    if (win.isMinimized()) win.restore();
    win.focus();
  }
});

app.on("open-url", (event, url) => {
  dialog.showErrorBox("Welcome Back", `You arrived from: ${url}`);
});
