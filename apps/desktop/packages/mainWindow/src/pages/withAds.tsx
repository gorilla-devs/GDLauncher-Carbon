import { AdsBanner } from "@/components/AdBanner";
import AppNavbar from "@/components/Navbar";
import { Outlet, useLocation, useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import fetchData from "./app.data";
import { setMappedMcVersions, setMcVersions } from "@/utils/mcVersion";
import {
  setCurseForgeModloaders,
  setCurseforgeCategories,
  setModrinthCategories,
  setSupportedModloaders
} from "@/utils/sidebar";
import { supportedCfModloaders } from "@/utils/constants";
import adSize from "@/utils/adhelper";

function withAdsLayout() {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const location = useLocation();

  const modpackPathRegex = /(modpacks|mods)\/(\w+)(\/.*)?/;
  const isDetailPage = () => modpackPathRegex.test(location.pathname);

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
      setCurseForgeModloaders(routeData.curseForgeModloaders.data);

      const curseforgeModpackModloaders = () => {
        const filtered = routeData.curseForgeModloaders.data?.filter(
          (modloader) => supportedCfModloaders.includes(modloader as string)
        );
        return filtered || [];
      };

      setSupportedModloaders(curseforgeModpackModloaders());
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
          <div
            class="grid justify-end h-[calc(100vh-60px)]"
            classList={{
              "xs:grid-cols-[auto_2fr_200px] sm:grid-cols-[auto_2fr_200px] md:grid-cols-[auto_2fr_200px] xl:grid-cols-[auto_2fr_440px]":
                !isDetailPage(),
              "xs:grid-cols-[2fr_200px] sm:grid-cols-[2fr_200px] md:grid-cols-[2fr_200px] lg:grid-cols-[2fr_440px]":
                isDetailPage()
            }}
          >
            <Outlet />
            <div
              id="ads-layout-container"
              class="flex flex-col gap-4 px-5 pt-5 bg-darkSlate-800 justify-start flex-initial"
              style={{
                width: `${adSize.width + 40}px`
              }}
            >
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
