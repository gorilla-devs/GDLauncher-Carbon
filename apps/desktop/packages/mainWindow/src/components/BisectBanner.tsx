import adSize from "@/utils/adhelper";
import BisectBannerImage from "/assets/images/bisect_banner.webp";
import BisectBannerVerticalImage from "/assets/images/bisect_banner_vertical.webp";

export const BisectBanner = () => {
  return (
    <div
      style={{
        width: `${adSize.width}px`
      }}
      class="absolute top-4 flex justify-center h-min max-w-full px-4 box-border z-100"
    >
      <img
        src={adSize.useVertical ? BisectBannerVerticalImage : BisectBannerImage}
        class="h-full w-full"
        classList={{
          "w-full": !adSize.useVertical,
          "w-auto": adSize.useVertical
        }}
        style={{
          "max-height": "calc(100vh - 160px)"
        }}
        data-cursor-pointer
        onClick={() => {
          window.open("https://www.bisecthosting.com/gdl");
        }}
      />
    </div>
  );
};
