import { lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import SettingsJavaData from "@/pages/Settings/settings.java.data";
import HomeData from "@/pages/home.data";
import SettingsGeneralData from "@/pages/Settings/settings.general.data";
import LoginData from "@/pages/Login/auth.login.data";
import AppData from "@/pages/app.data";
import BrowserData from "@/pages/Modpacks/browser.data";
import ModpackData from "@/pages/Modpacks/modpack.overview";
import ModpackVersionsData from "@/pages/Modpacks/modpack.versions";
/* Defining the routes for the application. */

export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("@/pages/Login")),
    data: LoginData,
  },
  {
    path: "/",
    component: lazy(() => import("@/pages/withAds")),
    data: AppData,
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
            data: HomeData,
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
                path: "/settings",
                component: lazy(
                  () => import("@/pages/Library/Instance/Settings")
                ),
                data: () => {
                  console.log("Fetching instance settings data...");
                },
              },
              {
                path: "/resourcepacks",
                component: lazy(
                  () => import("@/pages/Library/Instance/ResourcePacks")
                ),
              },
              {
                path: "/screenshots",
                component: lazy(
                  () => import("@/pages/Library/Instance/Screenshots")
                ),
              },
              {
                path: "/versions",
                component: lazy(
                  () => import("@/pages/Library/Instance/Versions")
                ),
              },
            ],
          },
        ],
      },
      {
        path: "/modpacks",
        component: lazy(() => import("@/pages/Modpacks")),
        data: BrowserData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Modpacks/Browser")),
          },
        ],
      },
      {
        path: "/modpacks/:id",
        component: lazy(() => import("@/pages/Modpacks/Explore")),
        data: ModpackData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Modpacks/Explore/Overview")),
          },
          {
            path: "/versions",
            component: lazy(() => import("@/pages/Modpacks/Explore/Versions")),
            data: ModpackVersionsData,
          },
        ],
      },
      {
        path: "/settings",
        component: lazy(() => import("@/pages/Settings")),
        data: SettingsGeneralData,
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
