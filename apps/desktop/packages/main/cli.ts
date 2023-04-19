import { app } from "electron";

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
