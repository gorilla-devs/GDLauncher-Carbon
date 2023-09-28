import { app } from "electron";
import * as Sentry from "@sentry/electron/main";
import fs from "fs";
import path from "path";
import handleUncaughtException from "./handleUncaughtException";
import {
  RUNTIME_PATH_OVERRIDE_NAME,
  getInitialRTPath,
  setCurrentRTPath,
} from "./runtimePath";

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
    setCurrentRTPath(overrideDataPath?.value);
  } else {
    const initialRTPath = getInitialRTPath();

    const runtimeOverridePath = path.join(
      initialRTPath,
      RUNTIME_PATH_OVERRIDE_NAME
    );

    const runtimePathExists = fs.existsSync(runtimeOverridePath);

    let override: string | null = null;
    if (runtimePathExists) {
      override = fs.readFileSync(runtimeOverridePath).toString();
    }

    setCurrentRTPath(override || initialRTPath);
  }
} else {
  const rtPath = import.meta.env.RUNTIME_PATH;
  if (!rtPath) {
    throw new Error("Missing runtime path");
  }

  setCurrentRTPath(rtPath);
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
