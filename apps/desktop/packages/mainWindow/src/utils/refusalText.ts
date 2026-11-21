import type { QueryClient } from "@tanstack/solid-query"
import { i18n, type NamespacedTranslationKey } from "@gd/i18n"

// ---------------------------------------------------------------------------
// The core's refusals of a change to a world or its data packs, in the
// player's language.
//
// The core sends each refusal with a code, its data and an English sentence.
// It reaches the player as a refused request, a failed task, a place of
// "Update everywhere" or an addon of a deleted selection: every one of those
// words it here, from the code.
// ---------------------------------------------------------------------------

interface Refusal {
  key: NamespacedTranslationKey
  /** The fields of the refusal's data its sentence names. */
  names?: readonly string[]
  /** The world list read before the refusal is no longer what is on disk. */
  outdatesWorlds?: true
}

const worldChanged: Refusal = {
  key: "content:_trn_worlds.refusal.world_changed",
  outdatesWorlds: true
}
const datapackChanged: NamespacedTranslationKey =
  "content:_trn_worlds.refusal.datapack_changed"

const REFUSALS: Record<string, Refusal> = {
  INSTANCE_GAME_ACTIVE: { key: "content:_trn_worlds.busy" },
  WORLD_GONE: {
    key: "content:_trn_worlds.refusal.world_gone",
    outdatesWorlds: true
  },
  WORLD_NOT_A_SAVE: { key: "content:_trn_worlds.unreadable" },
  WORLD_CHANGED: worldChanged,
  WORLD_CHANGED_DURING_COPY: worldChanged,
  DATAPACK_NAME_TAKEN: {
    key: "content:_trn_worlds.refusal.name_taken",
    names: ["filename"]
  },
  DATAPACK_GONE: {
    key: "content:_trn_worlds.refusal.datapack_gone",
    outdatesWorlds: true
  },
  DATAPACK_CHANGED: { key: datapackChanged, outdatesWorlds: true },
  DATAPACK_IN_OTHER_WORLD: { key: "content:_trn_worlds.refusal.other_world" },
  DATAPACK_FOLDER_NOT_REPLACED: {
    key: "content:_trn_worlds.refusal.folder_not_replaced"
  },
  NOT_A_DATAPACK_VERSION: {
    key: "content:_trn_worlds.refusal.not_a_datapack_version"
  },
  DATAPACK_NOT_A_ZIP: { key: "content:_trn_worlds.refusal.not_a_zip" },
  DATAPACK_INVALID: { key: "content:_trn_worlds.refusal.invalid_datapack" },
  DATAPACK_NOT_DOWNLOADABLE: {
    key: "content:_trn_worlds.refusal.not_downloadable"
  },
  DATAPACK_DOWNLOAD_FAILED: {
    key: "content:_trn_worlds.refusal.download_failed",
    names: ["filename"]
  },
  NO_DATAPACK_UPDATE: { key: "content:_trn_worlds.refusal.no_update" },
  NO_DATAPACK_FOR_VERSION: {
    key: "content:_trn_worlds.no_datapack_for_version",
    names: ["version"]
  },
  INSTANCE_DATAPACK_GONE: {
    key: "content:_trn_worlds.refusal.instance_datapack_gone"
  },
  INSTANCE_DATAPACK_CHANGED: { key: datapackChanged },
  WORLD_NAME_UNREADABLE: { key: "content:_trn_worlds.unreadable" },
  WORLD_NAME_INVALID: { key: "content:_trn_worlds.refusal.name_invalid" },
  WORLD_RENAME_INCOMPLETE: {
    key: "content:_trn_worlds.refusal.rename_incomplete",
    names: ["folder"],
    outdatesWorlds: true
  }
}

const refusal = (code: string | null | undefined): Refusal | undefined =>
  code && Object.prototype.hasOwnProperty.call(REFUSALS, code)
    ? REFUSALS[code]
    : undefined

/** Whether `code` is a refusal with a sentence of its own here. */
export function isRefusal(code: string | null | undefined): boolean {
  return refusal(code) !== undefined
}

/** The sentence for a refusal, in the player's language. `display`, the
 *  core's own English sentence, stands in for a code this build does not
 *  know, for data that lacks a field the sentence names, and before the
 *  translations are loaded. */
export function refusalText(
  code: string | null | undefined,
  data?: unknown,
  display?: string | null
): string | undefined {
  const known = refusal(code)
  if (known) {
    const sent = (data ?? {}) as Record<string, unknown>
    const names = known.names ?? []
    if (names.every((name) => typeof sent[name] === "string")) {
      // Shown as text in a toast: a file name must not be HTML-escaped.
      const text = i18n.t(known.key, {
        ...Object.fromEntries(names.map((name) => [name, sent[name]])),
        interpolation: { escapeValue: false }
      })
      if (text) return text
    }
  }
  return display || undefined
}

/** After a refusal that says the world or a pack in it is not what the list
 *  showed, has the worlds read again, so a second try starts from what is on
 *  disk: those of `instanceId`, or of every instance when it is not known. */
export function refreshWorldsAfter(
  code: string | null | undefined,
  client: Pick<QueryClient, "invalidateQueries">,
  instanceId?: number
) {
  if (!refusal(code)?.outdatesWorlds) return
  void client.invalidateQueries({
    queryKey:
      instanceId === undefined
        ? ["instance.getWorlds"]
        : ["instance.getWorlds", instanceId]
  })
}
