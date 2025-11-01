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
      <owadview class="relative" />
    </div>
  )
}
