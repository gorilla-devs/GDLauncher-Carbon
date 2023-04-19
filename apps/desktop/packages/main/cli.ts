import { app } from "electron";
import * as Sentry from "@sentry/electron/main";
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

const overrideDataPath = validateArgument("--gdl_override_data_path");
if (overrideDataPath?.value) {
  app.setPath("userData", overrideDataPath?.value);
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
