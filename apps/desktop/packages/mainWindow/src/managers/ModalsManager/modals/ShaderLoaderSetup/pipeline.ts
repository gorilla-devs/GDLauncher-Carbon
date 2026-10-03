// Keep completion sequencing independent from rendering so task disappearance and
// explicit file-only consent can be exercised without an Electron session.
export async function runShaderSteps(
  steps: (() => Promise<number | null>)[],
  wait: (task: number) => Promise<void>,
  verify: () => Promise<boolean>,
  fileOnly: boolean,
  destroyed: () => boolean,
  complete: (task: number | null) => void,
  afterTask?: (index: number) => Promise<void>
) {
  let last: number | null = null
  for (const [index, step] of steps.entries()) {
    if (destroyed()) return
    const task = await step()
    if (task !== null) {
      last = task
      await wait(task)
    }
    if (destroyed()) return
    await afterTask?.(index)
  }
  if (destroyed()) return
  if (!fileOnly && !(await verify()))
    throw new Error("The required shader loader is not enabled. Retry setup.")
  if (destroyed()) return
  complete(last)
}
