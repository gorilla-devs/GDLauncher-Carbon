import { Link, useLocation, useMatch, useNavigate } from "@solidjs/router";
import { For, Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";
import { Tab, TabList, Tabs } from "@gd/ui";
import getRouteIndex from "@/route/getRouteIndex";
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
  const navigate = useNavigate();
  const [t] = useTransContext();

  const isLogin = useMatch(() => "/");
  const isSettings = useMatch(() => "/settings");
  const isSettingsNested = useMatch(() => "/settings/*");

  const selectedIndex = () => getRouteIndex(NAVBAR_ROUTES, location.pathname);

  return (
    <Show when={!isLogin()}>
      <nav class="bg-shade-8 text-white h-15 flex items-center justify-between px-5">
        <div class="flex">
          <img
            src={GDLauncherWideLogo}
            class="h-9 cursor-pointer"
            onClick={() => navigate("/library")}
          />
          <ul class="flex items-between gap-6 m-0 text-white list-none pl-10">
            <Tabs index={selectedIndex()}>
              <TabList>
                <For each={NAVBAR_ROUTES}>
                  {(route) => {
                    const isMatch = useMatch(() => route.path);

                    return (
                      <Link
                        href={route.path}
                        class="no-underline"
                        classList={{
                          "text-white": !!isMatch(),
                          "text-slate-400": !isMatch(),
                        }}
                      >
                        <Tab>
                          <li class="no-underline">{route.label}</li>
                        </Tab>
                      </Link>
                    );
                  }}
                </For>
              </TabList>
            </Tabs>
          </ul>
        </div>
        <div class="flex gap-5 items-center">
          <div class="flex gap-5">
            <div class="i-ri:terminal-box-fill text-shade-0 text-2xl cursor-pointer" />
            <div
              class="i-ri:settings-3-fill text-shade-0 text-2xl cursor-pointer"
              classList={{
                "bg-primary": !!isSettings() || !!isSettingsNested(),
              }}
              onClick={() => navigate("/settings")}
            />
            <div class="i-ri:notification-2-fill text-shade-0 text-2xl cursor-pointer" />
          </div>
          <div class="w-40 h-10 bg-shade-7" />
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
