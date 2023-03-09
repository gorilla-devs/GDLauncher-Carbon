import { createSignal } from "solid-js";

export const [lastInstanceOpened, setLastInstanceOpened] = createSignal("");
export const [loggedOut, setLoggedOut] = createSignal(false);

export const libraryPathRegex = /^\/library(\/\w+)?\/?$/;

export const getInstanceIdFromPath = (path: string) => {
  const instaceUrlRegex = path.match(libraryPathRegex);
  const instanceId = instaceUrlRegex?.[1];
  return instanceId?.replace("/", "");
};

export const isLibraryPath = (path: string) => {
  return libraryPathRegex.test(path);
};
