/**
 * The `data-testid` values the e2e suite drives.
 *
 * Kept in one place so renaming an anchor is a single edit rather than a
 * search through specs. Deliberately few: only the elements a test must click
 * or wait on carry one.
 */
export const TEST_IDS = Object.freeze({
  termsCheckbox: "login-terms-checkbox",
  welcomeContinue: "login-welcome-continue",
  termsContinue: "login-terms-continue",
  useDeviceCode: "login-use-device-code",
  deviceCode: "login-device-code",
  gdlSyncAccount: "login-gdl-sync-account",
  libraryRoot: "library-root",
  betaPromptNever: "beta-prompt-never",
  addInstance: "library-add-instance",
  instanceCreationCustomTab: "instance-creation-custom-tab",
  instanceCreationName: "instance-creation-name",
  instanceCreationVersionTrigger: "instance-creation-version-trigger",
  // Prefix only — the five loader buttons (vanilla, forge, neoforge, fabric,
  // quilt) each append their key. The vanilla entry has no modloader key, so
  // its anchor is the stable literal "instance-creation-loader-vanilla"
  // rather than "-undefined". Use `byLoader` rather than concatenating by
  // hand.
  loaderOption: "instance-creation-loader",
  instanceCreationLoaderVersionTrigger:
    "instance-creation-loader-version-trigger",
  instanceCreationSubmit: "instance-creation-submit",
  instanceTile: "instance-tile",
  instancePlay: "instance-play",
  instanceContextDelete: "instance-context-delete",
  confirmInstanceDeletion: "confirm-instance-deletion",
  // Generic close (X) rendered by ModalLayout's header — present on every
  // modal that doesn't opt out with `noHeader`. Used here to dismiss the
  // changelogs modal on a fresh runtime path.
  modalClose: "modal-close",
  // The onboarding wizard has no header (`noHeader: true`), so its own
  // dismissal path is stepping through it: two "Next" clicks (the same
  // testid on both, since only one step is ever mounted at a time) into
  // the final step's "Skip" control.
  onboardingNext: "onboarding-next",
  onboardingSkip: "onboarding-skip",

  // Search: the expanded query input on `/search` (same testid on the
  // collapsed trigger input too, since only one of the pair is ever
  // mounted at a time).
  searchInput: "search-input",
  // Platform filter radios in the search sidebar. Fixed, two-valued set
  // (curseforge/modrinth), so no `byPlatform`-style helper — unlike
  // `loaderOption`, these render twice at once in the live DOM (the
  // sidebar's docked panel and its hover flyout are both permanently
  // mounted, only swapped visually), so only the docked copy carries the
  // testid; the flyout's is deliberately absent, not a second match.
  searchPlatformCurseforge: "search-platform-curseforge",
  searchPlatformModrinth: "search-platform-modrinth",
  // A search-result row (list or grid view — whichever is active, only
  // one is ever mounted). Combine with `data-project-id`, the same way
  // `instanceTile`/`byInstanceName` pairs a generic testid with a keyed
  // attribute: `[data-testid="search-result-row"][data-project-id="..."]`.
  searchResultRow: "search-result-row",
  // The addon page's primary install/download button. `ModDownloadButton`
  // renders in several places at once that can all be on screen
  // simultaneously (search result rows, version rows, and — on the addon
  // page itself — a sticky-header icon-only duplicate of this same
  // button); this testid is wired only to the addon page's main header
  // instance, so it never has more than one match.
  addonInstallButton: "addon-install-button",
  // A row in the instance Addons tab, keyed by the mod's on-disk base
  // filename via `byModRow`. Filename (not the row's own database id) is
  // the stable key across an enable/disable toggle — the backend only
  // renames the file (appending/stripping `.disabled`); the cached
  // `filename` this UI reads is already the base name on both sides of
  // that rename (`managers/instance/mods.rs`'s `enable_mod`,
  // `managers/metadata/cache/mod.rs`'s disk-scan reconciliation), so it
  // never needs stripping here.
  modRow: "mod-row",
  // Controls inside a mod row. Not individually keyed by filename — scope
  // a query to the row first (`byModRow`) and find these underneath it,
  // the same way a specific instance's controls are found by scoping to
  // its tile rather than baking the instance name into every button's own
  // testid.
  modRowToggle: "mod-row-toggle",
  modRowDelete: "mod-row-delete",
  modRowUpdate: "mod-row-update"
})

export function byTestId(id: string): string {
  return `[data-testid="${id}"]`
}

/** An instance tile located by the name the test gave it. */
export function byInstanceName(name: string): string {
  return `[data-testid="instance-tile"][data-instance-name="${name}"]`
}

/** A modloader picker button located by its key ("vanilla", "forge", ...). */
export function byLoader(key: string): string {
  return byTestId(`${TEST_IDS.loaderOption}-${key}`)
}

/**
 * A mod row in the instance Addons tab, located by the mod's on-disk base
 * filename — stable across enable/disable (see `TEST_IDS.modRow`'s doc
 * comment for why). Scope further queries under this locator to reach a
 * specific row's toggle/delete/update controls, e.g.
 * `page.locator(byModRow(filename)).locator(byTestId(TEST_IDS.modRowToggle))`.
 */
export function byModRow(filename: string): string {
  return `[data-testid="${TEST_IDS.modRow}"][data-mod-filename="${filename}"]`
}
