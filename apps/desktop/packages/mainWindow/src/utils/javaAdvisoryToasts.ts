import type { FEJavaAdvisoryMatch } from "@gd/core_module/bindings"

/** One advisory's toast: the Javas it affects that the user should hear
 *  about. */
export interface AdvisoryToast {
  advisoryId: string
  name: string
  javaIds: string[]
}

/** The key `dismissJavaAdvisories` stores for one advisory and Java. */
const dismissalKey = (advisoryId: string, javaId: string) =>
  `${advisoryId}|${javaId}`

/** What "Don't warn again" stores for a toast: each of its Javas. */
export const dismissalKeys = (toast: AdvisoryToast) =>
  toast.javaIds.map((javaId) => dismissalKey(toast.advisoryId, javaId))

/** Settings › Java, landing on the Javas the toast counted. */
export function reviewPath(toast: AdvisoryToast): string {
  const params = new URLSearchParams({ review: toast.advisoryId })
  for (const javaId of toast.javaIds) params.append("java", javaId)
  return `/settings/java?${params.toString()}`
}

/** The toasts to show, one per advisory in order of first appearance. A
 *  match counts only when an instance uses the Java, auto-update won't fix
 *  it, and the user hasn't dismissed it; advisories in `alreadyShown` were
 *  toasted earlier this session. */
export function advisoryToastsToShow(
  matches: FEJavaAdvisoryMatch[],
  dismissed: string[],
  alreadyShown: Set<string>
): AdvisoryToast[] {
  const silenced = new Set(dismissed)
  const toasts = new Map<string, AdvisoryToast>()

  for (const match of matches) {
    if (match.autoFixable || !match.inUse) continue
    if (alreadyShown.has(match.advisoryId)) continue
    if (silenced.has(dismissalKey(match.advisoryId, match.javaId))) continue

    const toast = toasts.get(match.advisoryId)
    if (!toast) {
      toasts.set(match.advisoryId, {
        advisoryId: match.advisoryId,
        name: match.name,
        javaIds: [match.javaId]
      })
    } else if (!toast.javaIds.includes(match.javaId)) {
      toast.javaIds.push(match.javaId)
    }
  }

  return [...toasts.values()]
}

/** What the toasts on screen need after the matches change. */
export interface AdvisoryToastChanges {
  show: AdvisoryToast[]
  /** Open toasts whose Javas changed, to redraw in place. */
  update: AdvisoryToast[]
  /** Open toasts with no Java left to warn about. */
  close: string[]
}

const sameJavas = (a: AdvisoryToast, b: AdvisoryToast) =>
  a.javaIds.length === b.javaIds.length &&
  a.javaIds.every((id, i) => id === b.javaIds[i])

/** The session's advisory toasts. Each advisory is toasted once. While its
 *  toast is open it follows the matches, so a count taken halfway through
 *  the startup instance scan catches up, and "Don't warn again" covers every
 *  Java the toast shows by then. */
export function createAdvisoryToasts() {
  const shown = new Set<string>()
  const open = new Map<string, AdvisoryToast>()

  return {
    sync(
      matches: FEJavaAdvisoryMatch[],
      dismissed: string[]
    ): AdvisoryToastChanges {
      const changes: AdvisoryToastChanges = { show: [], update: [], close: [] }
      const current = advisoryToastsToShow(matches, dismissed, new Set())

      for (const toast of current) {
        const onScreen = open.get(toast.advisoryId)
        if (onScreen) {
          if (sameJavas(onScreen, toast)) continue
          changes.update.push(toast)
        } else if (!shown.has(toast.advisoryId)) {
          shown.add(toast.advisoryId)
          changes.show.push(toast)
        } else {
          continue
        }
        open.set(toast.advisoryId, toast)
      }

      const live = new Set(current.map((toast) => toast.advisoryId))
      for (const advisoryId of open.keys()) {
        if (live.has(advisoryId)) continue
        open.delete(advisoryId)
        changes.close.push(advisoryId)
      }

      return changes
    },
    /** The open toast as it is now, for its "Review" or "Don't warn again". It
     *  closes with that click. */
    take(advisoryId: string): AdvisoryToast | undefined {
      const toast = open.get(advisoryId)
      open.delete(advisoryId)
      return toast
    },
    /** Closed with its X or a swipe. */
    closed(advisoryId: string) {
      open.delete(advisoryId)
    }
  }
}
