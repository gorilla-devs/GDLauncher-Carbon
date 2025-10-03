/**
 * Utility functions for avatar generation and styling
 */

// Predefined color palette for consistent avatar backgrounds
const AVATAR_COLORS = [
  "#EF4444", // red-500
  "#F97316", // orange-500
  "#EAB308", // yellow-500
  "#22C55E", // green-500
  "#06B6D4", // cyan-500
  "#3B82F6", // blue-500
  "#8B5CF6", // violet-500
  "#EC4899", // pink-500
  "#F59E0B", // amber-500
  "#10B981", // emerald-500
  "#6366F1", // indigo-500
  "#F43F5E" // rose-500
]

/**
 * Generate a consistent hash from a string
 */
function stringToHash(str: string): number {
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i)
    hash = (hash << 5) - hash + char
    hash = hash & hash // Convert to 32-bit integer
  }
  return Math.abs(hash)
}

/**
 * Get a deterministic background color for an author
 */
export function getAuthorColor(identifier: string): string {
  const hash = stringToHash(identifier)
  return AVATAR_COLORS[hash % AVATAR_COLORS.length]
}

/**
 * Extract initials from a name
 */
export function getInitials(name: string): string {
  if (!name || name.trim().length === 0) return "?"

  const words = name.trim().split(/\s+/)

  if (words.length === 1) {
    // Single word - take first two characters
    return words[0].substring(0, 2).toUpperCase()
  }

  // Multiple words - take first letter of first two words
  return words
    .slice(0, 2)
    .map((word) => word.charAt(0))
    .join("")
    .toUpperCase()
}

/**
 * Calculate if text should be white or black based on background color
 */
export function getContrastTextColor(backgroundColor: string): string {
  // Remove # if present
  const hex = backgroundColor.replace("#", "")

  // Convert to RGB
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)

  // Calculate relative luminance
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255

  // Return white for dark backgrounds, black for light backgrounds
  return luminance > 0.5 ? "#000000" : "#FFFFFF"
}
