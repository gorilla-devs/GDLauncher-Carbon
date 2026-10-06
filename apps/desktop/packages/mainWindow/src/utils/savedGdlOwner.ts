import { createMemo, onCleanup } from "solid-js"
import type { FEGDLAccount } from "@gd/core_module/bindings"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { queryClient, rspc } from "./rspcClient"

export interface SavedGdlOwnerIdentity {
  uuid: string
  microsoftOid: string
  friendCode: string
}

export interface SavedGdlOwner extends SavedGdlOwnerIdentity {
  generation: number
  profile: FEGDLAccount
}

/**
 * - `owned`: the saved UUID's keyed profile matches the displayed one.
 * - `loading`: the first keyed answer for the saved UUID has not arrived yet.
 * - `unavailable`: the keyed answer failed, is null or names someone else.
 */
export type SavedGdlOwnerStatus = "loading" | "owned" | "unavailable"

export const sameGdlOwner = (
  a: SavedGdlOwnerIdentity | undefined,
  b: SavedGdlOwnerIdentity | undefined
) =>
  !!a &&
  !!b &&
  a.uuid === b.uuid &&
  a.microsoftOid === b.microsoftOid &&
  a.friendCode === b.friendCode

/** The unkeyed displayed profile alone cannot associate it with settings UUID. */
export function useSavedGdlOwner() {
  const store = useGlobalStore()
  const savedUuid = () => store.settings.data?.gdlAccountId || undefined
  const displayed = () =>
    store.gdlAccount.data?.status === "valid"
      ? store.gdlAccount.data.value
      : undefined
  const peek = rspc.createQuery(() => ({
    queryKey: ["account.peekGdlAccount", savedUuid() || ""],
    enabled: !!savedUuid() && !!displayed()
  }))
  const keyed = (uuid: string) =>
    queryClient.getQueryData<FEGDLAccount | null>([
      "account.peekGdlAccount",
      uuid
    ])
  let live = true

  // solid-query hands every refetch over as a new `data` object, even when
  // nothing changed. Compare the identity as a string so a routine refetch
  // (including the ones this dialog's own mutations trigger) is not mistaken
  // for a different owner.
  const identityKey = createMemo(() =>
    JSON.stringify([
      savedUuid(),
      displayed()?.microsoftOid,
      displayed()?.friendCode
    ])
  )

  // Last keyed answer seen for each saved UUID. A UUID that was never answered
  // has no entry, so its first answer is only recorded: an owner captured from
  // it stays current. Once known, every change of that answer counts (to
  // another owner, to null, to a missing entry, or back again), which keeps
  // A -> B -> A from resurrecting an owner captured before the change.
  const knownPeek = new Map<string, string>()
  const peekRevision = createMemo<number>((revision) => {
    const uuid = savedUuid()
    // The keyed cache is not reactive; the observer's data is read for
    // tracking only.
    void peek.data
    if (!uuid) return revision
    const cached = keyed(uuid)
    const identity =
      cached === undefined
        ? "missing"
        : cached === null
          ? "null"
          : JSON.stringify([cached.microsoftOid, cached.friendCode])
    const previous = knownPeek.get(uuid)
    if (previous === undefined) {
      if (cached !== undefined) knownPeek.set(uuid, identity)
      return revision
    }
    knownPeek.set(uuid, identity)
    return previous === identity ? revision : revision + 1
  }, 0)

  // A memo rather than an effect: anything that captures an owner reads the
  // settled generation, whatever order Solid runs the surrounding effects in.
  const generation = createMemo<number>((n) => {
    identityKey()
    peekRevision()
    return n + 1
  }, -1)

  onCleanup(() => {
    live = false
  })

  const identity = (): SavedGdlOwnerIdentity | undefined => {
    const uuid = savedUuid(),
      profile = displayed()
    if (!live || !uuid || !profile) return undefined
    return {
      uuid,
      microsoftOid: profile.microsoftOid,
      friendCode: profile.friendCode
    }
  }

  const capture = (): SavedGdlOwner | undefined => {
    const uuid = savedUuid(),
      profile = displayed()
    const observed = peek.data
    // Reading the exact keyed cache also refuses a retained old observer value
    // while the separately invalidated saved link moves to another UUID. A
    // background refetch keeps the cached answer: if it comes back as someone
    // else, the generation moves and this owner is retired.
    const confirmed = uuid ? keyed(uuid) : undefined
    if (
      !live ||
      !uuid ||
      !profile ||
      peek.isPending ||
      peek.isError ||
      !observed ||
      observed.microsoftOid !== confirmed?.microsoftOid ||
      observed.friendCode !== confirmed.friendCode ||
      confirmed.microsoftOid !== profile.microsoftOid ||
      confirmed.friendCode !== profile.friendCode
    )
      return undefined
    return {
      uuid,
      microsoftOid: profile.microsoftOid,
      friendCode: profile.friendCode,
      generation: generation(),
      profile: { ...profile }
    }
  }

  const status = (): SavedGdlOwnerStatus => {
    if (capture()) return "owned"
    const uuid = identity()?.uuid
    if (!uuid) return "unavailable"
    // A check running after a failed one (query-core keeps isError while it
    // refetches with a cached answer) is checking, not a fresh failure, so a
    // Retry shows the checking line instead of the unchanged warning.
    if (peek.isError && peek.isFetching) return "loading"
    return peek.isPending && !peek.isError && keyed(uuid) === undefined
      ? "loading"
      : "unavailable"
  }

  const current = (owner: SavedGdlOwner | undefined) => {
    const profile = displayed(),
      uuid = savedUuid()
    if (
      !live ||
      owner?.generation !== generation() ||
      owner.uuid !== uuid ||
      owner.microsoftOid !== profile?.microsoftOid ||
      owner.friendCode !== profile?.friendCode
    )
      return false
    // Own successful mutations invalidate/refetch these queries. Background
    // loading/error does not erase an already certified owner; a definite
    // replacement/null result still revokes it and the revision stays retired.
    const observed = peek.data
    const confirmed = keyed(owner.uuid)
    const matches = (data: FEGDLAccount | null | undefined) =>
      data === undefined ||
      (!!data &&
        data.microsoftOid === owner.microsoftOid &&
        data.friendCode === owner.friendCode)
    return !!confirmed && matches(observed) && matches(confirmed)
  }

  const refetch = async () => {
    // Without a saved UUID and a displayed profile the query is disabled and
    // its key is empty, so there is nothing meaningful to ask the core.
    if (!identity()) return
    try {
      await peek.refetch()
    } catch {
      // The result lands in the query state; the caller re-reads it.
    }
  }

  return { capture, current, status, identity, generation, refetch }
}
