import { lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";

/* Defining the routes for the application. */
export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("./pages/login")),
  },
  {
    path: "/",
    component: lazy(() => import("./layouts/withAds")),
    children: [
      {
        path: "/library",
        component: lazy(() => import("./layouts/library")),
        data: () => {
          console.log("Fetching all instances...");
        },
        children: [
          {
            path: "/",
            component: lazy(() => import("./pages/library")),
            data: () => {
              console.log("Fetching instances data...");
            },
          },
          {
            path: "/:id",
            component: lazy(() => import("./pages/library/instance")),
            data: () => {
              console.log("Fetching specific instance data...");
            },
          },
        ],
      },
      {
        path: "/modpacks",
        component: lazy(() => import("./layouts/modpacks")),
        children: [
          {
            path: "/",
            component: lazy(() => import("./pages/modpacks")),
            data: () => {
              console.log("Fetching modpacks data...");
            },
          },
        ],
      },
      {
        path: "/modpacks/:id",
        component: lazy(() => import("./pages/modpacks/modpack")),
        data: () => {
          console.log("Fetching specific modpack data...");
        },
      },
      {
        path: "/settings",
        component: lazy(() => import("./layouts/settings")),
        children: [
          {
            path: "/",
            component: lazy(() => import("./pages/settings")),
          },
        ],
      },
      {
        path: "**",
        component: lazy(() => import("./errors/404")),
      },
    ],
  },
];
