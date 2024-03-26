import { screen, Display } from "electron";

export default function getAdSize(display?: Display) {
  const primaryDisplay = display || screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.size;

  // only show fallback on macos
  const useFallbackAd =
    process.platform !== "darwin" && process.platform !== "win32";

  if (width < 1920 || height < 1080) {
    return {
      minWidth: width < 1024 ? width - 100 : 1024,
      minHeight: height < 790 ? height - 100 : 790,
      width: width < 1024 ? width - 100 : 1024,
      height: height < 790 ? height - 100 : 790,
      adSize: {
        useFallbackAd,
        useVertical: true,
        width: 160,
        height: 600
      }
    };
  } else {
    return {
      minWidth: 1280,
      minHeight: 790,
      width: 1600,
      height: 790,
      adSize: {
        useFallbackAd,
        useVertical: false,
        width: 400,
        height: 600
      }
    };
  }
}
