import fs from "node:fs/promises"
import { constants } from "node:fs"
import path from "node:path"

/** Errors that mean this user may not change the AppImage's folder. */
const REFUSED = new Set(["EACCES", "EPERM", "EROFS"])

/**
 * electron-updater installs an AppImage update by deleting the running
 * AppImage and moving the new one into its folder. In a folder this user
 * cannot write to (/opt, a read-only mount) that fails only after GDLauncher
 * has stopped its core for the install, so it is checked first. Any other
 * error leaves the decision to the install itself.
 */
export async function appImageInstallBlocker(
  appImage: string | undefined
): Promise<"install-folder-read-only" | null> {
  if (!appImage) return null
  try {
    await fs.access(path.dirname(appImage), constants.W_OK | constants.X_OK)
    return null
  } catch (error) {
    return REFUSED.has((error as NodeJS.ErrnoException).code ?? "")
      ? "install-folder-read-only"
      : null
  }
}
