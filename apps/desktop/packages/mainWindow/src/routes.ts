import { JSX, lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import AboutData from "./pages/about.data";
import Modpacks from "./components/Sidebar/contents/Modpacks";
import Settings from "./components/Sidebar/contents/Settings";

type CustomRouteDefinition = RouteDefinition & {
  component?: () => JSX.Element;
  sidebarComponent?: () => JSX.Element;
  visibileInNavbar?: boolean;
  label?: string;
  children?: CustomRouteDefinition[];
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
    sidebarComponent: Modpacks,
    children: [
      {
        path: "/",
        component: lazy(() => import("./pages/modpacks")),
      },
      {
        path: "/:id",
        component: lazy(() => import("./pages/modpacks/modpack")),
        sidebarComponent: Modpacks,
      },
    ] as CustomRouteDefinition[],
  },
  {
    path: "/settings",
    component: lazy(() => import("./pages/settings")),
    sidebarComponent: Settings,
  },
  {
    path: "**",
    component: lazy(() => import("./errors/404")),
  },
];
