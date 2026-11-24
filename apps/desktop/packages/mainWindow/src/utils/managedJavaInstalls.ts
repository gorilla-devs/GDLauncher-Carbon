import type { FEManagedJavaSetupArgs } from "@gd/core_module/bindings"
import { i18n } from "@gd/i18n"
import { toast } from "@gd/ui"
import { vendorLabel } from "../managers/ModalsManager/modals/Java/managedJavaCatalog"
import { extractErrorDisplay, rspc } from "./rspcClient"

// Runs managed Java installs outside the Add Java dialog, so closing the
// dialog mid-install still reports the result.

const claimedErrors = new Set<string>()

/** Called by the open Add Java dialog when it shows the failure of
 *  `buildId` inline, so the install doesn't also toast it. */
export function claimInstallError(buildId: string) {
  claimedErrors.add(buildId)
}

/** Installs the build, toasting "Azul Zulu 25.0.5.1 installed" as its row
 *  will name it, or what failed. The backend's reason for a failure is
 *  English and technical, so it goes to the log. */
export async function startManagedJavaInstall(
  args: FEManagedJavaSetupArgs,
  label: { version: string; major: number }
): Promise<string> {
  claimedErrors.delete(args.id)

  try {
    // The plain client, not `createMutation`: this runs outside any
    // component, and the global mutation error toast would duplicate ours.
    const javaId = await rspc
      .useContext()
      .client.mutation(["java.setupManagedJava", args])
    toast.success(
      i18n.t("java:_trn_java_installed", {
        vendor: vendorLabel(args.vendor),
        version: label.version
      })
    )
    return javaId
  } catch (error) {
    console.error(
      `Could not install Java ${label.version}:`,
      extractErrorDisplay(error)
    )
    // The dialog's own `catch` runs in a microtask after this rejection, so
    // check for its claim once those have run.
    setTimeout(() => {
      if (claimedErrors.delete(args.id)) return
      toast.error(i18n.t("java:_trn_download_failed", { major: label.major }))
    }, 0)
    throw error
  }
}
