import { AdsBanner } from "@/components/AdBanner";
import AppNavbar from "@/components/Navbar";
import { Outlet } from "@solidjs/router";

function withAdsLayout() {
  return (
    <>
      <AppNavbar />
      <div class="flex w-screen z-10 h-auto">
        <main class="relative flex-1 overflow-hidden">
          <div class="flex justify-end h-[calc(100vh-60px)]">
            <Outlet />
            <div class="flex justify-start flex-col gap-4 px-5 pt-5 bg-darkSlate-800 flex-initial">
              <AdsBanner />
              <div class="w-full h-16 bg-blue" />
            </div>
            <div class="absolute top-0 left-0 right-0 bottom-0 bg-image-gdlauncher_pattern.svg -z-10" />
          </div>
        </main>
      </div>
    </>
  );
}

export default withAdsLayout;
