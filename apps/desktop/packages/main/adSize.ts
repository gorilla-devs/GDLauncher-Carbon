import { screen } from "electron";

export default function getAdSize() {
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.size;

  console.log("width", width);
  console.log("height", height);

  if (width < 1920 || height < 1080) {
    // Smaller ads (160/600)
    return {
      minWidth: 1024,
      minHeight: 790,
      width: 1024,
      height: 790,
      adSize: {
        width: 160,
        height: 600
      }
    };
  } else {
    return {
      minWidth: 1000,
      minHeight: 790,
      width: 1600,
      height: 790,
      adSize: {
        width: 400,
        height: 600
      }
    };
  }
}
