import type { FEGDLAccountStatus } from "@gd/core_module/bindings"

/**
 * What the launcher knows about the GDLauncher account when a feature that
 * needs a verified one is opened:
 * - `ready`: an account with a verified email.
 * - `unverified`: an account whose email is not verified yet.
 * - `noAccount`: the core answered that there is none.
 * - `unknown`: the core has not answered (still loading, or the read failed).
 *   Nothing may be claimed about the account then.
 */
export type GdlAccountAccess = "ready" | "unverified" | "noAccount" | "unknown"

export function gdlAccountAccess(
  account: FEGDLAccountStatus | null | undefined
): GdlAccountAccess {
  if (!account) return "unknown"
  if (account.status !== "valid") return "noAccount"
  return account.value.isEmailVerified ? "ready" : "unverified"
}
