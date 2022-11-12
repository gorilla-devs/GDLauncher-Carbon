import minimumBounds from "@/modules/components/minimumBounds";

export const AdsBanner = () => {
  const isBannerSmall = () => minimumBounds.adSize.width === 160;

  return (
    <div
      style={{
        height: `${minimumBounds.adSize.height}px`,
        width: `${minimumBounds.adSize.width}px`,
        "margin-top": isBannerSmall() ? "-40px" : "0",
      }}
      class="bg-red-400 mx-5 mt-5"
    />
  );
};
