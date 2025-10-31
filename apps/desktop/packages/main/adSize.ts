import { screen, Display } from "electron"

export default function getAdSize(display?: Display) {
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

  // Tier 1: Large displays (≥1920×1080)
  if (width >= 1920 && height >= 1080) {
    return {
      minWidth: 1200,
      minHeight: 876, // navbar(60) + ad(730) + gap(16) + text(70)
      width: 1600,
      height: 960,
      adSize: {
        width: 440,
        height: 730,
        shouldShow: true
      }
    }
  }

  // Tier 2: Medium-Large displays (≥1680×1050) - Comfortable (Standard Ad only)
  if (width >= 1680 && height >= 1050) {
    return {
      minWidth: 1200,
      minHeight: 746, // navbar(60) + ad(600) + gap(16) + text(70)
      width: 1450,
      height: 850,
      adSize: {
        width: 400,
        height: 600,
        shouldShow: true
      }
    }
  }

  // Tier 3: Medium displays (≥1366×768) - Efficient (Standard Ad only)
  if (width >= 1366 && height >= 768) {
    return {
      minWidth: 1200,
      minHeight: 746, // navbar(60) + ad(600) + gap(16) + text(70)
      width: Math.min(width - 80, 1280),
      height: Math.max(746, Math.min(height - 48, 820)),
      adSize: {
        width: 400,
        height: 600,
        shouldShow: true
      }
    }
  }

  // Tier 4: Small displays (<1366×768) - Adaptive (Skyscraper only)
  return {
    minWidth: 960,
    minHeight: 746, // navbar(60) + ad(600) + gap(16) + text(70)
    width: Math.min(width - 48, 1150),
    height: Math.min(height - 48, 820),
    adSize: {
      width: 160,
      height: 600,
      shouldShow: true
    }
  }
}
