import { createSignal } from "solid-js"
import { i18n } from "@gd/i18n"
import { toast } from "@gd/ui"
import { extractErrorDisplay, rspc } from "./rspcClient"

// Manual updates of GDLauncher's Javas, kept outside Settings › Java so that
// leaving the page mid-update keeps the spinner and blocks a second update.

const [updating, setUpdating] = createSignal<ReadonlySet<string>>(new Set())

export function isUpdatingManagedJava(javaId: string): boolean {
  return updating().has(javaId)
}

/** Updates `javaId` to its newest build, `version`, and toasts the result.
 *  A Java already updating is left alone. */
export async function updateManagedJava(
  javaId: string,
  version: string
): Promise<void> {
  if (updating().has(javaId)) return
  setUpdating((ids) => new Set(ids).add(javaId))

  try {
    // The plain client: these toasts replace the global mutation error one.
    await rspc.useContext().client.mutation(["java.updateManagedJava", javaId])
    toast.success(i18n.t("java:_trn_java_updated", { version }))
  } catch (error) {
    toast.error(
      i18n.t("java:_trn_update_failed", {
        error: extractErrorDisplay(error),
        interpolation: { escapeValue: false }
      })
    )
  } finally {
    setUpdating((ids) => {
      const next = new Set(ids)
      next.delete(javaId)
      return next
    })
  }
}
