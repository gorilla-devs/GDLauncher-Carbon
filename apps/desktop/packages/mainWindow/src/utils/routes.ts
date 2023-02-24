import { BeforeLeaveEventArgs, useBeforeLeave } from "@solidjs/router";
import { createSignal } from "solid-js";

export const [lastInstanceOpened, setLastInstanceOpened] = createSignal("");

export const libraryUrlRegex = /\/library\/(\w+)/;

export const handleRouteChange = () => {
  useBeforeLeave((e: BeforeLeaveEventArgs) => {
    const instaceUrlRegex = e.to.toString().match(libraryUrlRegex);
    const instanceId = instaceUrlRegex?.[1];

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
