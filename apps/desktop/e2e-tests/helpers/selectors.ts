/**
 * The `data-testid` values the e2e suite drives.
 *
 * Kept in one place so renaming an anchor is a single edit rather than a
 * search through specs. Deliberately few: only the elements a test must click
 * or wait on carry one.
 *
 * Two hazards to check before adding an anchor here, both already hit on this
 * suite:
 *
 * 1. A `data-testid` on a `@gd/ui` component may not reach the DOM at all, or
 *    may reach an element that cannot be clicked. `Checkbox` does not forward
 *    unknown props; `Radio` and `Switch` do forward, but onto a `display:none`
 *    or zero-size native input. TypeScript accepts a hyphenated attribute on a
 *    component either way, so it type-checks regardless. Verify by reading the
 *    attribute back out of the running app's DOM, never by grepping source.
 *
 * 2. Anything under `pages/Search/FilterSidebar`'s `ExpandedPanel` renders
 *    twice — the docked panel and the hover flyout are both permanently
 *    mounted and only CSS-toggled — so an anchor placed there double-matches.
 *    `PlatformFilter` takes a `testAnchors` prop so only the docked copy
 *    carries ids; a new anchor in any sibling filter needs the same treatment.
 */
export const TEST_IDS = Object.freeze({
  termsCheckbox: "login-terms-checkbox",
  welcomeContinue: "login-welcome-continue",
  termsContinue: "login-terms-continue",
  useDeviceCode: "login-use-device-code",
  deviceCode: "login-device-code",
  gdlSyncAccount: "login-gdl-sync-account",
  libraryRoot: "library-root",
  // The navbar's wide logo (`Navbar.tsx`), the app's own "back to library"
  // control from anywhere. A plain `<img>`, not a `@gd/ui` component, so
  // hazard 1 above does not apply; it lives outside `FilterSidebar`, so
  // hazard 2 doesn't either. Added to replace a structural `nav img >>
  // first()` lookup that shared the account-avatar `<img>`'s tag and would
  // have silently clicked a wrong match instead of failing had a second
  // `<img>` ever been added under `<nav>`.
  navbarLogo: "navbar-logo",
  // The navbar's settings gear icon (`Navbar.tsx`), the app's only route to
  // `/settings`. A `@gd/ui` `TabsTrigger`, which spreads unknown props onto
  // Kobalte's own `<button>` (confirmed by reading `Tabs/index.tsx` — no
  // hazard-1 wrinkle here, unlike Checkbox/Radio/Switch), so this anchor
  // reaches a real clickable element directly.
  navbarSettings: "navbar-settings",
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

  // Cache Cleanup (Settings -> General -> "Clean up", and the modal it
  // opens). The two scope rows are anchored on the modal's own local
  // `ClickableRow` wrapper rather than on the `Checkbox` inside them:
  // `@gd/ui`'s `Checkbox` forwards no unknown props at all (hazard 1 in
  // this file's header), so a testid placed on it is dropped silently and
  // with no type error. Each row also exposes `data-checked`, which is the
  // only readable signal of that scope's state — the `Checkbox` renders no
  // queryable input either, so there is nothing else to assert against.
  //
  // The two `Button` anchors need no such workaround: `@gd/ui`'s `Button`
  // does spread unknown props onto the real `<button>` (the same reason
  // `helpers/mods.ts` can select on `button[type="primary"]`).
  settingsCacheCleanupOpen: "settings-cache-cleanup-open",
  cacheCleanupScopeGdlauncher: "cache-cleanup-scope-gdlauncher",
  cacheCleanupScopeMinecraft: "cache-cleanup-scope-minecraft",
  cacheCleanupStart: "cache-cleanup-start",
  // Rendered only by the modal's `phase() === "done"` branch, so its
  // presence is the completion signal a caller waits on.
  cacheCleanupDone: "cache-cleanup-done",

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
  modRowUpdate: "mod-row-update",
  // Entries in the Addons tab's own right-click menu
  // (`Library/Instance/Tabs/Addons/index.tsx`), single-selection branch —
  // a second route to the same actions the row's inline controls offer, and
  // the one a world's missing enable toggle has to be checked against
  // separately (hiding the column control alone left this menu still
  // offering it). Neither hazard in this file's header applies: `@gd/ui`'s
  // `ContextMenuItem` spreads unrecognised props onto Kobalte's polymorphic
  // `Menu.Item`, which renders a real, full-size `<div role="menuitem">` —
  // `worldLifecycle.spec.ts` proves the forwarding really happens by
  // asserting `addonContextDelete` is visible in the same open menu it
  // asserts `addonContextToggle` is absent from, so a testid silently
  // dropped by a future refactor fails loudly instead of passing the
  // absence check for the wrong reason.
  //
  // The menu must be opened first (right-click a `modRow`), and only one of
  // the two branches is ever mounted (`selectionCount() === 1` vs. the
  // multi-select fallback), so the multi-select entries — which carry no
  // anchors — can never double-match these.
  addonContextToggle: "addon-context-toggle",
  addonContextDelete: "addon-context-delete",
  // A row in the addon page's Versions tab (`AddonViewPage/Versions/index.tsx`),
  // keyed by the platform's own file/version id via `byAddonVersionRow` —
  // the same id `ModDownloadButton`'s `fileId` prop installs. This list is
  // virtualized (`@tanstack/solid-virtual`): only rows near the viewport are
  // ever mounted, so a query against this id only resolves once the target
  // has actually been scrolled into range. `helpers/mods.ts`'s
  // `scrollVersionRowIntoView` is what gets a row that far down mounted —
  // scrolling the virtualizer's own scroll parent and, past its end, paging
  // the infinite query, up to 40 viewport steps before giving up.
  addonVersionRow: "addon-version-row",
  // Settings > General's "Potato mode" (`reducedMotion`) toggle
  // (`pages/Settings/General.tsx`), used by `persistence.spec.ts` as the
  // "an app setting survives a restart" case. On a wrapping div, not the
  // Switch itself — same zero-size-native-input reason as `modRowToggle`
  // above; find the real `input[type="checkbox"]` underneath to read/click
  // it. Chosen over `discordIntegration`/`showAppCloseWarning` because it has
  // no side effect beyond a CSS class (no RPC connection attempt, no close
  // dialog to accidentally gate other flows on).
  settingsReducedMotionToggle: "settings-reduced-motion-toggle",

  // The confirm control on `WindowCloseWarning`, the modal raised when the
  // launcher is closed while a game is running. Clicking it is the only way
  // the quit ever completes: `main/index.ts`'s `win.on("close")` calls
  // `preventDefault()` and shows the modal instead, and this button's
  // `window.closeWindow()` is what destroys the window. A `@gd/ui` `Button`,
  // which forwards unknown props, so the anchor reaches the real
  // `<button>` — unlike the `Checkbox` beside it (hazard 1 in this file's
  // header), which is why only this control carries one.
  windowCloseWarningQuit: "window-close-warning-quit",

  // The Log tab's notice that the running game came from a previous launcher
  // session and has no live log (`Tabs/Log/LogsContent.tsx`). Rendered only
  // when the instance is Running with `adopted: true`, so its presence is the
  // frontend's whole observable answer to "was this instance adopted" — a
  // plain `<div>`, so neither hazard in this file's header applies.
  instanceAdoptedNoLiveLog: "instance-adopted-no-live-log",

  // The DB recovery ladder (`packages/preload/loading.ts`'s `fatalError`/
  // `backwardsMigrationError`, rendered into `#appFatalCrash` by
  // `packages/main/index.ts`'s failure path). Neither hazard above applies
  // here: this screen is raw `innerHTML` built from template strings in the
  // Electron preload, not a `@gd/ui` component (hazard 1) and not anywhere
  // near `FilterSidebar` (hazard 2) — confirmed by reading how it mounts,
  // not assumed. `fatalError` and `backwardsMigrationError` fully replace
  // `#appFatalCrash`'s contents and are mutually exclusive (only one status
  // event ever fires per launch), so the two screen-container ids below
  // never coexist, and `recoveryResetDbButton` is deliberately the same
  // testid on both screens for the same reason `searchInput`/`onboardingNext`
  // reuse one id across mutually-exclusive mounts.
  recoveryFatalScreen: "recovery-fatal-screen",
  recoveryBackwardsMigrationScreen: "recovery-backwards-migration-screen",
  recoveryRetryButton: "recovery-retry-button",
  // Present only when the core's `_STATUS_:DB_DOWNGRADE_FAILED` line carried
  // a snapshot path (`loading.ts`'s `restoreStepHtml`); absent, not merely
  // hidden, otherwise. A DOM query for this id is a real presence check, not
  // a visibility one.
  recoveryRestoreSnapshotButton: "recovery-restore-snapshot-button",
  recoveryResetDbButton: "recovery-reset-db-button",
  // The fatal screen's log text box — the core's own log lines plus the
  // `Database error: <EVENT>` line `main/index.ts` appends, rendered via
  // `textContent` (never interpolated as HTML; see `loading.ts`'s own
  // comment on why). Used to assert which status event actually drove the
  // screen, independent of which buttons happen to be present.
  recoveryErrorDetail: "recovery-error-detail",

  /** The addon page's modpack install button — `AddonExplore`'s header
   *  instance in `pages/AddonViewPage/index.tsx`. Gated only on
   *  `project.data?.type === "modpack"`, never on which sub-tab is active, so
   *  it is persistent chrome present on all four
   *  (Overview/Versions/Changelog/Screenshots) — the tabs render as
   *  `props.children` underneath it. Single-match because the Versions tab's
   *  per-row buttons carry a different id, `modpackVersionDownloadButton`,
   *  not this one — an unscoped query for this id on the Versions tab would
   *  otherwise resolve to this button plus one per rendered row. The
   *  sticky-header duplicate a few lines below stays unanchored: it is
   *  mounted simultaneously with this button and only CSS-toggled, so a
   *  shared anchor would double-match. */
  modpackDownloadButton: "modpack-download-button",
  /** The modpack install button rendered once per virtualized row on the
   *  addon page's Versions tab (`components/Browser/RowContainer.tsx`) — a
   *  different id from the header's `modpackDownloadButton` above because
   *  both are simultaneously mounted whenever the Versions tab is open. A row
   *  can also render a `ServerPackDownloadButton` beside it (CurseForge packs
   *  that ship a separate server pack), which is why this carries its own
   *  dedicated id rather than being reached as "the row's one descendant
   *  button" the way `ModDownloadButton`/`InstallButton` rows are — that
   *  selector would be ambiguous here. Scope a query under
   *  `byAddonVersionRow(fileId)` to reach one specific build's button. */
  modpackVersionDownloadButton: "modpack-version-download-button",
  /** The version dropdown's trigger in the change-version modal. Click it to
   *  open the listbox before querying for an option. */
  modpackVersionSelect: "modpack-version-select",
  /** One option in that listbox, carrying `data-version-id` — a Modrinth
   *  version id or a CurseForge file id. On a plain `<div>` inside
   *  `SelectItem` rather than on the component, per hazard 1. */
  modpackVersionOption: "modpack-version-option",
  modpackVersionUpdateConfirm: "modpack-version-update-confirm",
  /** Instance Settings, modpack block. `unlock` is only rendered while the
   *  instance is locked, and there is no re-lock control anywhere in the
   *  shipped UI — unlocking is one-way. */
  instanceSettingsUnlock: "instance-settings-unlock",
  instanceSettingsUnpair: "instance-settings-unpair",
  instanceSettingsChangeVersion: "instance-settings-change-version",
  /** Opens the instance page's overflow menu (`Library/Instance/index.tsx`'s
   *  `menuItems()`) — the reinstall entry below only resolves once this has
   *  been clicked. On the `@gd/ui` `Button` (`as="div"`) nested inside
   *  `DropdownMenuTrigger`, not the trigger itself — `Button` is already
   *  confirmed elsewhere in this file to spread unknown props onto whatever
   *  element `as` renders (hazard 1 does not apply to it). */
  instanceMenuTrigger: "instance-menu-trigger",
  /** Reinstall lives in the instance page's overflow menu
   *  (`Library/Instance/index.tsx`'s `menuItems()`), NOT in the Settings tab
   *  with the three above — hence the different prefix. The menu must be
   *  opened (`instanceMenuTrigger`) before this resolves. */
  instanceMenuReinstall: "instance-menu-reinstall",
  confirmReinstallConfirm: "confirm-reinstall-confirm",
  // The confirm control on `Confirmation`
  // (`ModalsManager/modals/Confirmation`), which `ModalsManager/index.tsx`
  // registers under both `unlock_confirmation` and `unpair_confirmation` for
  // the same component. Only the unpair path is reachable today —
  // `Instance/Tabs/Settings/index.tsx`'s unlock button mutates directly and
  // has its own `openModal("unlock_confirmation", ...)` call commented out —
  // so this id is named for the one flow it actually gates. Re-enabling the
  // unlock modal would put both flows' confirm buttons behind this same id.
  confirmUnpairConfirm: "confirm-unpair-confirm",
  /** The Addons tab's "Add Addons" button, which `Tabs/Addons/index.tsx`
   *  disables while the instance's modpack is locked. */
  addonsAddButton: "addons-add-button",

  worldDeletionConfirm: "world-deletion-confirm",
  worldDeletionDontAsk: "world-deletion-dont-ask",
  settingsWorldDeletionWarning: "settings-world-deletion-warning",

  /** `AddonTypeDropdown`'s trigger (`components/AddonTypeDropdown.tsx`) — the
   *  search page's content-type filter, e.g. mods vs. shaders vs. resource
   *  packs. `DropdownMenuTrigger` is Kobalte's own `Menu.Trigger`, confirmed
   *  by reading `menu-trigger.tsx` to spread unrecognised props onto its
   *  polymorphic `Button.Root` render, so this anchor reaches a real
   *  clickable element directly — no hazard-1 wrinkle here, unlike
   *  `Checkbox`/`Radio`/`Switch`. Click this, then query
   *  `addonTypeDropdownOption` below for the option to select.
   *
   *  Lives inside `EnhancedSearchBar`'s `OnboardingTip`-wrapped div (same
   *  wrapper `searchInput` sits in), so a real `.click()` on it can trigger
   *  the one-shot `search-input-syntax` tip — see `helpers/mods.ts`'s
   *  `dismissSearchOnboardingTip` for the guard `searchForMod` applies right
   *  after using this. */
  addonTypeDropdownTrigger: "addon-type-dropdown-trigger",
  /** One option in that dropdown's listbox, carrying `data-addon-type` — the
   *  option's own `FEUnifiedSearchType` value (e.g. `"resourcePack"`,
   *  `"shader"`, `"datapack"`, `"world"`), not the plural `AddonType`
   *  addon-type string `helpers/mods.ts`'s `InstalledMod.addonType` reads off
   *  an installed row (`ADDON_FIXTURES`'s own `searchType` field doc comment
   *  in `helpers/addonFixtures.ts` notes the same singular/plural split).
   *  `DropdownMenuContent` renders through `DropdownMenuPrimitive.Portal`, so
   *  this option is not itself a descendant of the `OnboardingTip`-wrapped
   *  div the trigger above lives in — only the trigger click carries that
   *  hazard. The dropdown must be open first — use `byAddonTypeOption`. */
  addonTypeDropdownOption: "addon-type-dropdown-option",

  /** The "Continue anyway" control on `ShaderLoaderSetup`'s intro step
   *  (`managers/ModalsManager/modals/ShaderLoaderSetup/index.tsx`) — one of
   *  three choices (Cancel / Continue anyway / Auto setup) a shader install
   *  is routed through instead of installing directly whenever
   *  `instance.checkShaderRequirements` reports anything other than
   *  `"LoaderPresent"` (`ModDownloadButton`'s `maybeOpenShaderWizard`,
   *  `hooks/useModInstallation.ts`). Only this one choice carries an
   *  anchor — `helpers/mods.ts`'s `installModIntoInstance` clicks it to
   *  install just the shader file, the closest match to what every other
   *  addon type's single "Download" click already does; the other two
   *  (cancel entirely, or auto-install a recommended loader alongside it)
   *  are a different feature this suite does not otherwise exercise, so
   *  they carry no anchor per this file's "only what a test must click"
   *  rule. On a `@gd/ui` `Button`, confirmed elsewhere in this file to
   *  spread unknown props onto the real `<button>` — no hazard-1 wrinkle. */
  shaderLoaderContinueAnyway: "shader-loader-continue-anyway"
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

/**
 * A version row on an addon page's Versions tab, located by the platform's
 * own file/version id (a CurseForge file id or Modrinth version id, both
 * carried as `fileId` throughout the download-button plumbing — see
 * `TEST_IDS.addonVersionRow`'s doc comment). Scope a query to this locator's
 * one descendant `<button>` (rendered by `ModDownloadButton`/`InstallButton`)
 * to click that specific build's install control.
 */
export function byAddonVersionRow(fileId: string): string {
  return `[data-testid="${TEST_IDS.addonVersionRow}"][data-file-id="${fileId}"]`
}

/**
 * One option in the change-version modal's listbox, located by the
 * platform's own version id (a Modrinth version id or a CurseForge file id).
 * The listbox must be open — click `TEST_IDS.modpackVersionSelect` first.
 */
export function byModpackVersionOption(versionId: string): string {
  return `[data-testid="${TEST_IDS.modpackVersionOption}"][data-version-id="${versionId}"]`
}

/**
 * One option in `AddonTypeDropdown`'s listbox, located by its own
 * `FEUnifiedSearchType` value (see `TEST_IDS.addonTypeDropdownOption`'s doc
 * comment for why that, not the plural `AddonType`, is the key). The
 * dropdown must already be open — click `TEST_IDS.addonTypeDropdownTrigger`
 * first.
 */
export function byAddonTypeOption(searchType: string): string {
  return `[data-testid="${TEST_IDS.addonTypeDropdownOption}"][data-addon-type="${searchType}"]`
}
