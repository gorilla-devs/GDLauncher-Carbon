import { lazy } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";
import AboutData from "./pages/about.data";

/* Defining the routes for the application. */
export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("./pages/login")),
  },
  {
    path: "/library",
    component: lazy(() => import("./pages/library")),
  },
  {
    path: "/library/:id",
    component: lazy(() => import("./pages/library/instace")),
  },
  {
    path: "/modpacks",
    component: lazy(() => import("./pages/modpacks")),
    data: AboutData,
  },
  {
    path: "**",
    component: lazy(() => import("./errors/404")),
  },
];
