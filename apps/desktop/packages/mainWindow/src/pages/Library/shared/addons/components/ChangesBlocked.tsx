import type { InstanceDetails } from "@gd/core_module/bindings"
import { Trans } from "@gd/i18n"
import { Show } from "solid-js"

/** Why changes to an instance's data packs are refused: a locked modpack, or
 *  a game that is queued, preparing or running. */
export type ChangesBlocked = "locked" | "busy" | null

export const changesBlocked = (
  details: Pick<InstanceDetails, "modpack" | "state"> | null | undefined
): ChangesBlocked => {
  if (details?.modpack?.locked) return "locked"
  const state = details?.state?.state
  if (state === "queued" || state === "preparing" || state === "running")
    return "busy"
  return null
}

export const BlockedText = (props: { blocked: ChangesBlocked }) => (
  <Show
    when={props.blocked === "locked"}
    fallback={<Trans key="content:_trn_worlds.busy" />}
  >
    <Trans key="instances:_trn_locked_cannot_apply_changes" />
  </Show>
)
