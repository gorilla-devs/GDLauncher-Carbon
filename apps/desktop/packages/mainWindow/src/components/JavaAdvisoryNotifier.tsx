import { useTransContext } from "@gd/i18n"
import { toast } from "@gd/ui"
import { createEffect, untrack } from "solid-js"
import { useLocation } from "@solidjs/router"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import {
  createAdvisoryToasts,
  dismissalKeys,
  reviewPath,
  type AdvisoryToast
} from "@/utils/javaAdvisoryToasts"

/** This session's toasts. Each advisory shows once until the next launch,
 *  whether it was reviewed, closed or dismissed. */
const toasts = createAdvisoryToasts()

const toastId = (advisoryId: string) => `java-advisory-${advisoryId}`

/** Toasts the known issues of the Javas the user's instances run, once per
 *  session. Renders nothing. */
const JavaAdvisoryNotifier = () => {
  const [t] = useTransContext()
  const navigate = useGDNavigate()
  const location = useLocation()

  // A failed check shows no warning rather than an error about one.
  const matches = rspc.createQuery(() => ({
    queryKey: ["java.getJavaAdvisoryMatches"],
    meta: { silentError: true }
  }))
  const dismissed = rspc.createQuery(() => ({
    queryKey: ["java.getDismissedJavaAdvisories"],
    meta: { silentError: true }
  }))
  const dismiss = rspc.createMutation(() => ({
    mutationKey: ["java.dismissJavaAdvisories"]
  }))

  /** Shows the toast, or redraws the open one with the same id. */
  const show = (advisory: AdvisoryToast, update: boolean) => {
    const id = advisory.advisoryId
    // The buttons act on the toast as it is when clicked.
    const latest = () => toasts.take(id) ?? advisory

    toast.warning(
      t("java:_trn_security_flaw_in_javas", {
        name: advisory.name,
        count: advisory.javaIds.length,
        interpolation: { escapeValue: false }
      }),
      {
        id: toastId(id),
        icon: <div class="i-hugeicons:alert-diamond h-4 w-4 text-yellow-500" />,
        duration: Infinity,
        // "Don't warn again" is long in most languages: the buttons get
        // their own line.
        className: update
          ? "toast-actions-below toast-no-animate"
          : "toast-actions-below",
        action: {
          label: t("java:_trn_review"),
          onClick: () => navigate.navigate(reviewPath(latest()))
        },
        // Unlike the toast's X, which only hides it until the next launch.
        cancel: {
          label: t("java:_trn_dont_warn_again"),
          onClick: () => dismiss.mutate(dismissalKeys(latest()))
        },
        onDismiss: () => toasts.closed(id)
      }
    )
  }

  createEffect(() => {
    // Not over the login screen: it waits until the user is in.
    if (location.pathname === "/") return
    if (!matches.data || !dismissed.data) return
    const changes = toasts.sync(matches.data, dismissed.data)

    untrack(() => {
      for (const advisory of changes.show) show(advisory, false)
      for (const advisory of changes.update) show(advisory, true)
      for (const advisoryId of changes.close) toast.dismiss(toastId(advisoryId))
    })
  })

  return null
}

export default JavaAdvisoryNotifier
