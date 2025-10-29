import adSize from "@/utils/adhelper"

export const AdsBanner = () => {
  return (
    <div
      style={{
        height: `${adSize.height}px`,
        width: `${adSize.width}px`,
        "z-index": "50000",
        position: "relative"
      }}
    >
      <owadview class="relative" />
    </div>
  )
}
