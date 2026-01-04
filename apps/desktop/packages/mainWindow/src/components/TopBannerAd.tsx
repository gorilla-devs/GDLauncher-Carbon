import { bannerAdSize } from "@/utils/adhelper"

export const TopBannerAd = () => {
  return (
    <div
      style={{
        height: `${bannerAdSize.height}px`,
        width: `${bannerAdSize.width}px`,
        "z-index": "50000",
        position: "relative"
      }}
    >
      {/* @ts-ignore */}
      <owadview cid="gdlauncher_horizontal_400_60" class="relative" />
    </div>
  )
}
