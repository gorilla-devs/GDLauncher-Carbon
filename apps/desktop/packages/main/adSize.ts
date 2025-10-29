import { screen, Display } from "electron"

export default function getAdSize(display?: Display) {
  // Showcase mode - 4:3 aspect ratio with no ads
  if (__SHOWCASE_MODE__) {
    return {
      minWidth: 1280,
      minHeight: 960,
      width: 1280,
      height: 960,
      adSize: {
        width: 0,
        height: 0,
        shouldShow: false
      }
    }
  }

  const primaryDisplay = display || screen.getPrimaryDisplay()
  const { width, height } = primaryDisplay.size

  // Tier 1: Large displays (≥1920×1080) - Spacious
  if (width >= 1920 && height >= 1080) {
    return {
      minWidth: 1200,
      minHeight: 720,
      width: 1600,
      height: 900,
      adSize: {
        width: 440,
        height: 670,
        shouldShow: true
      }
    }
  }

  // Tier 2: Medium-Large displays (≥1680×1050) - Comfortable
  if (width >= 1680 && height >= 1050) {
    return {
      minWidth: 1200,
      minHeight: 720,
      width: 1450,
      height: 850,
      adSize: {
        width: 400,
        height: 600,
        shouldShow: true
      }
    }
  }

  // Tier 3: Medium displays (≥1366×768) - Efficient
  if (width >= 1366 && height >= 768) {
    return {
      minWidth: 1200,
      minHeight: 720,
      width: Math.min(width - 80, 1280),
      height: 720,
      adSize: {
        width: 400,
        height: 600,
        shouldShow: true
      }
    }
  }

  // Tier 4: Small displays (<1366×768) - Adaptive
  return {
    minWidth: 960,
    minHeight: 680,
    width: Math.min(width - 48, 1150),
    height: Math.min(height - 48, 720),
    adSize: {
      width: 160,
      height: 600,
      shouldShow: true
    }
  }
}
