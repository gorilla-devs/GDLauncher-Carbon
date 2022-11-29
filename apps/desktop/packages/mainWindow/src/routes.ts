import { JSX, lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import AboutData from "./pages/about.data";
import Modpacks from "./components/Sidebar/contents/Modpacks";
import Settings from "./components/Sidebar/contents/Settings";

type CustomRouteDefinition = RouteDefinition & {
  component?: () => JSX.Element;
  sidebar?: () => JSX.Element;
  visibileInNavbar?: boolean;
  label?: string;
  children?: CustomRouteDefinition | CustomRouteDefinition[];
};

/* Defining the routes for the application. */
export const routes: CustomRouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("./pages/login")),
  },
  {
    label: "Library",
    visibileInNavbar: true,
    path: "/library",
    children: [
      {
        path: "/",
        component: lazy(() => import("./pages/library")),
      },
      {
        path: "/:id",
        component: lazy(() => import("./pages/library/instace")),
      },
    ],
  },
  {
    label: "Modpacks",
    visibileInNavbar: true,
    path: "/modpacks",
    data: AboutData,
    sidebar: Modpacks,
    children: [
      {
        path: "/",
        component: lazy(() => import("./pages/modpacks")),
      },
      {
        path: "/:id",
        component: lazy(() => import("./pages/modpacks/modpack")),
        sidebar: Modpacks,
      },
    ],
  },
  {
    path: "/settings",
    component: lazy(() => import("./pages/settings")),
    sidebar: Settings,
  },
  {
    path: "**",
    component: lazy(() => import("./errors/404")),
  },
];
