import path from "node:path"

/** Every addon type that is not `mods`. Values are `AddonType`'s serialised
 *  form, as it arrives on `Mod.addon_type`. */
export type NonModAddonType =
  | "resourcepacks"
  | "shaders"
  | "datapacks"
  | "worlds"

/**
 * On-disk folder per addon type, mirroring `AddonType::get_folder_path`
 * (`crates/carbon_app/src/domain/instance/mod.rs`).
 *
 * Two of these do not match their type name and are exactly what the placement
 * spec exists to pin: `shaders` installs into `shaderpacks/`, and `worlds`
 * installs into `saves/`.
 */
export const ADDON_FOLDER: Record<NonModAddonType, string> = {
  resourcepacks: "resourcepacks",
  shaders: "shaderpacks",
  datapacks: "datapacks",
  worlds: "saves"
}

export const addonDir = (
  instanceRoot: string,
  addonType: NonModAddonType
): string => path.join(instanceRoot, ADDON_FOLDER[addonType])

export interface AddonFixture {
  addonType: NonModAddonType
  platform: "curseforge" | "modrinth"
  /** Search string that reaches the project as the first result. */
  query: string
  /** Platform project id, confirmed live — never read off a search page. */
  projectId: string
  /** The search page's content-type filter, i.e. `FEUnifiedSearchType`'s
   *  serialised form. Note it is singular where `addonType` is plural. */
  searchType: string
}

/**
 * Six combinations, not eight — two real platform asymmetries, both verified
 * live and documented here so neither gets "fixed" later by someone assuming
 * it's an oversight:
 *
 * - **Worlds are CurseForge-only.** Modrinth has no world project type
 *   (`FEUnifiedSearchType::World` maps to `ProjectType::Unknown`,
 *   `api/modplatforms/responses.rs`).
 * - **Datapacks are CurseForge-only too, for a different reason.** Installing
 *   a Modrinth addon routes its destination folder through
 *   `ModrinthModInstaller::get_install_path`
 *   (`managers/instance/installer/mod.rs:967-976`), which switches purely on
 *   the project's own `project_type` field (fetched fresh from Modrinth's
 *   `/v2/project` at install time) — never on which search tab/`searchType`
 *   reached it, and never on a specific version's own `loaders` list. On
 *   2026-08-05 I ran a full census of every Modrinth project MC-1.20.1 search
 *   associates with the datapack content type: paginated the
 *   `project_type:datapack` + `versions:1.20.1` search facet to its reported
 *   total (4,801 hits, all of them), then batch-checked each one's actual
 *   `project_type` via `GET /v2/projects`. Zero had the literal value
 *   `"datapack"` — the ecosystem files this content as `"mod"` plus a
 * `datapack` loader instead (confirmed directly for the candidates considered:
 *   VeinMiner, Terralith, and Tectonic are all
 *   `project_type: "mod"`). Picking one of those anyway would install into
 *   `mods/`, silently testing the app's *mod* routing instead of its datapack
 *   routing while reading as datapack coverage. The methodology was three
 * independent search strategies plus this census, sanity-checked by confirming
 * the same method is 20/20 reliable for
 *   `resourcepack`/`shader` (i.e. this is a real ecosystem gap, not a search
 *   or tooling artifact). If Modrinth's ecosystem changes, re-run that census
 *   before adding a `datapacks`/`modrinth` row back — a project surfacing
 *   under the search facet is not sufficient evidence on its own, as this
 *   census demonstrates.
 *
 * Every id below was confirmed live against the running platform APIs, never
 * read off a search page, per five criteria: published for Minecraft 1.20.1;
 * at least two release-channel versions; no declared dependencies; small
 * files (a real CDN download on every CI run, on three OSes).
 *
 * "At least two versions" is necessary but **not sufficient** for
 * `addonLifecycle.spec.ts`'s version move, and the Modrinth resource-pack
 * entry below is the proof: its two newest 1.20.1 builds are byte-identical
 * (both `96932a034fff4cd0cad08ba12db730450c4772da`), and the app identifies
 * an installed file by hashing it and asking the platform which version owns
 * that hash — so installing the second-newest reconciles as the newest, and
 * a "move to a newer build" against it moves nowhere. Two versions with
 * **distinct file hashes** is the real requirement; check the hashes, not
 * just the count, before swapping any fixture here.
 * The `resourcepacks`/`shaders` Modrinth entries' `project_type` is checked
 * directly rather than inferred from a search facet.
 */
export const ADDON_FIXTURES: AddonFixture[] = [
  // CurseForge classId 12 (ResourcePacks). 4 release files for 1.20.1,
  // 8-13KB each, zero declared dependencies. classId confirmed directly off
  // `/v1/mods/538850` (`12`), not merely off the classId-scoped search that
  // found it.
  {
    addonType: "resourcepacks",
    platform: "curseforge",
    query: "Eclectic Trove",
    projectId: "538850",
    searchType: "resourcePack"
  },
  // Modrinth `project_type` confirmed directly (not just the search facet,
  // which is unreliable for `datapack`) to be the
  // literal string "resourcepack". 3 release versions for 1.20.1, 42-48KB
  // each, zero declared dependencies.
  {
    addonType: "resourcepacks",
    platform: "modrinth",
    query: "Low On Fire",
    projectId: "RRxvWKNC",
    searchType: "resourcePack"
  },
  // CurseForge classId 6552 (Shaders), confirmed directly off
  // `/v1/mods/544096`. 6 release files for 1.20.1, all 139KB, zero declared
  // dependencies.
  {
    addonType: "shaders",
    platform: "curseforge",
    query: "Sildurs Vibrant Shaders",
    projectId: "544096",
    searchType: "shader"
  },
  // Modrinth `project_type` confirmed directly to be the literal string
  // "shader". 20 release versions for 1.20.1, 265-534KB, zero declared
  // dependencies.
  {
    addonType: "shaders",
    platform: "modrinth",
    query: "Complementary Reimagined",
    projectId: "HVnmMxH1",
    searchType: "shader"
  },
  // CurseForge classId 6945 (Datapacks), confirmed directly off
  // `/v1/mods/831385`. Many release files for 1.20.1 (10+), 351-387KB, zero
  // declared dependencies.
  {
    addonType: "datapacks",
    platform: "curseforge",
    query: "Dynamic Lights",
    projectId: "831385",
    searchType: "datapack"
  },
  // CurseForge classId 17 (Worlds), confirmed directly off
  // `/v1/mods/1310753`. 4 release files for 1.20.1, 2.4-4.3MB (the smallest
  // multi-version, dependency-free world found; worlds cannot get anywhere
  // near the other types' sizes), zero
  // declared dependencies.
  {
    addonType: "worlds",
    platform: "curseforge",
    query: "Find The Button Plus 2",
    projectId: "1310753",
    searchType: "world"
  }
]
