import { Link, useLocation, useNavigate } from "@solidjs/router";
import { For, Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import GDLauncherLogo from "/assets/images/gdlauncher_logo.svg";
import { routes } from "@/routes";

type Props = {
  sidebarCollapsed: boolean;
};

const AppNavbar = (props: Props) => {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <Show when={location.pathname !== "/"}>
      <nav class="bg-black-black text-white h-15 flex items-center justify-between px-5">
        <div class="flex">
          <img
            src={props.sidebarCollapsed ? GDLauncherLogo : GDLauncherWideLogo}
            class="h-9"
          />
          <ul
            class="flex items-between gap-6 m-0 text-white list-none"
            classList={{
              "pl-10": props.sidebarCollapsed,
              "pl-20": !props.sidebarCollapsed,
            }}
          >
            <For each={routes.filter((route) => route.visibileInNavbar)}>
              {(route) => (
                <li class="py-2 no-underline">
                  <Link
                    href={route.path}
                    class="no-underline"
                    classList={{
                      "text-white": location.pathname === route.path,
                      "text-slate-400": location.pathname !== route.path,
                    }}
                  >
                    {route.label}
                  </Link>
                </li>
              )}
            </For>
          </ul>
        </div>
        <div class="flex gap-5">
          <div class="flex gap-5">
            <div class="i-ri:terminal-box-fill text-black-lightGray text-2xl cursor-pointer" />
            <div
              class="i-ri:settings-3-fill text-black-lightGray text-2xl cursor-pointer"
              classList={{
                "bg-accent-main": location.pathname === "/settings"
              }}
              onClick={() => navigate("/settings")}
            />
            <div class="i-ri:notification-2-fill text-black-lightGray text-2xl cursor-pointer" />
          </div>
          <div class="w-40 h-10 bg-black-semiblack"></div>
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
