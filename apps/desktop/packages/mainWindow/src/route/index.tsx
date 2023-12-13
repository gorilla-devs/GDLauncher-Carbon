import { lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import SettingsJavaData from "@/pages/Settings/settings.java.data";
import SettingsGeneralData from "@/pages/Settings/settings.general.data";
import LoginData from "@/pages/Login/auth.login.data";
import AppData from "@/pages/app.data";
import ModpackBrowserData from "@/pages/Modpacks/modpacksBrowser.data";
import ModsBrowserData from "@/pages/Mods/modsBrowser.data";
import ModpackData from "@/pages/Modpacks/modpack.overview";
import ModpackVersionsData from "@/pages/Modpacks/modpack.versions";
import ModVersionsData from "@/pages/Mods/mods.versions";
import ModpackScreenshotsData from "@/pages/Modpacks/modpack.screenshots";
import InstanceData from "@/pages/Library/Instance/instance.data";
import InstanceLogsData from "@/pages/Library/Instance/instance.logs.data";
import LibraryData from "@/pages/Library/library.data";
/* Defining the routes for the application. */

export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("@/pages/Login")),
    data: LoginData
  },
  {
    path: "/",
    component: lazy(() => import("@/pages/withAds")),
    data: AppData,
    children: [
      {
        path: "/library",
        component: lazy(() => import("@/pages/Library")),
        data: LibraryData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Library/Home"))
          },
          {
            path: "/:id",
            component: lazy(() => import("@/pages/Library/Instance")),
            data: InstanceData,
            children: [
              {
                path: "/",
                component: lazy(
                  () => import("@/pages/Library/Instance/Overview")
                )
              },
              {
                path: "/mods",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Mods")
                )
              },
              {
                path: "/settings",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Settings")
                )
              },
              {
                path: "/logs",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Log")
                ),
                data: InstanceLogsData
              },
              {
                path: "/resourcepacks",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/ResourcePacks")
                )
              },
              {
                path: "/screenshots",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Screenshots")
                )
              },
              {
                path: "/versions",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Versions")
                )
              }
            ]
          }
        ]
      },
      {
        path: "/modpacks",
        component: lazy(() => import("@/pages/Modpacks")),
        data: ModpackBrowserData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Modpacks/ModpacksBrowser"))
          }
        ]
      },
      {
        path: "/mods",
        component: lazy(() => import("@/pages/Mods")),
        data: ModsBrowserData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Mods/ModsBrowser"))
          }
        ]
      },
      {
        path: "/mods/:id/:platform",
        component: lazy(() => import("@/pages/Mods/Explore")),
        data: ModpackData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Mods/Explore/Overview"))
          },
          {
            path: "/versions",
            component: lazy(() => import("@/pages/Mods/Explore/Versions")),
            data: ModVersionsData
          },
          {
            path: "/changelog",
            component: lazy(() => import("@/pages/Mods/Explore/Changelog"))
          },
          {
            path: "/screenshots",
            component: lazy(() => import("@/pages/Mods/Explore/Screenshots")),
            data: ModpackScreenshotsData
          }
        ]
      },
      {
        path: "/modpacks/:id/:platform",
        component: lazy(() => import("@/pages/Modpacks/Explore")),
        data: ModpackData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Modpacks/Explore/Overview"))
          },
          {
            path: "/versions",
            component: lazy(() => import("@/pages/Modpacks/Explore/Versions")),
            data: ModpackVersionsData
          },
          {
            path: "/changelog",
            component: lazy(() => import("@/pages/Modpacks/Explore/Changelog"))
          },
          {
            path: "/screenshots",
            component: lazy(
              () => import("@/pages/Modpacks/Explore/Screenshots")
            ),
            data: ModpackScreenshotsData
          }
        ]
      },
      {
        path: "/settings",
        component: lazy(() => import("@/pages/Settings")),
        data: SettingsGeneralData,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/Settings/General"))
          },
          {
            path: "/language",
            component: lazy(() => import("@/pages/Settings/Language"))
          },
          {
            path: "/appearance",
            component: lazy(() => import("@/pages/Settings/Appearance"))
          },
          {
            path: "/java",
            component: lazy(() => import("@/pages/Settings/Java")),
            data: SettingsJavaData
          },
          {
            path: "/privacy",
            component: lazy(() => import("@/pages/Settings/Privacy"))
          },
          {
            path: "/runtime-path",
            component: lazy(() => import("@/pages/Settings/RuntimePath"))
          }
        ]
      },
      {
        path: "**",
        component: lazy(() => import("@/errors/404"))
      }
    ]
  }
];
