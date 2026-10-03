let active = false
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
  }
}
