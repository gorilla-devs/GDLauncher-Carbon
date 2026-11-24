import { screen, Display } from "electron"
import os from "os"
import { NAVBAR_HEIGHT } from "./adView/geometry"

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

// Gap and text height configurations for different tiers
const GAP_LARGE = 16
const GAP_SMALL = 10
const TEXT_HEIGHT_FULL = 100
// The ad column stacks its ads with this gap between them (`gap-2`).
const AD_STACK_GAP = 8

// An ad is never shown cut off, and the window never has to be taller than
// the screen's work area to show it. So a tier is used only on a display its
// minimum height fits. A display too short for an ad under the navbar gets
// the ad column beside the navbar instead (`besideNavbar`), which leaves the
// ad the window's whole height.

const getTitlebarHeight = () => {
  const platform = os.platform() as keyof typeof TITLEBAR_HEIGHT
  return TITLEBAR_HEIGHT[platform] || 30
}

/** Electron workArea uses DIP coordinates, matching BrowserWindow dimensions. */
function getEffectiveDisplaySize(display: Display) {
  return { width: display.workArea.width, height: display.workArea.height }
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
        shouldShow: false,
        besideNavbar: false
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
    const fullMinHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      mainAd.height +
      GAP_LARGE +
      TEXT_HEIGHT_FULL
    // Both ads whole, without the "Why are ads needed?" line under them:
    // what 2560×1440 at 150% and 4K at 225% (about 912 high) have room for.
    const adsMinHeight =
      getTitlebarHeight() +
      NAVBAR_HEIGHT +
      AD_SIZES.BANNER.height +
      AD_STACK_GAP +
      mainAd.height +
      GAP_LARGE
    const withText = height >= fullMinHeight
    const minHeight = withText ? fullMinHeight : adsMinHeight

    // A display between the two falls through to tier 2, whose ad fits.
    if (height >= minHeight)
      return {
        minWidth: 1260,
        minHeight,
        width: Math.min(width - 60, 1600),
        height: Math.max(minHeight, Math.min(height - 40, minHeight + 50)),
        adSize: {
          width: mainAd.width,
          height: mainAd.height,
          shouldShow: true,
          besideNavbar: false
        },
        bannerAdSize: {
          width: AD_SIZES.BANNER.width,
          height: AD_SIZES.BANNER.height,
          shouldShow: true
        },
        hideAdText: !withText
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
        shouldShow: true,
        besideNavbar: false
      },
      bannerAdSize: {
        width: 0,
        height: 0,
        shouldShow: false
      },
      hideAdText: false
    }
  }

  // The SKYSCRAPER (160px) tiers: under the navbar when the display has the
  // height for it, beside the navbar when it only has the height for the ad.
  const skyscraperMinHeight =
    getTitlebarHeight() + NAVBAR_HEIGHT + AD_SIZES.SKYSCRAPER.height + GAP_SMALL
  const besideNavbarMinHeight =
    getTitlebarHeight() + AD_SIZES.SKYSCRAPER.height + GAP_SMALL
  const skyscraper = {
    width: AD_SIZES.SKYSCRAPER.width,
    height: AD_SIZES.SKYSCRAPER.height,
    shouldShow: true
  }
  const noBanner = { width: 0, height: 0, shouldShow: false }

  // Tier 3: Medium displays (≥1024×700 effective)
  // Designed for 1366×768 laptops with taskbar
  // Uses SKYSCRAPER (160px) to preserve content area on laptops
  if (width >= 1024 && height >= 700 && height >= skyscraperMinHeight) {
    return {
      minWidth: 900,
      minHeight: skyscraperMinHeight,
      width: Math.min(width - 60, 1100),
      height: Math.max(skyscraperMinHeight, Math.min(height - 40, 750)),
      adSize: { ...skyscraper, besideNavbar: false },
      bannerAdSize: noBanner,
      hideAdText: true
    }
  }

  // Short displays: 1920×1080 at 150% (about 1280×672) and 1280×720. The ad
  // does not fit under the navbar, so its column runs beside it. The navbar
  // loses the column's width, so the window is kept wider where the display
  // allows it.
  if (height < skyscraperMinHeight && height >= besideNavbarMinHeight) {
    return {
      minWidth: width >= 1024 ? 960 : 800,
      minHeight: besideNavbarMinHeight,
      width: Math.min(width - 40, 1100),
      height: Math.max(besideNavbarMinHeight, Math.min(height, 720)),
      adSize: { ...skyscraper, besideNavbar: true },
      bannerAdSize: noBanner,
      hideAdText: true
    }
  }

  // Tier 4: Small displays (<1024 width effective), and displays too short
  // for the ad in either place, which keep the size they always had.
  // Uses SKYSCRAPER (160px), minWidth 460px for ≤35% coverage (34.8%)
  // But we set minWidth 800px for usability
  // Always hides ad text to minimize height requirements
  return {
    minWidth: 800,
    minHeight: skyscraperMinHeight,
    width: Math.min(width - 40, 1000),
    height: Math.max(skyscraperMinHeight, Math.min(height - 30, 720)),
    adSize: { ...skyscraper, besideNavbar: false },
    bannerAdSize: noBanner,
    hideAdText: true
  }
}
