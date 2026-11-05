let active = false
let brief = false
let interruptible: AbortController | undefined
const whenIdle = new Set<() => void>()
export function assertRuntimeOperationAvailable() {
  // A move, a restart, a database reset or a brief step such as removing old
  // recovery copies can hold it, so the refusal names none of them.
  if (active)
    throw new Error(
      "GDLauncher is still finishing another task. Try again when it is done."
    )
}
export function runtimeOperationActive() {
  return active
}
/**
 * Whether the running operation is a brief one: it ends in moments without a
 * restart and has nothing the user must see, so a quit that comes meanwhile
 * waits for it and then runs.
 */
export function runtimeOperationBrief() {
  return active && brief
}
export async function withRuntimeOperation<T>(
  operation: () => Promise<T>,
  options: { brief?: boolean } = {}
): Promise<T> {
  assertRuntimeOperationAvailable()
  active = true
  brief = options.brief ?? false
  try {
    return await operation()
  } finally {
    active = false
    brief = false
    interruptible = undefined
    const callbacks = [...whenIdle]
    whenIdle.clear()
    for (const callback of callbacks) queueMicrotask(callback)
  }
}

/**
 * Lets the user's quit or last-window close end the running operation while
 * it has not changed anything yet. `signal` aborts on such a request. Call
 * `commit()` right before the first change: it throws the abort reason if the
 * request already came, and from then on a quit waits for the operation.
 * Only valid inside `withRuntimeOperation`, which drops it when it ends.
 */
export function interruptibleUntilCommitted(): {
  signal: AbortSignal
  commit(): void
} {
  const controller = new AbortController()
  interruptible = controller
  return {
    signal: controller.signal,
    commit() {
      if (interruptible === controller) interruptible = undefined
      controller.signal.throwIfAborted()
    }
  }
}

/**
 * Ends the running operation if it has not committed to a change yet.
 * Returns whether it did; an operation past its commit runs to its end.
 */
export function interruptRuntimeOperation(reason: Error): boolean {
  if (!interruptible) return false
  interruptible.abort(reason)
  return true
}

/**
 * Runs `callback` once no runtime operation is active: right away if none is,
 * otherwise after the current one ends. Registering the same function twice
 * runs it once.
 */
export function whenRuntimeOperationIdle(callback: () => void) {
  if (!active) callback()
  else whenIdle.add(callback)
}
