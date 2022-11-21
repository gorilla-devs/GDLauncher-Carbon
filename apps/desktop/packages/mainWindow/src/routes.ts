import { lazy } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";
import AboutData from "./pages/about.data";
import Modpacks from "./components/Sidebar/contents/modpacks";

/* Defining the routes for the application. */
export const routes = [
  {
    path: "/",
    component: lazy(() => import("./pages/login")),
  },
  {
    label: "Library",
    visibileInNavbar: true,
    path: "/library",
    component: lazy(() => import("./pages/library")),
  },
  {
    path: "/library/:id",
    component: lazy(() => import("./pages/library/instace")),
  },
  {
    label: "Modpacks",
    visibileInNavbar: true,
    path: "/modpacks",
    component: lazy(() => import("./pages/modpacks")),
    data: AboutData,
    sidebar: Modpacks,
  },
  {
    path: "**",
    component: lazy(() => import("./errors/404")),
  },
];
