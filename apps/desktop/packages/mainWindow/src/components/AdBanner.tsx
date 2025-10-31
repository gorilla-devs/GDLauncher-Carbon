import adSize from "@/utils/adhelper"

export const AdsBanner = () => {
  const isHighImpact = () => adSize.width === 440

  return (
    <div
      style={{
        height: `${adSize.height}px`,
        width: `${adSize.width}px`,
        "z-index": "50000",
        position: "relative"
      }}
    >
      <owadview
        class="relative"
        {...(isHighImpact() ? { adstyle: "high-impact-ad" } : {})}
      />
    </div>
  )
}
