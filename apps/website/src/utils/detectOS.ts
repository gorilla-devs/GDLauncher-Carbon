export type OS = "Windows" | "MacOS" | "Linux"

export function detectOS(): OS {
  if (typeof window === "undefined") return "Windows"

  const ua = window.navigator.userAgent.toLowerCase()
  if (ua.includes("windows")) return "Windows"
  if (ua.includes("mac")) return "MacOS"
  if (ua.includes("linux")) return "Linux"
  return "Windows"
}
