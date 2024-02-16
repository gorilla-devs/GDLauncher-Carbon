import { AdsBanner } from "@/components/AdBanner";
import AppNavbar from "@/components/Navbar";
import { Outlet, useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import fetchData from "./app.data";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import {
  setCurseforgeCategories,
  setModrinthCategories,
  setSupportedModloaders
} from "@/utils/sidebar";
import adSize from "@/utils/adhelper";
import { Trans } from "@gd/i18n";
import { useModal } from "@/managers/ModalsManager";

function withAdsLayout() {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const modalContext = useModal();

  createEffect(() => {
    if (routeData.minecraftVersions.data) {
      setMcVersions(routeData.minecraftVersions.data);
      routeData.minecraftVersions.data.forEach((version) => {
        if (version.type === "release") {
          setMappedMcVersions((prev) => [
            ...prev,
            { label: version.id, key: version.id }
          ]);
        }
      });
      setMappedMcVersions((prev) => [
        { key: "", label: "All version" },
        ...prev
      ]);
    }
  });

  createEffect(() => {
    if (routeData.curseForgeModloaders.data) {
      setSupportedModloaders("curseforge", routeData.curseForgeModloaders.data);
    }
    if (routeData.modrinthModloaders.data) {
      setSupportedModloaders("modrinth", routeData.modrinthModloaders.data);
    }
  });

  createEffect(() => {
    if (routeData.curseforgeCategories.data)
      setCurseforgeCategories(routeData.curseforgeCategories.data.data);
  });

  createEffect(() => {
    if (routeData.modrinthCategories.data)
      setModrinthCategories(routeData.modrinthCategories.data);
  });

  return (
    <>
      <AppNavbar />
      <div class="flex w-screen z-10 h-auto">
        <main class="relative flex-grow">
          <div class="flex justify-end h-[calc(100vh-60px)]">
            <div
              style={{
                width: `calc(100vw - ${adSize.width}px)`
              }}
            >
              <Outlet />
            </div>
            <div class="flex flex-col justify-between h-[calc(100vh-100px)]">
              <div
                class="bg-darkSlate-800 py-4"
                style={{
                  width: `${adSize.width}px`,
                  height: `${adSize.height}px`
                }}
              >
                <AdsBanner />
              </div>
              <div class="flex justify-center">
                <div
                  class="text-center hover:text-darkSlate-50 transition-colors duration-200 text-darkSlate-200"
                  onClick={() => {
                    modalContext?.openModal({
                      name: "whyAreAdsNeeded"
                    });
                  }}
                >
                  <Trans key="why_are_ads_needed" />
                </div>
              </div>
            </div>
            <div class="absolute top-0 left-0 right-0 bottom-0 bg-image-gdlauncher_pattern.svg -z-10" />
          </div>
        </main>
      </div>
    </>
  );
}

export default withAdsLayout;
