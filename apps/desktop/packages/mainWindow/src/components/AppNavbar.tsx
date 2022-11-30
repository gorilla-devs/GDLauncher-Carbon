import { Link, useLocation } from "@solidjs/router";
import { Show } from "solid-js";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";

export default function AppNavbar() {
  const location = useLocation();

  return (
    <Show when={location.pathname !== "/"}>
      <nav class="bg-[#1D2028] text-white h-15 flex items-center px-5">
        <img src={GDLauncherWideLogo} class="h-9" />
        <ul class="flex items-between gap-6 m-0 text-white list-none pl-23">
          <li class="py-2">
            <Link href="/home" class="no-underline hover:underline text-white">
              Home
            </Link>
          </li>
          <li class="py-2">
            <Link href="/about" class="no-underline hover:underline text-white">
              About
            </Link>
          </li>
          <li class="py-2">
            <Link
              href="/error"
              class="no-underline decoratione hover:underline text-white"
            >
              Error
            </Link>
          </li>
          <li class="py-2">
            <Link
              href="/"
              class="no-underline decoratione hover:underline text-white"
            >
              Logout
            </Link>
          </li>

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
}
