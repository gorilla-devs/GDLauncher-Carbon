/**
 * The question asked before GDLauncher quits, restarts or installs an update
 * while that would stop a local server or cancel a download or install.
 *
 * A quit from the window's close button is held by main, which sends the
 * reasons with the close warning. A restart or an update install starts in
 * the page, so the page asks main first (`window.shutdownStops`), and the
 * modal manager shows the question when there is something to say.
 */
export type ShutdownQuestionAction = "quit" | "restart" | "update"

export interface ShutdownQuestion {
  action: ShutdownQuestionAction
  /** A game is running. It keeps running; only a quit mentions it. */
  game: boolean
  servers: boolean
  installs: boolean
  /** The answer to a restart or update question. A quit is answered by
   *  closing the window instead. */
  answer?: (confirmed: boolean) => void
}

const SHUTDOWN_QUESTION_EVENT = "gdl:shutdown-question"

/** The modal manager shows each question it is handed. */
export function listenShutdownQuestion(
  callback: (question: ShutdownQuestion) => void
) {
  const handler = (event: Event) => {
    // Taken: the sender now waits for the answer.
    event.preventDefault()
    callback((event as CustomEvent<ShutdownQuestion>).detail)
  }
  window.addEventListener(SHUTDOWN_QUESTION_EVENT, handler)
  return () => window.removeEventListener(SHUTDOWN_QUESTION_EVENT, handler)
}

/**
 * Resolves whether a restart or an update install may go ahead. It asks only
 * when a local server is running or a download or install is in progress and
 * the close warning is switched on. A main that cannot say (it failed, or it
 * is an older one) never blocks the restart or the update.
 */
export async function confirmShutdownStops(
  action: Exclude<ShutdownQuestionAction, "quit">
): Promise<boolean> {
  let stops: { servers?: unknown; installs?: unknown } | undefined
  try {
    stops = await window.shutdownStops?.()
  } catch (error) {
    console.warn("Could not ask what a restart would stop", error)
    return true
  }
  const servers = stops?.servers === true
  const installs = stops?.installs === true
  if (!servers && !installs) return true
  return new Promise<boolean>((resolve) => {
    const question: ShutdownQuestion = {
      action,
      game: false,
      servers,
      installs,
      answer: resolve
    }
    // Nothing listening (the modal manager is not mounted) must not leave
    // the restart or the update waiting for an answer nobody can give.
    const delivered = !window.dispatchEvent(
      new CustomEvent(SHUTDOWN_QUESTION_EVENT, {
        detail: question,
        cancelable: true
      })
    )
    if (!delivered) resolve(true)
  })
}
