// Intentionally putting this on top to catch any potential error in dependencies as well

process.on("uncaughtException", handleUncaughtException);

import {
  app,
  BrowserWindow,
  dialog,
  Menu,
  session,
  shell,
  screen,
  ipcMain,
} from "electron";
import { release } from "os";
import { join, resolve } from "path";
import "./cli";
import coreModule from "./CoreModuleLoaded";
// import autoUpdater from "./autoUpdater";
import "./preloadListeners";
import getAdSize from "./adSize";
import handleUncaughtException from "./handleUncaughtException";

if ((app as any).overwolf) {
  (app as any).overwolf.disableAnonymousAnalytics();
}

// Disable GPU Acceleration for Windows 7
if (release().startsWith("6.1")) app.disableHardwareAcceleration();

// Set application name for Windows 10+ notifications
if (process.platform === "win32") app.setAppUserModelId(app.getName());

if (process.defaultApp) {
  if (process.argv.length >= 2) {
    app.setAsDefaultProtocolClient("gdlauncher", process.execPath, [
      resolve(process.argv[1]),
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
      sandbox: false, // TODO: fix, see https://github.com/electron-react-boilerplate/electron-react-boilerplate/issues/3288
    },
  });

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

  ipcMain.handle("openFileDialog", async (_, filters) => {
    return dialog.showOpenDialog({
      properties: ["openFile"],
      filters,
    });
  });

  if (app.isPackaged) {
    win.loadFile(join(__dirname, "../mainWindow/index.html"));
  } else {
    const url = `http://${import.meta.env.VITE_DEV_SERVER_HOST}:${
      import.meta.env.VITE_DEV_MAIN_WINDOW_PORT
    }`;

    win.loadURL(url, {
      userAgent: "GDLauncher Carbon",
    });
  }

  win.webContents.on("before-input-event", (event, input) => {
    if (input.alt && input.shift && input.code === "KeyI") {
      event.preventDefault();
      console.log("Opening dev tools");
      win?.webContents.openDevTools();
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
          responseHeaders,
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
}

app.whenReady().then(() => {
  console.log("OVERWOLF APP ID", process.env.OVERWOLF_APP_UID);
  session.defaultSession.webRequest.onBeforeSendHeaders(
    {
      urls: ["http://*/*", "https://*/*"],
    },
    (details, callback) => {
      details.requestHeaders["Origin"] = "https://app.gdlauncher.com";
      callback({ requestHeaders: details.requestHeaders });
    }
  );

  session.defaultSession.webRequest.onHeadersReceived(
    {
      urls: ["http://*/*", "https://*/*"],
    },
    (details, callback) => {
      // eslint-disable-next-line
      delete details.responseHeaders!["Access-Control-Allow-Origin"];
      // eslint-disable-next-line
      delete details.responseHeaders!["access-control-allow-origin"];
      details.responseHeaders!["Access-Control-Allow-Origin"] = ["*"];
      callback({
        cancel: false,
        responseHeaders: details.responseHeaders,
      });
    }
  );
  createWindow();
});

app.on("window-all-closed", () => {
  win = null;
  app.quit();
});

app.on("second-instance", (e, argv) => {
  dialog.showErrorBox("Welcome Back", `You arrived from: ${argv}`);
  if (win) {
    // Focus on the main window if the user tried to open another
    if (win.isMinimized()) win.restore();
    win.focus();
  }
});

app.on("open-url", (event, url) => {
  dialog.showErrorBox("Welcome Back", `You arrived from: ${url}`);
});

app.on("activate", () => {
  const allWindows = BrowserWindow.getAllWindows();
  if (allWindows.length) {
    allWindows[0].focus();
  } else {
    createWindow();
  }
});
