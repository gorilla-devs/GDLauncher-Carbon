import type { ListInstance } from "@gd/core_module/bindings"
import type { JavaMismatchData } from "./javaMismatchBridge"

/** A game that crashed on a Java older than one of its mods needs, as the
 *  crash help dialog shows it. Or, with `beforeLaunch`, a launch on a Java
 *  that doesn't fit the instance, held back to ask first. */
export interface JavaIssuePrompt {
  instanceId: number
  instanceName: string
  /** The oldest Java the crash says works; before a launch, the Java
   *  Automatic would give the instance. */
  neededMajor: number
  usedMajor: number
  /** The Java the fix uses: the oldest installed or downloadable one of
   *  `neededMajor` or later; before a launch, one that fits. */
  fixMajor: number
  /** The game hasn't run: PLAY found its Java doesn't fit. */
  beforeLaunch?: boolean
}

/** The dialog for a launch the core held back on a Java that doesn't fit. */
export function javaMismatchPrompt(data: JavaMismatchData): JavaIssuePrompt {
  return {
    instanceId: data.instanceId,
    instanceName: data.instanceName,
    neededMajor: data.neededMajor,
    usedMajor: data.usedMajor,
    fixMajor: data.fixMajor,
    beforeLaunch: true
  }
}

/** For each instance, the needed Javas already asked about. */
export type ShownJavaIssues = Map<number, Set<number>>

/** The instances' Java issues, in list order, leaving out those already in
 *  `shown`. */
export function javaIssuesToShow(
  instances: ListInstance[],
  shown: ShownJavaIssues
): JavaIssuePrompt[] {
  const prompts: JavaIssuePrompt[] = []

  for (const instance of instances) {
    if (instance.status.status !== "valid") continue
    const issue = instance.status.value.java_issue
    if (!issue || shown.get(instance.id)?.has(issue.neededMajor)) continue

    prompts.push({
      instanceId: instance.id,
      instanceName: instance.name,
      neededMajor: issue.neededMajor,
      usedMajor: issue.usedMajor,
      fixMajor: issue.fixMajor
    })
  }

  return prompts
}

/** This session's crash prompts. Each crash is asked about once, whatever
 *  the user does with it; an instance's next launch clears its issue, so a
 *  crash after that is asked about again. */
export function createJavaIssuePrompts() {
  const shown: ShownJavaIssues = new Map()

  return {
    /** The prompts to open now, counted as shown. */
    take(instances: ListInstance[]): JavaIssuePrompt[] {
      for (const instance of instances) {
        if (
          instance.status.status === "valid" &&
          instance.status.value.java_issue === null
        ) {
          shown.delete(instance.id)
        }
      }

      const prompts = javaIssuesToShow(instances, shown)
      for (const prompt of prompts) {
        const needed = shown.get(prompt.instanceId) ?? new Set<number>()
        needed.add(prompt.neededMajor)
        shown.set(prompt.instanceId, needed)
      }
      return prompts
    }
  }
}
