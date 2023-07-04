import { AdsBanner } from "@/components/AdBanner";
import AppNavbar from "@/components/Navbar";
import { Outlet, useLocation, useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import fetchData from "./app.data";
import { setMcVersions } from "@/utils/mcVersion";

function withAdsLayout() {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const location = useLocation();

  const modpackPathRegex = /\/modpacks\/(\d+)(\/.*)?/;
  const isModpackDetails = () => modpackPathRegex.test(location.pathname);

  createEffect(() => {
    if (routeData.minecraftVersions.data)
      setMcVersions(routeData.minecraftVersions.data);
  });

  return (
    <>
      <AppNavbar />
      <div class="flex w-screen z-10 h-auto">
        <main class="relative flex-1">
          <div
            class="grid justify-end h-[calc(100vh-60px)]"
            classList={{
              "grid-cols-[auto_2fr_440px]": !isModpackDetails(),
              "grid-cols-[2fr_440px]": isModpackDetails(),
            }}
          >
            <Outlet />
            <div class="flex justify-start flex-col gap-4 px-5 pt-5 bg-darkSlate-800 flex-initial w-100">
              <AdsBanner />
            </div>
            <div class="absolute top-0 left-0 right-0 bottom-0 bg-image-gdlauncher_pattern.svg -z-10" />
          </div>
        </main>
      </div>
    </>
  );
}

export default withAdsLayout;
