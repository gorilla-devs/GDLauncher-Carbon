import { screen, Display } from "electron"
import os from "os"

// Ad Configuration
const USE_HIGH_IMPACT_AD = false

const AD_SIZES = {
  STANDARD: { width: 400, height: 600 },
  HIGH_IMPACT: { width: 440, height: 730 },
  SKYSCRAPER: { width: 160, height: 600 },
  BANNER: { width: 400, height: 60 }
}

// Window layout constants
const TITLEBAR_HEIGHT = {
  darwin: 28, // macOS
  win32: 32, // Windows
  linux: 30 // Linux (TODO: varies by DE. Best approach: ??)
}
const NAVBAR_HEIGHT = 60
const GAP = 16
const TEXT_HEIGHT = 100

const getTitlebarHeight = () => {
  const platform = os.platform() as keyof typeof TITLEBAR_HEIGHT
  return TITLEBAR_HEIGHT[platform] || 30
}

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
      },
      bannerAdSize: {
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
    const mainAd = USE_HIGH_IMPACT_AD ? AD_SIZES.HIGH_IMPACT : AD_SIZES.STANDARD
    const minHeight =
      getTitlebarHeight() + NAVBAR_HEIGHT + mainAd.height + GAP + TEXT_HEIGHT

    return {
      minWidth: 1200,
      minHeight,
      width: 1400,
      height: minHeight,
      adSize: {
        width: mainAd.width,
        height: mainAd.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: AD_SIZES.BANNER.width,
        height: AD_SIZES.BANNER.height,
        shouldShow: true
      }
    }
  }

  // Tier 2: Medium-Large displays (≥1680×1050) - Comfortable (Standard Ad only)
  if (width >= 1680 && height >= 1050) {
    const minHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      AD_SIZES.STANDARD.height +
      GAP +
      TEXT_HEIGHT

    return {
      minWidth: 1200,
      minHeight,
      width: 1350,
      height: 850,
      adSize: {
        width: AD_SIZES.STANDARD.width,
        height: AD_SIZES.STANDARD.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: 0,
        height: 0,
        shouldShow: false
      }
    }
  }

  // Tier 3: Medium displays (≥1366×768) - Efficient (Standard Ad only)
  if (width >= 1366 && height >= 768) {
    const minHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      AD_SIZES.STANDARD.height +
      GAP +
      TEXT_HEIGHT

    return {
      minWidth: 1200,
      minHeight,
      width: Math.min(width - 80, 1280),
      height: Math.max(minHeight, Math.min(height - 48, 820)),
      adSize: {
        width: AD_SIZES.STANDARD.width,
        height: AD_SIZES.STANDARD.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: 0,
        height: 0,
        shouldShow: false
      }
    }
  }

  // Tier 4: Small displays (<1366×768) - Adaptive (Skyscraper only)
  const minHeight =
    getTitlebarHeight() +
    NAVBAR_HEIGHT +
    AD_SIZES.SKYSCRAPER.height +
    GAP +
    TEXT_HEIGHT

  return {
    minWidth: 960,
    minHeight,
    width: Math.min(width - 48, 1150),
    height: Math.min(height - 48, 820),
    adSize: {
      width: AD_SIZES.SKYSCRAPER.width,
      height: AD_SIZES.SKYSCRAPER.height,
      shouldShow: true
    },
    bannerAdSize: {
      width: 0,
      height: 0,
      shouldShow: false
    }
  }
}
