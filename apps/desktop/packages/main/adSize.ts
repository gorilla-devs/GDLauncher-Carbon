import { screen, Display } from "electron"
import os from "os"

// Ad Configuration
const USE_HIGH_IMPACT_AD = true

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

// Gap and text height configurations for different tiers
const GAP_LARGE = 16
const GAP_SMALL = 10
const TEXT_HEIGHT_FULL = 100

const getTitlebarHeight = () => {
  const platform = os.platform() as keyof typeof TITLEBAR_HEIGHT
  return TITLEBAR_HEIGHT[platform] || 30
}

/**
 * Get the effective display size accounting for:
 * - Work area (excludes taskbar/dock)
 * - DPI scaling (on Windows, workArea is in physical pixels)
 */
function getEffectiveDisplaySize(display: Display) {
  const workArea = display.workArea
  const scaleFactor = display.scaleFactor || 1

  // Windows returns workArea in physical pixels, divide by scaleFactor
  // macOS/Linux return logical pixels already
  const isWindows = os.platform() === "win32"

  return {
    width: isWindows
      ? Math.floor(workArea.width / scaleFactor)
      : workArea.width,
    height: isWindows
      ? Math.floor(workArea.height / scaleFactor)
      : workArea.height,
    scaleFactor
  }
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
      },
      hideAdText: true
    }
  }

  const primaryDisplay = display || screen.getPrimaryDisplay()
  const { width, height } = getEffectiveDisplaySize(primaryDisplay)

  // Tier 1: Large displays (≥1600×900 effective)
  // Uses HIGH_IMPACT (440px) + BANNER, minWidth 1260px for ≤35% coverage (34.9%)
  if (width >= 1600 && height >= 900) {
    const mainAd = USE_HIGH_IMPACT_AD ? AD_SIZES.HIGH_IMPACT : AD_SIZES.STANDARD
    const minHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      mainAd.height +
      GAP_LARGE +
      TEXT_HEIGHT_FULL

    return {
      minWidth: 1260,
      minHeight,
      width: Math.min(width - 60, 1600),
      height: Math.min(height - 40, minHeight + 50),
      adSize: {
        width: mainAd.width,
        height: mainAd.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: AD_SIZES.BANNER.width,
        height: AD_SIZES.BANNER.height,
        shouldShow: true
      },
      hideAdText: false
    }
  }

  // Tier 2: Medium-Large displays (≥1400×850 effective)
  // Uses STANDARD (400px), minWidth 1145px for ≤35% coverage (34.9%)
  if (width >= 1400 && height >= 850) {
    const minHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      AD_SIZES.STANDARD.height +
      GAP_LARGE +
      TEXT_HEIGHT_FULL

    return {
      minWidth: 1145,
      minHeight,
      width: Math.min(width - 60, 1450),
      height: Math.min(height - 40, 870),
      adSize: {
        width: AD_SIZES.STANDARD.width,
        height: AD_SIZES.STANDARD.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: 0,
        height: 0,
        shouldShow: false
      },
      hideAdText: false
    }
  }

  // Tier 3: Medium displays (≥1024×700 effective)
  // Designed for 1366×768 laptops with taskbar
  // Uses SKYSCRAPER (160px) to preserve content area on laptops
  if (width >= 1024 && height >= 700) {
    const minHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      AD_SIZES.SKYSCRAPER.height +
      GAP_SMALL

    return {
      minWidth: 900,
      minHeight,
      width: Math.min(width - 60, 1100),
      height: Math.min(height - 40, Math.max(minHeight, 750)),
      adSize: {
        width: AD_SIZES.SKYSCRAPER.width,
        height: AD_SIZES.SKYSCRAPER.height,
        shouldShow: true
      },
      bannerAdSize: {
        width: 0,
        height: 0,
        shouldShow: false
      },
      hideAdText: true
    }
  }

  // Tier 4: Small displays (<1024 width or <700 height effective)
  // Uses SKYSCRAPER (160px), minWidth 460px for ≤35% coverage (34.8%)
  // But we set minWidth 800px for usability
  // Always hides ad text to minimize height requirements
  const minHeight =
    getTitlebarHeight() + NAVBAR_HEIGHT + AD_SIZES.SKYSCRAPER.height + GAP_SMALL

  return {
    minWidth: 800,
    minHeight,
    width: Math.min(width - 40, 1000),
    height: Math.min(height - 30, Math.max(minHeight, 720)),
    adSize: {
      width: AD_SIZES.SKYSCRAPER.width,
      height: AD_SIZES.SKYSCRAPER.height,
      shouldShow: true
    },
    bannerAdSize: {
      width: 0,
      height: 0,
      shouldShow: false
    },
    hideAdText: true
  }
}
