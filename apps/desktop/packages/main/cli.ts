import { app } from "electron";
import * as Sentry from "@sentry/electron/main";
import fs from "fs";
import path from "path";
import handleUncaughtException from "./handleUncaughtException";

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
      value: args[args.indexOf(arg) + 1],
    };
  }

  if (args.includes(arg)) {
    return {
      argument: arg,
      value: null,
    };
  }

  return null;
}

if (app.isPackaged) {
  const overrideDataPath = validateArgument("--runtime_path");
  if (overrideDataPath?.value) {
    app.setPath("userData", overrideDataPath?.value);
  } else {
    const userData = path.join(app.getPath("appData"), "gdlauncher_carbon");
    const runtimeOverridePath = path.join(userData, "runtime_path_override");

    const runtimePathExists = fs.existsSync(runtimeOverridePath);

    if (runtimePathExists) {
      const override = fs.readFileSync(runtimeOverridePath);
      app.setPath("userData", override.toString());
    } else {
      app.setPath("userData", userData);
    }
  }
} else {
  const userData = import.meta.env.RUNTIME_PATH;
  if (!userData) {
    throw new Error("Missing runtime path");
  }

  app.setPath("userData", userData);
}
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
      dsn: import.meta.env.VITE_MAIN_DSN,
    });
  }
}
