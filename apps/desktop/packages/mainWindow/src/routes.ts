import { lazy } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";
import AboutData from "./pages/about.data";

/* Defining the routes for the application. */
export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("./pages/auth")),
  },
  {
    path: "/home",
    component: lazy(() => import("./pages/home")),
  },
  {
    path: "/about",
    component: lazy(() => import("./pages/about")),
    data: AboutData,
  },
  {
    path: "**",
    component: lazy(() => import("./errors/404")),
  },
];
