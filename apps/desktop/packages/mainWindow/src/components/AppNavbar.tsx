import { Link, useLocation } from "@solidjs/router";
import { For, Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import GDLauncherLogo from "/assets/images/gdlauncher_logo.svg";
import { routes } from "@/utils/constants";

type Props = {
  sidebarCollapsed: boolean;
};

const AppNavbar = (props: Props) => {
  const location = useLocation();

  return (
    <Show when={location.pathname !== "/"}>
      <nav class="bg-black-black text-white h-15 flex items-center px-5">
        <img
          src={props.sidebarCollapsed ? GDLauncherLogo : GDLauncherWideLogo}
          class="h-9"
        />
        <ul
          class="flex items-between gap-6 m-0 text-white list-none"
          classList={{
            "pl-12": props.sidebarCollapsed,
            "pl-23": !props.sidebarCollapsed,
          }}
        >
          <For each={routes}>
            {(route) => (
              <li class="py-2 no-underline">
                <Link
                  href={route.href}
                  class="no-underline"
                  classList={{
                    "text-white": location.pathname === route.href,
                    "text-slate-400": location.pathname !== route.href,
                  }}
                >
                  {route.label}
                </Link>
              </li>
            )}
          </For>

          {/* <li class="text-sm flex items-center space-x-1 ml-auto">
            <span>URL:</span>
            <input
              class="w-75px p-1 bg-white text-sm rounded-lg"
              type="text"
              readOnly
              value={location.pathname + location.search}
            />
          </li> */}
        </ul>
      </nav>
    </Show>
  );
};

export default AppNavbar;
