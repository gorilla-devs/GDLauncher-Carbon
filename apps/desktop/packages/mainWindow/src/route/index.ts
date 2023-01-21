import { lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import SettingsJavaData from "@/pages/Settings/settings.java.data";

/* Defining the routes for the application. */
export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("@/pages/Login")),
  },
  {
    path: "/",
    component: lazy(() => import("@/pages/withAds")),
    children: [
      {
        path: "/library",
        component: lazy(() => import("@/pages/Library")),
        data: () => {
          console.log("Fetching all instances...");
        },
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Library/Home")),
            data: () => {
              console.log("Fetching instances data...");
            },
          },
          {
            path: "/:id",
            component: lazy(() => import("@/pages/Library/Instance")),
            data: () => {
              console.log("Fetching specific instance data...");
            },
            children: [
              {
                path: "/",
                component: lazy(
                  () => import("@/pages/Library/Instance/Overview")
                ),
              },
              {
                path: "/mods",
                component: lazy(() => import("@/pages/Library/Instance/Mods")),
                data: () => {
                  console.log("Fetching mods data...");
                },
              },
              {
                path: "/resourcepacks",
                component: lazy(
                  () => import("@/pages/Library/Instance/ResourcePacks")
                ),
              },
            ],
          },
        ],
      },
      {
        path: "/modpacks",
        component: lazy(() => import("@/pages/Modpacks")),
        data: async () => {
          await new Promise((resolve) => setTimeout(resolve, 10000));
          console.log("Fetching whatever data...");
          return {};
        },
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Modpacks/Browser")),
            data: () => {
              console.log("Fetching modpacks data...");
            },
          },
        ],
      },
      {
        path: "/modpacks/:id",
        component: lazy(() => import("@/pages/Modpacks/Explore")),
        data: () => {
          console.log("Fetching specific modpack data...");
        },
      },
      {
        path: "/settings",
        component: lazy(() => import("@/pages/Settings")),
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Settings/General")),
          },
          {
            path: "/appearance",
            component: lazy(() => import("@/pages/Settings/Appearance")),
          },
          {
            path: "/java",
            component: lazy(() => import("@/pages/Settings/Java")),
            data: SettingsJavaData,
          },
        ],
      },
      {
        path: "**",
        component: lazy(() => import("@/errors/404")),
      },
    ],
  },
];
