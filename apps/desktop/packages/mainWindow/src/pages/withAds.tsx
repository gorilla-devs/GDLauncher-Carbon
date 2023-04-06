import { AdsBanner } from "@/components/AdBanner";
import { Outlet } from "@solidjs/router";

function withAdsLayout() {
  return (
    <div class="flex justify-end h-[calc(100vh-60px-28px)]">
      <Outlet />
      <div class="flex justify-start flex-col gap-4 px-5 pt-5 bg-shade-8 flex-initial">
        <AdsBanner />
        <div class="w-full h-16 bg-blue" />
      </div>
      <div class="absolute top-0 left-0 right-0 bottom-0 bg-image-gdlauncher_pattern.svg -z-10" />
    </div>
  );
}

export default withAdsLayout;
