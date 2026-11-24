import type { FEJavaEntry } from "@gd/core_module/bindings"
import { toast } from "@gd/ui"
import { extractErrorDisplay, rspc } from "@/utils/rspcClient"
import { addFromFileDialogProperties } from "./javaOverviewView"

type AddJavaFromFileKey = "java:_trn_add_from_file" | "java:_trn_not_a_java"

/** Asks for a Java in a file dialog and adds it to the list as user-added.
 *  Resolves to the added Java, or `null` when the user cancels or the file
 *  isn't a Java we can run, which an error toast says. */
export async function addJavaFromFile(
  t: (key: AddJavaFromFileKey) => string
): Promise<FEJavaEntry | null> {
  try {
    const picked = await window.openFileDialog({
      title: t("java:_trn_add_from_file"),
      properties: addFromFileDialogProperties(navigator.platform)
    })
    const path = picked.filePaths[0]
    if (picked.canceled || !path) return null

    // The plain client: this error toast replaces the global one.
    return await rspc
      .useContext()
      .client.mutation(["java.addJavaFromFile", path])
  } catch (error) {
    // The backend's reason is English and technical; the log keeps it.
    console.error(
      "Could not add a Java from a file:",
      extractErrorDisplay(error)
    )
    toast.error(t("java:_trn_not_a_java"))
    return null
  }
}
