import { BeforeLeaveEventArgs, useBeforeLeave } from "@solidjs/router";
import { createSignal } from "solid-js";

export const [lastInstanceOpened, setLastInstanceOpened] = createSignal("");

export const libraryPathRegex = /\/library\/(\w+)/;

export const getInstanceIdFromPath = (path: string) => {
  const instaceUrlRegex = path.match(libraryPathRegex);
  const instanceId = instaceUrlRegex?.[1];
  return instanceId;
};

export const handleRouteChange = () => {
  useBeforeLeave((e: BeforeLeaveEventArgs) => {
    const instanceId = getInstanceIdFromPath(e.to.toString());

    if (instanceId) setLastInstanceOpened(instanceId);
  });
};

export const composePathUrl = (route: { label: string; path: string }) => {
  switch (route.label) {
    case "library": {
      return `${route.path}/${lastInstanceOpened()}`;
    }
    default:
      return route.path;
  }
};
