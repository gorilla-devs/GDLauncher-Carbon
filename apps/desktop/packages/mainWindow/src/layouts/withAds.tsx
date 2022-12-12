import { AdsBanner } from "@/components/AdBanner";
import { Pattern } from "@gd/ui";
import { Outlet } from "@solidjs/router";

function withAdsLayout() {
  return (
    <div class="flex h-full justify-end">
      <Outlet />
      {/* <Show when={location.pathname !== "/"}> */}
      <div class="flex justify-end flex-initial">
        <AdsBanner />
      </div>
      <Pattern class="absolute top-0 left-0 right-0 bottom-0" />
      {/* </Show> */}
    </div>
  );
}

export default withAdsLayout;
