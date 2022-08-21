import { app, BrowserWindow, dialog, session, shell } from "electron";
import { release } from "os";
import { join, resolve } from "path";
import { autoUpdater } from "electron-updater";

// Disable GPU Acceleration for Windows 7
if (release().startsWith("6.1")) app.disableHardwareAcceleration();

// Set application name for Windows 10+ notifications
if (process.platform === "win32") app.setAppUserModelId(app.getName());

if (!app.requestSingleInstanceLock()) {
  app.quit();
  process.exit(0);
}

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

async function createWindow() {
  win = new BrowserWindow({
    title: "Main window",
    webPreferences: {
      preload: join(__dirname, "../preload/index.cjs"),
      sandbox: false, // TODO: fix, see https://github.com/electron-react-boilerplate/electron-react-boilerplate/issues/3288
    },
  });

  if (app.isPackaged) {
    win.loadFile(join(__dirname, "../mainWindow/index.html"));
  } else {
    // 🚧 Use ['ENV_NAME'] avoid vite:define plugin
    const url = `http://${process.env["VITE_DEV_SERVER_HOST"]}:${process.env["VITE_DEV_MAIN_WINDOW_PORT"]}`;

    win.loadURL(url, {
      userAgent: "GDLauncher Carbon",
    });
  }
  win.webContents.openDevTools();

  // Test active push message to Renderer-process
  win.webContents.on("did-finish-load", () => {
    win?.webContents.send("main-process-message", new Date().toLocaleString());
  });

  // Make all links open with the browser, not with the application
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith("https:")) shell.openExternal(url);
    return { action: "deny" };
  });
}

app.whenReady().then(() => {
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
  dialog.showErrorBox('Welcome Back', `You arrived from: ${argv}`)
  if (win) {
    // Focus on the main window if the user tried to open another
    if (win.isMinimized()) win.restore();
    win.focus();
  }
});

app.on('open-url', (event, url) => {
  dialog.showErrorBox('Welcome Back', `You arrived from: ${url}`)
})

app.on("activate", () => {
  const allWindows = BrowserWindow.getAllWindows();
  if (allWindows.length) {
    allWindows[0].focus();
  } else {
    createWindow();
  }
});
