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
import Login from "@/pages/Login";
import withAdsLayout from "@/pages/withAds";
import Library from "@/pages/Library";
import Home from "@/pages/Library/Home";
import Instance from "@/pages/Library/Instance";
import ModpacksLayout from "@/pages/Modpacks";
import ModpackBrowser from "@/pages/Modpacks/ModpacksBrowser";
import ModsBrowser from "@/pages/Mods/ModsBrowser";
import ModsInfiniteScrollQueryWrapper from "@/pages/Mods/Explore";
import ModpacksInfiniteScrollQueryWrapper from "@/pages/Modpacks/Explore";
import ModsLayout from "@/pages/Mods";
/* Defining the routes for the application. */

export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: Login,
    data: LoginData
  },
  {
    path: "/",
    component: withAdsLayout,
    data: AppData,
    children: [
      {
        path: "/library",
        component: Library,
        children: [
          {
            path: "/",
            component: Home
          },
          {
            path: "/:id",
            component: Instance,
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
                  () =>
                    import("@/pages/Library/Instance/Tabs/Log/LogsRouteWrapper")
                ),
                children: [
                  {
                    path: "/",
                    component: lazy(
                      () => import("@/pages/Library/Instance/Tabs/Log")
                    )
                  }
                ]
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
        component: ModpacksLayout,
        data: ModpackBrowserData,
        children: [
          {
            path: "/",
            component: ModpackBrowser
          }
        ]
      },
      {
        path: "/mods",
        component: ModsLayout,
        data: ModsBrowserData,
        children: [
          {
            path: "/",
            component: ModsBrowser
          }
        ]
      },
      {
        path: "/mods/:id/:platform",
        component: ModsInfiniteScrollQueryWrapper,
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
        component: ModpacksInfiniteScrollQueryWrapper,
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
            path: "/accounts",
            component: lazy(() => import("@/pages/Settings/Accounts"))
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
            path: "/custom-commands",
            component: lazy(() => import("@/pages/Settings/CustomCommands"))
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
