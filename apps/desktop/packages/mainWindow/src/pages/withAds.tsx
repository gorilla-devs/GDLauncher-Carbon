import { AdsBanner } from "@/components/AdBanner";
import { Outlet } from "@solidjs/router";

function withAdsLayout() {
  return (
    <div class="flex h-[calc(100vh-60px-28px)] justify-end">
      <Outlet />
      <div class="flex justify-start flex-initial flex-col gap-4 mx-5 mt-5">
        <AdsBanner />
        <div class="bg-blue w-full h-16" />
      </div>
      <div class="bg-image-gdlauncher_pattern.svg -z-10 absolute top-0 left-0 right-0 bottom-0" />
    </div>
  );
}

export default withAdsLayout;
