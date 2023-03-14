import { lazy } from "solid-js";
import { RouteDefinition } from "@solidjs/router";
import SettingsJavaData from "@/pages/Settings/settings.java.data";
import NewsData from "@/pages/news.data";
import SettingsAppearanceData from "@/pages/Settings/settings.appearance.data";
import LoginData from "@/pages/Login/auth.login.data";
import AppData from "@/pages/app.data";
import { PrivateRoute } from "./protectedRoute";

/* Defining the routes for the application. */

export const routes: RouteDefinition[] = [
  {
    path: "/",
    component: lazy(() => import("@/pages/Login")),
    data: LoginData,
  },
  {
    path: "/",
    component: () => (
      <PrivateRoute component={lazy(() => import("@/pages/withAds"))} />
    ),
    data: AppData,
    children: [
      {
        path: "/library",
        component: () => (
          <PrivateRoute component={lazy(() => import("@/pages/Library"))} />
        ),
        data: () => {
          console.log("Fetching all instances...");
        },
        children: [
          {
            path: "/",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Library/Home"))}
              />
            ),
            data: NewsData,
          },
          {
            path: "/:id",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Library/Instance"))}
              />
            ),
            data: () => {
              console.log("Fetching specific instance data...");
            },
            children: [
              {
                path: "/",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/Overview")
                    )}
                  />
                ),
              },
              {
                path: "/mods",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/Mods")
                    )}
                  />
                ),
                data: () => {
                  console.log("Fetching mods data...");
                },
              },
              {
                path: "/settings",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/Settings")
                    )}
                  />
                ),
                data: () => {
                  console.log("Fetching instance settings data...");
                },
              },
              {
                path: "/resourcepacks",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/ResourcePacks")
                    )}
                  />
                ),
              },
              {
                path: "/screenshots",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/Screenshots")
                    )}
                  />
                ),
              },
              {
                path: "/versions",
                component: () => (
                  <PrivateRoute
                    component={lazy(
                      () => import("@/pages/Library/Instance/Versions")
                    )}
                  />
                ),
              },
            ],
          },
        ],
      },
      {
        path: "/modpacks",
        component: () => (
          <PrivateRoute component={lazy(() => import("@/pages/Modpacks"))} />
        ),
        data: async () => {
          await new Promise((resolve) => setTimeout(resolve, 10000));
          console.log("Fetching whatever data...");
          return {};
        },
        children: [
          {
            path: "/",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Modpacks/Browser"))}
              />
            ),
            data: () => {
              console.log("Fetching modpacks data...");
            },
          },
        ],
      },
      {
        path: "/modpacks/:id",
        component: () => (
          <PrivateRoute
            component={lazy(() => import("@/pages/Modpacks/Explore"))}
          />
        ),
        data: () => {
          console.log("Fetching specific modpack data...");
        },
      },
      {
        path: "/settings",
        component: () => (
          <PrivateRoute component={lazy(() => import("@/pages/Settings"))} />
        ),
        children: [
          {
            path: "/",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Settings/General"))}
              />
            ),
          },
          {
            path: "/appearance",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Settings/Appearance"))}
              />
            ),
            data: SettingsAppearanceData,
          },
          {
            path: "/java",
            component: () => (
              <PrivateRoute
                component={lazy(() => import("@/pages/Settings/Java"))}
              />
            ),
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
