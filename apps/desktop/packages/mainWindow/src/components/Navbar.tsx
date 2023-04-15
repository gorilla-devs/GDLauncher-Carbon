import { Link, useLocation, useMatch } from "@solidjs/router";
import { For, Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs, Spacing } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
import { useGDNavigate } from "@/managers/NavigationManager";

// import { createMatcher, expandOptionals } from "@solidjs/router";

// const isLocationMatch = (path: string) => {
//   const location = useLocation();
//   const matchers = expandOptionals(path).map((path) => createMatcher(path));

//   for (const matcher of matchers) {
//     const match = matcher(location.pathname);
//     if (match) return match;
//   }
// };

const AppNavbar = () => {
  const location = useLocation();
  const navigate = useGDNavigate();

  const isLogin = useMatch(() => "/");
  const isSettings = useMatch(() => "/settings");
  const isSettingsNested = useMatch(() => "/settings/*");

  const selectedIndex = () =>
    !!isSettings() || !!isSettingsNested()
      ? 4
      : getRouteIndex(NAVBAR_ROUTES, location.pathname);

  return (
    <Show when={!isLogin()}>
      <nav class="bg-dark-slate-800 flex items-center text-white px-5 h-15">
        <div class="flex w-full">
          <div class="flex items-center w-36">
            <img
              src={GDLauncherWideLogo}
              class="cursor-pointer h-9"
              onClick={() => navigate("/library")}
            />
          </div>
          <ul class="flex text-white w-full m-0 items-between gap-6 list-none pl-10">
            <Tabs index={selectedIndex()}>
              <TabList aligment="between">
                <div class="flex gap-6">
                  <For each={NAVBAR_ROUTES}>
                    {(route) => {
                      const isMatch = useMatch(() => route.path);

                      return (
                        <div
                          onClick={() =>
                            navigate(route.path, {
                              getLastInstance: true,
                            })
                          }
                          class="no-underline"
                          classList={{
                            "text-white": !!isMatch(),
                            "text-slate-400": !isMatch(),
                          }}
                        >
                          <Tab>
                            <li class="no-underline">{route.label}</li>
                          </Tab>
                        </div>
                      );
                    }}
                  </For>
                </div>
                <Spacing class="w-full" />
                <div class="flex gap-6 items-center">
                  <Tab ignored>
                    <div class="cursor-pointer text-dark-slate-50 text-2xl i-ri:terminal-box-fill" />
                  </Tab>
                  <Link href="/settings" class="no-underline">
                    <Tab>
                      <div
                        class="text-dark-slate-50 text-2xl cursor-pointer i-ri:settings-3-fill"
                        classList={{
                          "bg-primary-500":
                            !!isSettings() || !!isSettingsNested(),
                        }}
                      />
                    </Tab>
                  </Link>
                  <div class="text-dark-slate-50 text-2xl cursor-pointer i-ri:notification-2-fill" />
                </div>
              </TabList>
            </Tabs>
          </ul>
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
