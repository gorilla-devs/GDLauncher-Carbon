// @ts-ignore
import * as discordRPC from "discord-rpc";

let client: any | undefined;
let activity: typeof defaultValue | undefined;

const initialAppStartup = Math.floor(Date.now() / 1000);

const defaultValue = {
  details: "Idle",
  startTimestamp: initialAppStartup,
  largeImageKey: "default_big",
  largeImageText: "GDLauncher - A Custom Minecraft Launcher",
  instance: false,
};

export const initDRPC = () => {
  client = new discordRPC.Client({ transport: "ipc" });
  activity = defaultValue;
  client.on("ready", () => {
    console.log("Discord RPC Connected");
    if (!client) return;
    client.setActivity(activity);
  });
  client.login({ clientId: "555898932467597312" }).catch((error: Error) => {
    if (error.message.includes("ENOENT")) {
      console.error("Unable to initialize Discord RPC, no client detected.");
    } else {
      console.error("Unable to initialize Discord RPC:", error);
    }
  });
};

export const updateActivityDRPC = (details: string) => {
  if (!client || !activity) return;
  activity = {
    ...activity,
    startTimestamp: Math.floor(Date.now() / 1000),
    details: `Playing ${details}`,
  };
  client.setActivity(activity);
};

export const stopActivityDRPC = () => {
  if (!client) return;
  activity = defaultValue;
  activity.startTimestamp = initialAppStartup;
  client.setActivity(activity);
};

export const shutdownDRPC = () => {
  if (!client) return;
  client.clearActivity();
  client.destroy();
  client = undefined;
  activity = undefined;
};
