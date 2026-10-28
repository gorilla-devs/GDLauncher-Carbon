import { runCleanupSteps } from "./cleanup.js"
export interface AdoptionOwner<A> {
  initial: A | null
  initialQuit: boolean
  reopened: A | null
  initialExited: boolean
  reopenedExited: boolean
}
export function adoptionOwner<A>(): AdoptionOwner<A> {
  return {
    initial: null,
    initialQuit: false,
    reopened: null,
    initialExited: true,
    reopenedExited: true
  }
}
export async function leaveOwnedGameRunning<A, T>(
  owner: AdoptionOwner<A>,
  deps: {
    launch: () => Promise<A>
    prepareAndPlay: (app: A) => Promise<T>
    quit: (app: A) => Promise<void>
    verifySurvival: (game: T) => Promise<void>
  }
): Promise<T> {
  const current = await deps.launch()
  owner.initial = current
  owner.initialExited = false
  const game = await deps.prepareAndPlay(current)
  await deps.quit(current)
  owner.initialQuit = true
  owner.initialExited = true
  await deps.verifySurvival(game)
  return game
}
/** Bound the entire close adapter; only its confirmed return authorizes deletion. */
async function confirmedClose<A>(
  app: A,
  close: (app: A) => Promise<void>,
  timeoutMs: number
) {
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    await Promise.race([
      close(app),
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error("Owned launcher exit unconfirmed")),
          timeoutMs
        )
      })
    ])
  } finally {
    clearTimeout(timer)
  }
}
export async function cleanupAdoption<A>(
  owner: AdoptionOwner<A>,
  alreadyFailed: boolean,
  deps: {
    logs: () => Promise<void>
    reapGames: () => Promise<void>
    close: (app: A) => Promise<void>
    stop: (preserveRuntime: boolean) => Promise<void>
    timeoutMs?: number
  }
): Promise<void> {
  await runCleanupSteps(
    [
      deps.logs,
      deps.reapGames,
      async () => {
        if (owner.initial && !owner.initialQuit) {
          await confirmedClose(
            owner.initial,
            deps.close,
            deps.timeoutMs ?? 30_000
          )
          owner.initialExited = true
        }
      },
      async () => {
        if (owner.reopened) {
          await confirmedClose(
            owner.reopened,
            deps.close,
            deps.timeoutMs ?? 30_000
          )
          owner.reopenedExited = true
        }
      },
      () => deps.stop(!owner.initialExited || !owner.reopenedExited)
    ],
    alreadyFailed,
    "adoption teardown also failed:"
  )
}
