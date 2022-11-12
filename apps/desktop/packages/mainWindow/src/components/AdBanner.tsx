import minimumBounds from "@/modules/components/minimumBounds";

export const AdsBanner = () => {
  const isBannerSmall = () => minimumBounds.adSize.width === 160;

  return (
    <div
      style={{
        height: `${minimumBounds.adSize.height}px`,
        width: `${minimumBounds.adSize.width}px`,
        // TODO: 60 is the hardcoded navbar height, we should put that in a varÍ
        "margin-top": isBannerSmall()
          ? `-${60 - minimumBounds.adSize.padding}px`
          : "0",
      }}
      class="bg-red-400 mx-5 mt-5"
    />
  );
};
