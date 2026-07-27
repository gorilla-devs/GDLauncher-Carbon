/**
 * Pure resolution logic: the model of "which build should the app have
 * picked", kept free of Playwright so it is unit-testable.
 *
 * Two predicates, not one, because install and update resolve differently.
 * See `.superpowers/specs/2026-07-27-e2e-mod-resolution-design.md`.
 */

export type ModChannel = "alpha" | "beta" | "stable"

/** Matches `ModChannel` in `crates/carbon_platforms` — Alpha = 0, Beta, Stable. */
const CHANNEL_RANK: Record<ModChannel, number> = {
  alpha: 0,
  beta: 1,
  stable: 2
}

/** The order `AppConfiguration.modChannels` ships by default:
 *  `'stable:true,beta:true,alpha:true'`, all three allowing updates. */
const DEFAULT_CHANNEL_ORDER: ModChannel[] = ["stable", "beta", "alpha"]

export interface ResolutionCandidate {
  /** The platform's own version/file id, as a string on both platforms —
   *  a Modrinth version id, or a CurseForge file id stringified. */
  id: string
  /** ISO 8601. Modrinth `date_published`, CurseForge `fileDate`. */
  datePublished: string
  channel: ModChannel
  /** Minecraft versions this build declares. */
  gameVersions: string[]
  /** Loaders this build declares, lowercased. */
  loaders: string[]
}

/** Exported so failure messages and the prove-it-can-fail steps can reach the
 *  same ordering the predicates use, rather than re-deriving it. */
export function sortNewestFirst(
  candidates: ResolutionCandidate[]
): ResolutionCandidate[] {
  return [...candidates].sort(
    (a, b) => Date.parse(b.datePublished) - Date.parse(a.datePublished)
  )
}

/**
 * Throws when more than one candidate shares `sorted[0]`'s publish date.
 * The app picks one of them and nothing determines which, so an assertion
 * against either is a coin flip dressed as an assertion — a named throw is
 * strictly better than a test that passes half the time.
 */
function assertUnambiguous(
  sorted: ResolutionCandidate[],
  label: string
): ResolutionCandidate {
  const top = Date.parse(sorted[0].datePublished)
  const tied = sorted.filter((c) => Date.parse(c.datePublished) === top)
  if (tied.length > 1) {
    throw new Error(
      `${label}: ${tied.length} candidates share the newest publish date ` +
        `(${sorted[0].datePublished}): ${tied.map((c) => c.id).join(", ")} — ` +
        `"newest" is ambiguous here, so asserting on either would be a coin flip`
    )
  }
  return sorted[0]
}

/**
 * The **install** path's notion of latest: newest by publish date, with no
 * channel filter at all. `install_latest_modrinth_mod` takes `.get(0)` of the
 * filtered response and `install_latest_curseforge_mod` takes the first entry
 * whose `game_versions` contains the release; neither consults a channel.
 */
export function newestByDate(
  candidates: ResolutionCandidate[]
): ResolutionCandidate {
  if (candidates.length === 0) {
    throw new Error("newestByDate: candidate list is empty")
  }
  return assertUnambiguous(sortNewestFirst(candidates), "newestByDate")
}

/**
 * The **update** path's notion of latest: `find_mod_update` sorts newest-first
 * and walks the allowed channels in list order, taking the first candidate at
 * or above that channel's level. Under the shipped default the stable pass
 * runs first, so this is the newest *stable* build — falling back to beta and
 * then alpha only when no candidate of the preceding channel exists.
 */
export function newestUpdateCandidate(
  candidates: ResolutionCandidate[]
): ResolutionCandidate {
  if (candidates.length === 0) {
    throw new Error("newestUpdateCandidate: candidate list is empty")
  }

  const sorted = sortNewestFirst(candidates)
  for (const channel of DEFAULT_CHANNEL_ORDER) {
    const eligible = sorted.filter(
      (c) => CHANNEL_RANK[c.channel] >= CHANNEL_RANK[channel]
    )
    if (eligible.length > 0) {
      return assertUnambiguous(
        eligible,
        `newestUpdateCandidate (${channel} pass)`
      )
    }
  }

  throw new Error(
    "newestUpdateCandidate: no candidate matched any allowed channel — " +
      `channels present: ${[...new Set(candidates.map((c) => c.channel))].join(", ")}`
  )
}

/** Modrinth `version_type` -> channel. Throws rather than defaulting: a
 *  silently-defaulted channel would quietly corrupt `newestUpdateCandidate`. */
export function modrinthChannel(versionType: string): ModChannel {
  switch (versionType.toLowerCase()) {
    case "release":
      return "stable"
    case "beta":
      return "beta"
    case "alpha":
      return "alpha"
    default:
      throw new Error(
        `modrinthChannel: unrecognised version_type "${versionType}"`
      )
  }
}

/**
 * CurseForge `releaseType` -> channel.
 *
 * A **string** on the wire, despite the Rust `ReleaseType` enum being
 * `#[repr(i32)]` — confirmed live in Task 1, where all eight sampled Cloth
 * Config files at Minecraft 1.20.1 carried `"releaseType": "stable"`.
 * `"stable"` is the only value this project actually observed; `"beta"` and
 * `"alpha"` are the enum's other two members and are covered by unit tests
 * rather than by live evidence.
 *
 * Rejects a numeric input rather than coercing it: if the wire representation
 * ever reverts to the integer the Rust source implies, that must surface as a
 * named failure, not as a silent mapping to the wrong channel.
 */
export function curseforgeChannel(releaseType: string): ModChannel {
  if (typeof releaseType !== "string") {
    throw new Error(
      `curseforgeChannel: expected a string releaseType, got ` +
        `${typeof releaseType} (${String(releaseType)}) — the wire ` +
        "representation has changed and the mapping must be re-confirmed"
    )
  }
  switch (releaseType.toLowerCase()) {
    case "stable":
      return "stable"
    case "beta":
      return "beta"
    case "alpha":
      return "alpha"
    default:
      throw new Error(
        `curseforgeChannel: unrecognised releaseType "${releaseType}"`
      )
  }
}
