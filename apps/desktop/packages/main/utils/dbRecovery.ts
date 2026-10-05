/** Whitelisted guidance category; unrelated startup failures omit it. */
export type DbRecoveryCategory =
  | "locked"
  | "access"
  | "storage"
  | "corruption"
  | "version"
  | "downgrade"
  | "interrupted_restore"
  | "unknown"
export function dbRecoveryCategory(
  event: string,
  payload?: string
): DbRecoveryCategory {
  if (event === "DB_CORRUPT") return "corruption"
  if (event === "DB_DIVERGED") return "version"
  if (event === "DB_DOWNGRADE_FAILED") return "downgrade"
  // A migration history from a 2024 build that this build refuses to convert:
  // restarting or updating cannot change it, as with a diverged history.
  if (payload === "history") return "version"
  return payload === "locked" || payload === "access" || payload === "storage"
    ? payload
    : "unknown"
}
export function dbRecoveryAdvice(category?: DbRecoveryCategory): string {
  switch (category) {
    case "locked":
      return "Another operation may hold a database lock. Wait for it to finish, then Restart. Reset does not resolve a continuing lock."
    case "access":
      return "The database cannot be accessed. Check path availability, permissions and read-only storage, then Restart. Reset does not fix an access restriction."
    case "storage":
      return "SQLite reported a storage or I/O failure. Check free space and storage availability, then Restart. Reset does not repair the storage device."
    case "corruption":
      return "SQLite reported corrupt or non-database bytes. Preserve the recovery files and contact support; a last-resort reset creates a fresh database."
    case "version":
      return "Database history differs from this build. Use a compatible launcher version or contact support before resetting metadata."
    case "downgrade":
      return "The database downgrade failed. Use a compatible launcher version, or restore the supplied snapshot if available."
    case "interrupted_restore":
      return "A database restore did not finish, so GDLauncher will not open the database until its files are back together. Choose Recover Database below. It checks every file and changes nothing it cannot verify."
    case "unknown":
      return "Database initialization failed without a known cause. Restart, check for updates, or inspect the database folder with support before resetting metadata."
    default:
      return "Restart and check for updates first. Reset is a last resort for database metadata and may not resolve this failure."
  }
}
export const RESET_WARNING =
  "You'll need to sign in again. Active accounts, settings, preferences, Java configuration, library metadata and server registrations/settings return to defaults. Instance, world, addon and server directories on disk remain. A complete recovery copy of existing database files is retained for manual recovery; it is not automatically merged or guaranteed healthy."
