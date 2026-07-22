import crypto from "crypto"

/**
 * Normalize email address per UID2 specification
 * https://unifiedid.com/docs/getting-started/gs-normalization-encoding#email-address-normalization
 *
 * Steps:
 * 1. Trim leading/trailing whitespace
 * 2. Convert to lowercase
 * 3. Apply Gmail-specific rules:
 *    - Remove periods (.) from local part
 *    - Remove + and everything after it before @gmail.com
 *
 * @param email - Raw email address
 * @returns Normalized email address
 */
export function normalizeEmail(email: string): string {
  // 1. Trim whitespace
  let normalized = email.trim()

  // 2. Lowercase
  normalized = normalized.toLowerCase()

  // 3. Gmail-specific rules
  if (normalized.endsWith("@gmail.com")) {
    const [localPart, domain] = normalized.split("@")

    // Remove periods from local part
    let gmailLocal = localPart.replace(/\./g, "")

    // Remove + and everything after it
    const plusIndex = gmailLocal.indexOf("+")
    if (plusIndex !== -1) {
      gmailLocal = gmailLocal.substring(0, plusIndex)
    }

    normalized = `${gmailLocal}@${domain}`
  }

  return normalized
}

/**
 * Hash formats required by Overwolf's setUserEmailHashes API
 */
export interface EmailHashes {
  sha1: string
  sha256: string
  md5: string
}

/**
 * Generate all hash formats required by Overwolf's setUserEmailHashes API
 *
 * Per Overwolf documentation, hashes should be:
 * - Normalized email (lowercase, trimmed, Gmail rules applied)
 * - Hex-encoded (lowercase)
 *
 * The key names are read verbatim by `setUserEmailHashes` and are case
 * sensitive, so they must match `overwolf.EmailHashes` exactly.
 *
 * @param email - Raw email address
 * @returns Object with sha1, sha256, and md5 hex hashes
 */
export function hashEmailForOverwolf(email: string): EmailHashes {
  const normalized = normalizeEmail(email)

  return {
    sha1: crypto.createHash("sha1").update(normalized).digest("hex"),
    sha256: crypto.createHash("sha256").update(normalized).digest("hex"),
    md5: crypto.createHash("md5").update(normalized).digest("hex")
  }
}
