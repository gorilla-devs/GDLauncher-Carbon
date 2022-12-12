import { Link, useLocation, useNavigate } from "@solidjs/router";
import { For, Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { NAVBAR_ROUTES } from "@/constants";

const AppNavbar = () => {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <Show when={location.pathname !== "/"}>
      <nav class="bg-black-black text-white h-15 flex items-center justify-between px-5">
        <div class="flex">
          <img src={GDLauncherWideLogo} class="h-9" />
          <ul class="flex items-between gap-6 m-0 text-white list-none pl-10">
            <For each={NAVBAR_ROUTES}>
              {(route) => {
                const isActiveRoute = () =>
                  location.pathname.includes(route.path);
                return (
                  <li class="py-2 no-underline">
                    <Link
                      href={route.path}
                      class="no-underline"
                      classList={{
                        "text-white": isActiveRoute(),
                        "text-slate-400": !isActiveRoute(),
                      }}
                    >
                      {route.label}
                    </Link>
                  </li>
                );
              }}
            </For>
          </ul>
        </div>
        <div class="flex gap-5 items-center">
          <div class="flex gap-5">
            <div class="i-ri:terminal-box-fill text-black-lightGray text-2xl cursor-pointer" />
            <div
              class="i-ri:settings-3-fill text-black-lightGray text-2xl cursor-pointer"
              classList={{
                "bg-accent-main": location.pathname === "/settings",
              }}
              onClick={() => navigate("/settings")}
            />
            <div class="i-ri:notification-2-fill text-black-lightGray text-2xl cursor-pointer" />
          </div>
          <div class="w-40 h-10 bg-black-semiblack" />
        </div>
      </nav>
    </Show>
  );
};

export default AppNavbar;
