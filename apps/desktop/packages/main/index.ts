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
import {
  setupTitlebar,
  attachTitlebarToWindow,
} from "custom-electron-titlebar/main";
import { release } from "os";
import { join, resolve } from "path";
import "./preloadListeners";

if ((app as any).overwolf) {
  (app as any).overwolf.disableAnonymousAnalytics();
}

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

setupTitlebar();

let win: BrowserWindow | null = null;

const menu = Menu.buildFromTemplate([]);
Menu.setApplicationMenu(menu);

function getAdSize() {
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.workAreaSize;

  if (width <= 800 || height <= 600) {
    // Smaller ads (160/600)
    return {
      minWidth: 760,
      minHeight: 500,
      width: 760,
      height: 500,
      adSize: {
        width: 160,
        height: 600,
      },
    };
  } else if (width < 1000 || height < 800) {
    // Smaller ads (160/600)
    return {
      minWidth: 800,
      minHeight: 600,
      width: 800,
      height: 600,
      adSize: {
        width: 160,
        height: 600,
      },
    };
  } else if (width < 1500 || height < 870) {
    // Smaller ads (160/600)
    return {
      minWidth: 1160,
      minHeight: 670,
      width: 1160,
      height: 670,
      adSize: {
        width: 160,
        height: 600,
      },
    };
  } else {
    return {
      minWidth: 1280,
      minHeight: 740,
      width: 1560,
      height: 740,
      adSize: {
        width: 300,
        height: 250,
      },
    };
  }
}

async function createWindow() {
  const { minWidth, minHeight, width, height } = getAdSize();

  win = new BrowserWindow({
    title: "GDLauncher Carbon",
    minHeight,
    height,
    minWidth,
    width,
    titleBarStyle: "hidden",
    frame: false,
    show: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, "../preload/index.cjs"),
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

  attachTitlebarToWindow(win);

  if (app.isPackaged) {
    win.loadFile(join(__dirname, "../mainWindow/index.html"));
  } else {
    // 🚧 Use ['ENV_NAME'] avoid vite:define plugin
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
    win?.show();
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
