import adSize from "@/utils/adhelper";

export const AdsBanner = () => {
  return (
    <div
      style={{
        height: `${adSize.height}px`,
        width: `${adSize.width}px`
      }}
    >
      <owadview class="relative z-50" />
    </div>
  );
};
