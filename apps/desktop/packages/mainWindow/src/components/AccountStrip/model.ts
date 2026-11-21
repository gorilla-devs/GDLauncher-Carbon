import type { AccountStatus } from "@gd/core_module/bindings"

/** How many accounts besides the active one show as faces; the rest are
 *  counted in "+N". */
export const MAX_FACES = 3

/** What the open strip shows: the active account by name, a few others as
 *  faces in list order, and how many more there are. */
export function stripAccounts<A extends { uuid: string }>(
  accounts: readonly A[],
  activeUuid: string | null | undefined,
  maxFaces = MAX_FACES
): { active: A | undefined; faces: A[]; more: number } {
  const active = accounts.find((a) => a.uuid === activeUuid)
  const others = accounts.filter((a) => a !== active)
  return {
    active,
    faces: others.slice(0, maxFaces),
    more: Math.max(0, others.length - maxFaces)
  }
}

/** Whether an account needs looking at: a warning while it is expired or
 *  refreshing, an error when it is invalid. */
export function needsAttention(
  status: AccountStatus
): "warning" | "error" | null {
  if (status === "invalid") return "error"
  if (status === "expired" || status === "refreshing") return "warning"
  return null
}
