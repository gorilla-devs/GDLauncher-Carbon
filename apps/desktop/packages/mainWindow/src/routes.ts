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
            component: lazy(() => import("./pages/library/Instance")),
            data: () => {
              console.log("Fetching specific instance data...");
            },
            children: [
              {
                path: "/",
                component: lazy(
                  () => import("./pages/library/Instance/Overview")
                ),
              },
              {
                path: "/mods",
                component: lazy(() => import("./pages/library/Instance/Mods")),
                data: () => {
                  console.log("Fetching mods data...");
                },
              },
              {
                path: "/resourcepacks",
                component: lazy(
                  () => import("./pages/library/Instance/ResourcePacks")
                ),
              },
            ],
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
        component: lazy(() => import("./layouts/Settings")),
        children: [
          {
            path: "/",
            component: lazy(() => import("./pages/Settings/General")),
          },
          {
            path: "/appearance",
            component: lazy(() => import("./pages/Settings/Appearance")),
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
