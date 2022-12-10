import { AdsBanner } from "@/components/AdBanner";
import { Pattern } from "@gd/ui";
import { Outlet } from "@solidjs/router";
import { Show } from "solid-js";

function withAdsLayout() {
  return (
    <div class="flex">
      <Outlet />
      {/* <Show when={location.pathname !== "/"}> */}
      <AdsBanner />
      <Pattern class="absolute top-0 left-0 right-0 bottom-0" />
      {/* </Show> */}
    </div>
  );
}

export default withAdsLayout;
