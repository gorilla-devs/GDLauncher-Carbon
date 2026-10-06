import type { DuplicatedMod, ModVersion } from "./ModSelectionStep"

/** Validate the whole choice before dispatching any destructive operation. */
export function duplicateResolutionPlan(
  groups: readonly DuplicatedMod[],
  selected: Readonly<Record<string, string>>
): readonly ModVersion[] | undefined {
  if (!groups.length) return undefined
  const groupKeys = new Set<string>()
  const members = new Set<string>()
  const plan: ModVersion[] = []
  for (const group of groups) {
    if (!group.groupKey || groupKeys.has(group.groupKey)) return undefined
    groupKeys.add(group.groupKey)
    if (
      group.versions.filter((v) => v.id === selected[group.groupKey]).length !==
      1
    )
      return undefined
    for (const version of group.versions) {
      if (!version.id || members.has(version.id)) return undefined
      members.add(version.id)
      if (version.id !== selected[group.groupKey])
        plan.push(Object.freeze({ ...version }))
    }
  }
  return Object.freeze(plan)
}
