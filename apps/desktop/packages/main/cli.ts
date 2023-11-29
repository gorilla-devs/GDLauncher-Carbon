import { app } from "electron";
import * as Sentry from "@sentry/electron/main";
import os from "os";
import path from "path";
import handleUncaughtException from "./handleUncaughtException";
import { CURRENT_RUNTIME_PATH, initRTPath } from "./runtimePath";

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

  if (os.platform() !== "linux") {
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

app.setPath("userData", getPatchedUserData());

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

const allowMultipleInstances = validateArgument(
  "--gdl_allow_multiple_instances"
);

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
      dsn: import.meta.env.VITE_MAIN_DSN
    });
  }
}
