import { AdsBanner } from "@/components/AdBanner";
import { Outlet } from "@solidjs/router";

function withAdsLayout() {
  return (
    <div class="flex h-full justify-end">
      <Outlet />
      <div class="flex justify-end flex-initial">
        <AdsBanner />
      </div>
      <div class="bg-image-gdlauncher_pattern.svg -z-10 absolute top-0 left-0 right-0 bottom-0" />
    </div>
  );
}

export default withAdsLayout;
