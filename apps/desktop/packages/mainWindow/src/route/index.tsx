import { lazy } from "solid-js"
import Login from "@/pages/Login"
import withAdsLayout from "@/pages/withAds"
import Library from "@/pages/Library"
import Home from "@/pages/Library/Home"
import Instance from "@/pages/Library/Instance"
import AddonViewPage from "@/pages/AddonViewPage"
import Search from "@/pages/Search"
/* Defining the routes for the application. */

export const routes = [
  {
    path: "/",
    component: Login
  },
  {
    path: "/",
    component: withAdsLayout,
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
            path: "/server/:id",
            component: lazy(
              () => import("@/pages/Library/Server")
            )
          },
          {
            path: "/:id",
            component: Instance,
            children: [
              {
                path: "/",
                component: lazy(
                  () => import("@/pages/Library/Instance/Overview")
                )
              },
              {
                path: "/addons",
                component: lazy(
                  () => import("@/pages/Library/Instance/Tabs/Addons")
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
        path: "/news",
        component: lazy(() => import("@/pages/News/NewsWrapper")),
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/News"))
          },
          {
            path: "/:id",
            component: lazy(() => import("@/pages/News/PageView"))
          }
        ]
      },
      {
        path: "/search",
        component: Search,
        children: [
          {
            path: "/:type?",
            component: lazy(() => import("@/pages/Search/List"))
          }
        ]
      },
      {
        path: "/addon/:id/:platform",
        component: AddonViewPage,
        children: [
          {
            path: "/",
            component: lazy(() => import("@/pages/AddonViewPage/Overview"))
          },
          {
            path: "/versions",
            component: lazy(() => import("@/pages/AddonViewPage/Versions"))
          },
          {
            path: "/changelog",
            component: lazy(() => import("@/pages/AddonViewPage/Changelog"))
          },
          {
            path: "/screenshots",
            component: lazy(() => import("@/pages/AddonViewPage/Screenshots"))
          }
        ]
      },

      {
        path: "/settings",
        component: lazy(() => import("@/pages/Settings")),
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
            component: lazy(() => import("@/pages/Settings/Java"))
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
]
