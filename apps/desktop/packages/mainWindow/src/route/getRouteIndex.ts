import { useMatch } from "@solidjs/router";
import { JSXElement } from "solid-js";

export interface Route {
  label: JSXElement;
  path: string;
}

const getRouteIndex = (
  routes: Route[],
  _pathname: string, //  for reactivity
  isExact: boolean = false
) => {
  return routes.findIndex((route) => {
    const matchesBase = useMatch(() => route.path)();
    const matchesChildren = useMatch(() => `${route.path}/*`)();

    if (isExact) {
      return matchesBase;
    }

    return matchesBase || matchesChildren;
  });
};

export default getRouteIndex;
