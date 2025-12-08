/**
 * Channel-based logo selection
 *
 * Returns the appropriate logo paths based on the current release channel
 * determined from __APP_VERSION__
 */

// Import all logo variants (Vite processes these at build time)
import wideLogoStable from "/assets/images/gdlauncher_wide_logo_blue.svg"
import wideLogoBeta from "/assets/images/gdlauncher_wide_logo_blue_beta.svg"
import wideLogoAlpha from "/assets/images/gdlauncher_wide_logo_blue_alpha.svg"

import logoStable from "/assets/images/gdlauncher_logo.svg"
import logoBeta from "/assets/images/gdlauncher_logo_beta.svg"
import logoAlpha from "/assets/images/gdlauncher_logo_alpha.svg"

// Determine channel from version string
const isBeta = __APP_VERSION__.includes("-beta")
const isAlpha = __APP_VERSION__.includes("-alpha")

/**
 * Wide horizontal logo used in login sidebar, navbar, settings
 */
export const wideLogoUrl = isAlpha
  ? wideLogoAlpha
  : isBeta
    ? wideLogoBeta
    : wideLogoStable

/**
 * Small square logo used in favicon, account dropdown
 */
export const logoUrl = isAlpha ? logoAlpha : isBeta ? logoBeta : logoStable

// Update favicon dynamically for beta/alpha channels
if (isBeta || isAlpha) {
  const favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
  if (favicon) {
    favicon.href = logoUrl
  }
}
