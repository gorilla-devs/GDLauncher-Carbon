import { screen } from "electron";

export default function getAdSize() {
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.workAreaSize;

  if (width <= 800 || height <= 600) {
    // Smaller ads (160/600)
    return {
      minWidth: 760,
      minHeight: 500,
      width: 760,
      height: 500,
      adSize: {
        width: 160,
        height: 600
      }
    };
  } else if (width < 1000 || height < 800) {
    // Smaller ads (160/600)
    return {
      minWidth: 800,
      minHeight: 600,
      width: 800,
      height: 600,
      adSize: {
        width: 160,
        height: 600
      }
    };
  } else if (width < 1500 || height < 870) {
    // Smaller ads (160/600)
    return {
      minWidth: 1160,
      minHeight: 720,
      width: 1160,
      height: 720,
      adSize: {
        width: 160,
        height: 600
      }
    };
  } else {
    return {
      minWidth: 1300,
      minHeight: 740,
      width: 1560,
      height: 740,
      adSize: {
        // width: 400,
        // height: 300
        width: 160,
        height: 600
      }
    };
  }
}
