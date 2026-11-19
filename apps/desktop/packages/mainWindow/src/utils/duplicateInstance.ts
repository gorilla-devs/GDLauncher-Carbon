import { createSignal } from "solid-js"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { errorToastCopyAction } from "@/utils/errorToastCopyAction"
import { extractErrorDisplay } from "@/utils/rspcErrorText"

// The instances a copy is being made of, for every menu that offers
// Duplicate: the library tile and the favourites bar show the same instance.
const [copying, setCopying] = createSignal<ReadonlySet<number>>(new Set())
// The notice each instance's last copy left, when it failed. The next copy
// puts it away and tells itself in a notice of its own: one reused would
// keep the failure's text under the spinner.
const failed = new Map<number, string>()
let copies = 0

/** Whether a copy of `instance` is being made, so Duplicate is not offered
 * again for it: a second click would start a second full copy. */
export const isDuplicating = (instance: number) => copying().has(instance)

/**
 * Duplicate with something to see while it runs. Copying a large instance
 * takes minutes, and its tile only appears at the end, so a notice shows for
 * as long as the copy is made. It goes when the copy's tile is there, which
 * says the rest, and turns into the reason when the copy failed.
 */
export function useDuplicateInstance() {
  const [t] = useTransContext()
  const mutation = rspc.createMutation(() => ({
    mutationKey: ["instance.duplicateInstance"],
    meta: LOCAL_ERROR_TOAST
  }))

  return async (instance: number, name: string) => {
    if (isDuplicating(instance)) return
    const said = { name, interpolation: { escapeValue: false } }
    const last = failed.get(instance)
    if (last !== undefined) toast.dismiss(last)
    failed.delete(instance)
    const id = `duplicate-${instance}-${++copies}`
    setCopying((now) => new Set(now).add(instance))
    toast.loading(t("instances:_trn_duplicating", said), {
      id,
      duration: Infinity
    })
    try {
      await mutation.mutateAsync({ instance, new_name: name })
      toast.dismiss(id)
    } catch (error) {
      const description = extractErrorDisplay(error)
      failed.set(instance, id)
      toast.error(t("instances:_trn_duplicate_failed", said), {
        id,
        description,
        action: errorToastCopyAction(description),
        duration: Infinity
      })
    } finally {
      setCopying((now) => {
        const next = new Set(now)
        next.delete(instance)
        return next
      })
    }
  }
}
