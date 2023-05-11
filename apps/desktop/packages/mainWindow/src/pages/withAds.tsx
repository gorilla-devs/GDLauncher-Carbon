import { AdsBanner } from "@/components/AdBanner";
import AppNavbar from "@/components/Navbar";
import { Outlet, useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import fetchData from "./app.data";
import {
  setForgeVersions,
  setMappedMcVersions,
  setMcVersions,
} from "@/utils/mcVersion";

function withAdsLayout() {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (routeData.forgeVersions.data)
      setForgeVersions(routeData.forgeVersions.data);
  });

  createEffect(() => {
    if (routeData.minecraftVersions.data) {
      setMcVersions(routeData.minecraftVersions.data);

      routeData.minecraftVersions.data.forEach((version) => {
        if (version.type === "release") {
          setMappedMcVersions((prev) => [
            ...prev,
            { label: version.id, key: version.id },
          ]);
        }
      });
    }
  });

  return (
    <>
      <AppNavbar />
      <div class="flex w-screen z-10 h-auto">
        <main class="relative flex-1 overflow-hidden">
          <div class="flex justify-end h-[calc(100vh-60px)]">
            <Outlet />
            <div class="flex justify-start flex-col gap-4 px-5 pt-5 bg-darkSlate-800 flex-initial">
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
