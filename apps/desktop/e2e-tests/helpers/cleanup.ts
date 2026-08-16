/**
 * Applies this suite's cleanup-failure masking policy in one place: a
 * `throw` inside a `finally` block discards whatever the guarded body was
 * already throwing — JS semantics, not a Playwright reporting choice — so a
 * cleanup failure must never re-throw over a body that is already failing,
 * only over one that passed (otherwise a real body failure gets buried
 * under an unrelated cleanup error, and the actual cause is lost). `message`
 * is passed verbatim to `console.error` alongside the swallowed error, so
 * the cleanup failure is still visible even when it isn't what fails the
 * test.
 *
 * The `eslint-disable` for `no-unsafe-finally` lives here once, instead of
 * once per call site: this is the only place left in the suite where a
 * `finally` (by way of `withCleanup` below) still throws.
 */
export function reportCleanupFailure(
  cleanupError: unknown,
  alreadyFailed: boolean,
  message: string
): void {
  if (!alreadyFailed) {
    // Deliberate: this branch only runs when the guarded body succeeded, so
    // there is no body error here for the throw to discard.
    // eslint-disable-next-line no-unsafe-finally
    throw cleanupError
  }
  console.error(message, cleanupError)
}

/**
 * Runs `body`, tracking whether it failed, then always runs `cleanup` in a
 * `finally`. Centralizes the `bodyFailed`/try/catch/finally scaffolding so
 * no spec's cleanup site in this suite has to hand-write it.
 *
 * Failure is tracked with an explicit boolean rather than an
 * "error is undefined" sentinel: a literal `throw undefined` from `body`
 * would otherwise be misread as "the body succeeded" (same reasoning as
 * `hasFirstError` in `fixtures/mockIdp.ts`'s `stopHarness`).
 *
 * Two forms, depending on what a call site's cleanup actually needs:
 *
 * - `withCleanup(body, cleanup, cleanupErrorMessage)` — the common case: one
 *   cleanup step. A failure is routed through `reportCleanupFailure` with
 *   `cleanupErrorMessage`.
 * - `withCleanup(body, cleanup)` — `cleanup` itself receives whether the
 *   body already failed, and is responsible for applying
 *   `reportCleanupFailure` (or its own equivalent) to whatever it needs to
 *   guard. For call sites that run more than one independent cleanup step
 *   (each attempted regardless of whether an earlier one failed), or that
 *   have unconditional work to do after a guarded step — see
 *   `gameLaunch.spec.ts` and `modResolution.spec.ts` for real examples of
 *   each.
 */
export async function withCleanup<T>(
  body: () => Promise<T>,
  cleanup: () => Promise<void>,
  cleanupErrorMessage: string
): Promise<T>
export async function withCleanup<T>(
  body: () => Promise<T>,
  cleanup: (alreadyFailed: boolean) => Promise<void>
): Promise<T>
export async function withCleanup<T>(
  body: () => Promise<T>,
  cleanup: (() => Promise<void>) | ((alreadyFailed: boolean) => Promise<void>),
  cleanupErrorMessage?: string
): Promise<T> {
  let alreadyFailed = false
  try {
    return await body()
  } catch (error) {
    alreadyFailed = true
    throw error
  } finally {
    if (cleanupErrorMessage === undefined) {
      await (cleanup as (alreadyFailed: boolean) => Promise<void>)(
        alreadyFailed
      )
    } else {
      try {
        await (cleanup as () => Promise<void>)()
      } catch (cleanupError) {
        reportCleanupFailure(cleanupError, alreadyFailed, cleanupErrorMessage)
      }
    }
  }
}
