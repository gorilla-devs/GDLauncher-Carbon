import { isWithinInterval, parse, startOfDay, endOfDay } from "date-fns"

export interface OccasionAssets {
  preloadRive: string
  authVideo: string
  logoIcon: string
  logoWide: string
}

export interface Occasion {
  id: string
  name: string
  startDate: string // "MM-DD" format
  endDate: string // "MM-DD" format
  assets: OccasionAssets
  message: string
  colors: {
    primary: string
    accent: string
  }
  duration: number // Milliseconds to show seasonal splash before auto-redirect
}

// Default assets (current production assets)
const DEFAULT_ASSETS: OccasionAssets = {
  preloadRive: "gd_logo_animation.riv",
  authVideo: "assets/images/login_background.webm",
  logoIcon: "assets/images/gdlauncher_logo.svg",
  logoWide: "assets/images/gdlauncher_wide_logo_blue.svg"
}

// Occasion configurations
// For now, all use default assets but structure is ready for seasonal swaps
const OCCASIONS: Occasion[] = [
  {
    id: "new-years",
    name: "New Year's Celebration",
    startDate: "12-28",
    endDate: "01-05",
    assets: { ...DEFAULT_ASSETS },
    message: "Happy New Year!",
    colors: {
      primary: "#FFD700", // Gold
      accent: "#FF6B6B" // Red
    },
    duration: 3000
  },
  {
    id: "valentines",
    name: "Valentine's Day",
    startDate: "02-10",
    endDate: "02-14",
    assets: { ...DEFAULT_ASSETS },
    message: "Happy Valentine's Day!",
    colors: {
      primary: "#FF69B4", // Hot Pink
      accent: "#DC143C" // Crimson
    },
    duration: 3000
  },
  {
    id: "halloween",
    name: "Halloween",
    startDate: "10-20",
    endDate: "11-02",
    assets: {
      ...DEFAULT_ASSETS,
      authVideo: "assets/images/login_background_halloween.webm"
    },
    message: "Happy Halloween!",
    colors: {
      primary: "#FF6B35", // Orange
      accent: "#9B59B6" // Purple
    },
    duration: 4000
  },
  {
    id: "winter-holidays",
    name: "Winter Holidays",
    startDate: "12-10",
    endDate: "12-26",
    assets: { ...DEFAULT_ASSETS },
    message: "Happy Holidays!",
    colors: {
      primary: "#4A90E2", // Blue
      accent: "#E8F4F8" // Light blue
    },
    duration: 3000
  }
]

/**
 * Check if a given date falls within an occasion's date range
 * Handles year wraparound (e.g., Dec 28 - Jan 5)
 */
function _isDateInOccasion(
  occasion: Occasion,
  date: Date = new Date()
): boolean {
  const currentYear = date.getFullYear()

  // Parse dates with current year
  const start = parse(occasion.startDate, "MM-dd", new Date(currentYear, 0, 1))
  const end = parse(occasion.endDate, "MM-dd", new Date(currentYear, 0, 1))

  // Handle year wraparound (e.g., Dec 28 - Jan 5)
  if (start > end) {
    // Occasion spans two years
    // Check if we're in the end of previous year OR beginning of current year
    const prevYearStart = parse(
      occasion.startDate,
      "MM-dd",
      new Date(currentYear - 1, 0, 1)
    )
    const prevYearEnd = new Date(currentYear - 1, 11, 31, 23, 59, 59)

    const nextYearStart = new Date(currentYear, 0, 1)
    const nextYearEnd = parse(
      occasion.endDate,
      "MM-dd",
      new Date(currentYear, 0, 1)
    )

    return (
      isWithinInterval(date, {
        start: startOfDay(prevYearStart),
        end: endOfDay(prevYearEnd)
      }) ||
      isWithinInterval(date, {
        start: startOfDay(nextYearStart),
        end: endOfDay(nextYearEnd)
      })
    )
  }

  // Normal date range within same year
  return isWithinInterval(date, {
    start: startOfDay(start),
    end: endOfDay(end)
  })
}

/**
 * Get the currently active occasion, if any
 * Checks against current date and dev override
 */
export function getCurrentOccasion(): Occasion | null {
  // TEMPORARILY DISABLED FOR THIS RELEASE
  // All occasions are disabled to focus on core functionality
  return null

  // // Dev mode override for testing
  // if (typeof window !== "undefined" && (window as any).forceOccasion) {
  //   const forcedId = (window as any).forceOccasion
  //   const forced = OCCASIONS.find((o) => o.id === forcedId)
  //   if (forced) {
  //     console.log(`[Occasions] Forced occasion: ${forced.name}`)
  //     return forced
  //   }
  // }

  // const now = new Date()

  // // Find first matching occasion
  // for (const occasion of OCCASIONS) {
  //   if (isDateInOccasion(occasion, now)) {
  //     console.log(`[Occasions] Active occasion: ${occasion.name}`)
  //     return occasion
  //   }
  // }

  // return null
}

/**
 * Quick check if any special occasion is currently active
 */
export function isSpecialOccasion(): boolean {
  return getCurrentOccasion() !== null
}

/**
 * Get the assets for the current occasion, or default assets if no occasion
 */
export function getOccasionAssets(): OccasionAssets {
  const occasion = getCurrentOccasion()
  return occasion ? occasion.assets : DEFAULT_ASSETS
}

/**
 * Get all configured occasions (for debugging/testing)
 */
export function getAllOccasions(): Occasion[] {
  return [...OCCASIONS]
}
