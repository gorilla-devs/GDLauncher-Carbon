import { lazy } from "solid-js";
import AboutData from "./pages/about.data";
import Modpacks from "./components/Sidebar/contents/modpacks";
import Settings from "./components/Sidebar/contents/Settings";

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
    path: "/modpacks/:id",
    component: lazy(() => import("./pages/modpacks/modpack")),
    sidebar: Modpacks,
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
