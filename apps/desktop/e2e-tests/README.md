# E2E tests

Playwright drives the **packaged** launcher, so a build must exist before the
suite can run.

```bash
pnpm build:linux-x64-e2e      # or :win-x64-e2e / :mac-universal-e2e
pnpm test:e2e
```

The `-e2e` build variants add the `e2e` cargo feature, which is what makes
`--gdl_e2e_auth_base` and `--gdl_e2e_entitlement_key` do anything. A normal
build ignores both flags, and no published artifact carries the feature.

## What the mock replaces

`mock-idp/` implements the whole chain enrollment walks — Microsoft device
code, Xbox Live, XSTS, and Minecraft services — so no test ever contacts a real
identity provider. It also answers `POST /v1/auth/token` on the GDL side,
because enderium validates the Microsoft `id_token` against Microsoft's live
JWKS and would always reject a locally minted one.

The entitlement response is signed with an RSA keypair generated fresh at
startup; the public half is written to the scratch runtime directory and passed
to the launcher, which is why `McEntitlement`'s offline signature check passes.

## Two modes

**proxy** — selected when both `TEST_BASE_API` and `E2E_INTERNAL_AUTH_TOKEN`
are set. A test user is provisioned from enderium, and every GDL call except
the token exchange is forwarded to the real backend. This is what CI runs.

**standalone** — the default without those variables. The GDL token is minted
locally and a small set of `/v1/*` routes are stubbed. Anything unstubbed
answers `501` naming the route. Tests needing real backend behaviour skip with
a stated reason.

## Adding a test

```ts
import { expect, test } from "./fixtures/index.js"

test("something", async ({ authenticatedApp }) => {
  const { page } = authenticatedApp
  // page is on /library
})
```

`authenticatedApp` is worker-scoped: one login is shared by every test the
worker runs, so return to `/library` in `afterEach` and do not assume a
pristine app. Use `freshApp` when a test needs untouched state — it pays for
its own launch.

New anchors go in `helpers/selectors.ts`, never inline in a spec.

## Install verification

`helpers/installVerify.ts` checks a completed install on disk, independent of
anything the app's own UI reports. Both `instanceInstall.spec.ts` and
`loaderInstall.spec.ts` call it after every install in the matrix.

### What it checks, and what it deliberately does not

- **Client jar**: existence, plus a full SHA-1 compare against
  `downloads.client.sha1` off the cached version JSON. Every install checks
  the whole jar, not a sample.
- **Every asset object** the index lists: existence-checked in full, no
  exceptions. Content is hash-checked on a **deterministic sample of 20
  objects per index** (`sampleKeys`, `HASH_SAMPLE_SIZE` in
  `installVerify.ts`), not all of them — hashing every object would dominate
  the cost of a test whose point is the install, not this check. The sample
  is chosen by sorting the object names and striding across the sorted list,
  so it depends only on the index's own key set, never on wall-clock time or
  a PRNG — two runs against the same index sample exactly the same objects.
  **Say this plainly so nobody assumes full hashing: a corrupted object
  outside the sampled 20 is not caught.**
- **Processor-generated libraries** (see the loader suite's processor
  assertion below): existence only. There is no per-file expected hash in
  the loader JSON's `processors`/`data` shape to check most of them against.

### The asset layout split

Legacy (`assets/virtual/<indexId>/<name>`) vs. modern
(`assets/objects/<hash prefix>/<hash>`) is never guessed from the version id.
`verifyAssetIndex` reads the asset index JSON's own `"virtual"` boolean and
branches on it — the same branch
`crates/carbon_app/src/managers/minecraft/assets.rs`'s `get_assets_dir` and
`reconstruct_assets` take, keyed off `AssetsIndex::map_virtual`
(`#[serde(rename = "virtual")]` in daedalus).

`1.6.4` is pinned in `versionMatrix.ts` specifically so a real install
exercises the virtual branch: its asset index id is `legacy` and its JSON
sets `"virtual": true`, confirmed against live Mojang data and by
instrumenting the production verifier live against a real install (see
task-2-report.md). **`1.7.10` is not the legacy case**, despite being the
version commonly assumed to be old-format — its asset index (id `"1.7.10"`)
carries no `"virtual"` key at all and resolves through the same modern,
content-addressed path as `1.12.2`, `1.16.5`, and `1.20.1`. Only the `1.6.x`
release line (index id `legacy`) is virtual.

## Instance install suite

`instanceInstall.spec.ts` installs a matrix of real Minecraft versions end to
end — create the instance, wait for the install to finish, assert it's ready
to play — through the actual UI, against the actual Mojang CDN.

### The version matrix

`globalSetup.ts` resolves the matrix once per run, before any worker starts,
and hands it to every worker through `E2E_VERSION_MATRIX` (see
`playwright.config.ts`'s `globalSetup`). It has two sources:

- **Pinned** (`versionMatrix.ts`'s `PINNED_VERSIONS`): a fixed set of
  versions, each chosen to straddle a real format boundary — legacy vs.
  modern asset index, pre/post-1.13 flattening, LWJGL 2→3 — plus the
  manifest's own current newest release, resolved fresh at run time so this
  never goes stale as Mojang ships new versions.
- **Random**: a seeded draw of additional releases from the rest of the
  manifest, so coverage of the wider version surface accumulates over time
  without every run paying for the whole catalog.

Two env vars control the draw:

- `E2E_VERSION_SEED` — the mulberry32 seed. Unset picks a fresh one from the
  clock and prints it; set it to reproduce a specific draw.
- `E2E_VERSION_RANDOM_COUNT` — how many random versions to draw on top of the
  pinned set. Defaults to 2.

Every run prints the resolved matrix and its seed up front:

```
  e2e version matrix
  seed: 469278827   (replay with E2E_VERSION_SEED=469278827)
    - 26.2 (pinned)
    - 1.20.1 (pinned)
    - 1.16.5 (pinned)
    - 1.12.2 (pinned)
    - 1.7.10 (pinned)
    - 1.6.4 (pinned)
    - 1.8.5 (random)
    - 1.10.2 (random)
```

To replay a failed run exactly, take the seed from that banner (or from the
failing test's title — it's appended there too) and re-run with
`E2E_VERSION_SEED=<seed>`.

**A random-draw failure is not necessarily caused by the commit under test.**
The random half of the matrix exists specifically to surface pre-existing
breakage on versions nobody has exercised recently — that is its job, not a
malfunction. Treat a red random entry as a real bug report on that version
and go fix or file it. Do not silence it by re-rolling the seed until it
passes; that defeats the reason the random draw exists at all.

### Inspecting the matrix without running it

`playwright test --list` does not work while `instanceInstall.spec.ts` is
present in the test dir: Playwright 1.58.2 skips `globalSetup` in list mode,
so `E2E_VERSION_MATRIX` is never set, and the spec's module-scope guard
throws at collection time. This is deliberate — falling back to a
pinned-only matrix because the real one failed to resolve would silently
under-test the run, which is a worse failure than a loud one.

To see what a given seed resolves to without paying for a run, invoke
`globalSetup.ts` directly:

```bash
pnpm exec tsx e2e-tests/globalSetup.ts
# or, to inspect a specific seed:
E2E_VERSION_SEED=469278827 pnpm exec tsx e2e-tests/globalSetup.ts
```

This prints the same banner shown above and exits. It works because the file
carries a small `import.meta`-guarded self-invocation that only fires when
the file is the process entry point — Playwright imports the module and
calls the exported `default` itself, so this is dead weight on that path and
never runs twice.

### Timeouts

- **15 minutes** is the hard per-test ceiling (`playwright.config.ts`'s
  `timeout`).
- **11 minutes** is `waitForInstallComplete`'s install bound
  (`helpers/instances.ts`), on top of its own 90s start bound — comfortably
  under the 15-minute ceiling, with room left for the creation-modal
  interactions, the post-install assertions, and cleanup, so a genuinely
  stuck install throws its own diagnosable message instead of the whole
  budget running out first and Playwright discarding it with no explanation.
- **60 seconds** is the global action timeout (`playwright.config.ts`'s
  `actionTimeout`), so a single missing anchor fails in a minute instead of
  hanging for the rest of the test's 15-minute budget.

### Why later installs in the same run are fast

`authenticatedApp` is worker-scoped (see `fixtures/index.ts`), so every
matrix entry in a worker shares one launched app and one runtime path by
design — this is not incidental, and `deleteInstanceViaUi` only ever removes
the instance's own folder, never `assets/`, `libraries/`, or
`managed_javas/`. That's what makes the timings look the way they do: at
seed `469278827`, all 8 versions installed in 1.4 minutes total. The first
(cold) install took ~22s and left ~830 MB on disk; every install after it
took 5–7s, including `1.6.4` — its 1120-object legacy index is smaller than
the newer ones, so it added no measurable overhead over its neighbours.
Minecraft's assets are content-addressed, and consecutive
versions in the matrix share roughly 93% of their asset objects; libraries
overlap heavily within an era; and managed JREs are cached per Java major
version rather than per instance. None of that reuse is a test artifact —
it's the same cache a real user's launcher builds up, which is exactly why
the suite is worth running against it instead of a mocked download path.

### The suite never launches the game

Every matrix test stops at "installed and shows as ready to play" and
deliberately never clicks play. The harness's accounts carry a mock
entitlement (see the top-level `## What the mock replaces` section above),
and real Minecraft rejects it — a launch would be asserting on a failure
rather than a success. This is a structural limit of the mock-IdP approach,
not a gap to "fix" by clicking the button anyway; a real launch needs an
actually-entitled Microsoft account from CI secrets as a separate harness
mode (see the plan's Out of Scope notes).

### Diagnosing a failed install

A screenshot of the library grid rarely explains why an 11-minute install
failed — the answer is almost always in the Rust core's own log, not the
DOM. `fixtures/electronApp.ts`'s `attachCoreLogOnFailure` reads the core's
session log from `<runtimePath>/__gdl_logs__` and attaches it to the
Playwright report (`testInfo.attach`) whenever a test's outcome doesn't match
what it expected; `instanceInstall.spec.ts` wires this in through
`test.afterEach`, since `authenticatedApp` is worker-scoped and never
receives a per-test result to gate on itself. Open the attachment from
`playwright-report` — it's the actual log the core process wrote during that
test's run, not a copy inferred after the fact.

(`__gdl_logs__` is only ever written to by release builds — see
`crates/carbon_app/src/logger.rs`'s `setup_logger` — which is what the `-e2e`
build variants are. A local debug build attaches nothing, by design, not by
bug.)

## Loader install suite

`loaderInstall.spec.ts` installs a matrix of five loader/Minecraft
combinations end to end, the same way `instanceInstall.spec.ts` does for
vanilla — through the actual UI, against real loader builds resolved live
from meta.gdl.gg.

### The matrix

| Loader | Minecraft version | Why it's in the matrix |
|---|---|---|
| Forge | 1.20.1 (pinned) | Runs install processors — the case the processor-artifact assertion below exists for. |
| Forge | 1.12.2 (pinned) | Pre-flattening Forge: its loader JSON genuinely declares zero processors (`"processors":[],"data":{}`), asserted as exactly zero rather than skipped. |
| NeoForge | newest supported (resolved live off the app's own manifest) | The actively developed fork; runs processors like modern Forge. |
| Fabric | 1.20.1 (pinned) | No install processors at all — `PartialVersionInfoCache` is never populated for it, a structurally different case from 1.12.2's confirmed zero. |
| Quilt | 1.20.1 (pinned) | Same as Fabric. |

Five combinations, one test per row. Client jar and asset-index verification
(see "Install verification" above) run for all five; the processor-artifact
assertion only runs for the two Forge/NeoForge entries that populate a
`PartialVersionInfoCache` row.

### Why loader versions come from the app's own dropdown, not an external manifest

The vanilla matrix (`globalSetup.ts`) deliberately draws from Mojang's
`version_manifest_v2.json`, while `createInstanceViaUi`'s Minecraft-version
dropdown is fed by `mc.getMinecraftVersions`, which the core resolves from
GDL's own meta.gdl.gg — two independent sources that can genuinely diverge
(most plausibly right after a Mojang release, before daedalus has ingested
it). A mismatch there is a real, reportable condition, not a harness bug.

There is no equivalent second source for loader builds: meta.gdl.gg is the
only place loader version lists come from, for the app and for anyone else.
Repeating the vanilla matrix's Mojang-vs-meta cross-check for loaders would
mean comparing meta.gdl.gg against itself, so there is no reason to. Instead
`loaderInstall.spec.ts` reads whatever the app's own loader-version dropdown
currently offers — `fetchLoaderManifest` captures the exact
`mc.get<Loader>Versions` response the dropdown's own click triggers — and
picks deterministically from that with `pickSeededOption` /
`deriveLoaderVersionSeed`, rather than fetching a manifest a second time from
somewhere else.

### The processor-artifact assertion, and why it exists

Forge and NeoForge installs run client-side install processors that generate
patched/SRG jars into `libraries/` at maven paths computed from the loader
build's own JSON (`processors`/`data`). These generated files are exactly
what a Minecraft-cache clear wipes without regenerating on a normal launch —
the resulting failure surfaces as "minecraft dependency missing" at launch,
not at install time. `loaderInstall.spec.ts` reads the app's own cached
loader JSON (`PartialVersionInfoCache`) after install, derives the required
artifact set via `helpers/processorOutputs.ts`, and asserts every one exists
on disk via `verifyLibrariesPresent`.

The assertion states the expected count explicitly rather than guarding with
a bare `if (required.length > 0)` — that shape would make "derived zero
because nothing needed generating" and "derived zero because a port bug or
install-profile regression broke derivation" indistinguishable, silently
degrading the suite's highest-value assertion to a log line. Each matrix
entry declares `expectsProcessorArtifacts`: `true` for Forge 1.20.1 and
NeoForge (`toBeGreaterThan(0)`), `false` for Forge 1.12.2 — asserted as
**exactly zero** (`toBe(0)`), a confirmed-correct property of pre-flattening
Forge's loader JSON, not a silently skipped check. Fabric and Quilt never
populate `PartialVersionInfoCache.processors`, so this block does not run for
them at all.

### The golden-file cross-check

`helpers/processorOutputs.ts` is a line-for-line TypeScript port of
`crates/carbon_app/src/managers/minecraft/processor_outputs.rs`'s
`required_files` — there is no Rust binding into the Playwright process, so
the e2e suite carries its own copy of that logic to know what a given
install *should* have produced.

Two independent implementations of the same logic can silently drift apart.
`processor_outputs.rs`'s `required_files_matches_committed_golden` test
computes the real Rust function's output for a committed input fixture and
compares it against a committed golden file
(`crates/carbon_app/fixtures/processor_outputs_golden/{input,output}.json`);
`helpers/processorOutputsGolden.test.ts` reads those exact same two files —
not a TypeScript-local copy — and asserts the TypeScript port produces the
same output. A `required_files` behavior change either regenerates the
golden (and the TypeScript test goes red until the port is updated to match)
or the Rust test fails first because nobody regenerated it — either way,
something is told.

To regenerate the golden after a deliberate `required_files` change:

```bash
set -a && . ./.env && set +a
UPDATE_GOLDEN_PROCESSOR_OUTPUTS=1 cargo test -p carbon_app --features e2e required_files_matches_committed_golden
```

Review the diff to `output.json` like any other reviewed change before
committing it.

**What this does not cover**: only `required_files`'s own input→output
mapping, over the one committed fixture. It says nothing about
`libraries_path` resolution (the function takes that as a parameter; the
golden fixture doesn't exercise different values of it) or
`PartialVersionInfoCache` keying (how the cache row this function's input
comes from gets looked up in the first place) — both live outside
`required_files`'s own signature, so a regression there would not turn this
cross-check red.

## Mod tests

Two spec files exercise mod management end to end:
`modInstall.spec.ts` (install from each platform) and
`modLifecycle.spec.ts` (disable, enable, delete, update on an already-installed
mod). Both share the `installedInstance` fixture below and both verify on
disk, never by trusting the app's own UI rendering of the same fact.

### The warm `installedInstance` fixture

`fixtures/installedInstance.ts` is worker-scoped (`{ scope: "worker" }`, same
mechanism as `authenticatedApp`): it installs one Fabric 1.20.1 instance
(`gdl-e2e-mods-fabric`) once per worker, on top of the worker's own
`authenticatedApp`, and every mod test in that worker reuses it rather than
paying for its own create-and-install. Fabric is the loader chosen for this
because it installs faster than every other loader (~8s once the
assets/libraries/JRE substrate is warm) and every mod this suite targets
supports it; 1.20.1 reuses the same pinned combination
`loaderInstall.spec.ts`'s own matrix already exercises.

The fixture hands out `instanceName` and `modsDir` — the instance's `mods/`
directory path, resolved from its real on-disk shortpath
(`fixtures/installedInstance.ts`'s `resolveModsDir`) rather than assumed
equal to the instance's display name. The non-obvious part, worth stating
plainly because it is easy to get wrong: **`instance/mods` does not exist
when the fixture hands this path out.** Nothing in the instance-install path
creates it — unlike the server-side counterpart
(`server/modloader_install.rs`, which eagerly `create_dir_all`s its own mods
path), the instance side only creates `mods/` lazily, the first time a
CurseForge/Modrinth install actually writes a file there
(`managers/instance/installer/mod.rs`'s `get_install_path`). The fixture
deliberately does **not** `mkdir` it: doing so would mutate the state the app
under test operates on, and would turn a real regression — mod installation
breaking such that `mods/` never gets created — into an invisible one, every
downstream test finding a directory that happens to exist and reporting only
"mod file missing" rather than "mods directory does not exist". Callers that
need the directory to exist (to list its contents, say) install a mod first
or handle the absence themselves; `helpers/modVerify.ts`'s functions already
treat a missing `mods/` as a reported problem rather than a thrown `ENOENT`
or a vacuous pass.

### What mod verification checks

`helpers/modVerify.ts` is pure Node — no Playwright, no DOM — so it can be
unit-tested directly and reused from any spec. It checks, independent of
anything the app itself reports:

- **Presence** (`verifyModInstalled`) — the named file exists somewhere
  under `modsDir`, enabled or disabled.
- **Size and sha1**, both optional and both checked exactly (no sampling,
  unlike the asset-index hash sampling above — a mod jar is a few MB, not
  worth trading precision for speed) against whichever variant is found.
- **Enabled state** (`verifyModEnabled`) — whether the enabled or disabled
  variant is the one present.

A disabled mod is not a flag or a sidecar file: it is the same file renamed
in place with a literal `.disabled` suffix appended to the full filename,
living in the same folder an enabled mod would
(`ManagerRef<InstanceManager>::enable_mod`,
`crates/carbon_app/src/managers/instance/mods.rs:300`, the suffix built at
:335-337 and the two `tokio::fs::rename` calls at :348 and :358). Both
`verifyModInstalled` and `verifyModEnabled` check both paths and treat
**both present at once as a failure**, not a state where one variant wins —
`enable_mod` refuses to rename onto an existing destination, so this should
never happen, and reporting it as its own distinct problem is what makes
"both variants present" a detectable bug state instead of a silently
tolerated one.

### Install suite: which mods, and why

`modInstall.spec.ts` installs one real mod from each platform:

- **Modrinth: Fabric API** (`P7dR8mSH`) — the load-bearing dependency of the
  Fabric mod ecosystem, maintained by FabricMC itself. Virtually no Fabric
  mod ships without it, which is what makes it a durable choice: it is
  vanishingly unlikely to disappear or stop publishing 1.20.1 builds.
- **CurseForge: Sodium** (`394468`) — CaffeineMC's rendering-optimization
  mod, one of the most widely installed Fabric mods that exists, mirrored to
  CurseForge from its Modrinth/GitHub origin. Deliberately a **different**
  mod than the Modrinth case installs: if both tests installed the same
  project, a bug that only ever satisfies one platform's install path could
  still leave a leftover file from the other test's run and make the
  assertion pass for the wrong reason. Two distinct mods mean neither test's
  disk assertion can be satisfied by the other's leftovers.

Filenames, sizes, and sha1s are never hardcoded. Both tests read the
just-installed mod's `filename`/`file_size`/sha1 back off the app's own
`instance.getInstanceMods` response (`helpers/mods.ts`'s
`openInstanceAddons`, called fresh after the install completes) and match by
exact platform project id — never "the first" or "the only" row. This is the
point of the test, not an implementation detail: a CDN regression that
changes what gets served changes what this list reports too, not just a
hand-built download URL that the test would otherwise still pass against.

**These two tests are the regression guard for the 2026-07-19 CurseForge CDN
key incident** (`edge.forgecdn.net` starting to require an `x-api-key`,
breaking every shipped client). Nothing else in this suite downloads a mod
file — `instanceInstall.spec.ts` and `loaderInstall.spec.ts` install Minecraft
itself and loader builds, never a mod — so before `modInstall.spec.ts`
existed, that entire code path had no coverage at all.

### Lifecycle suite: disable, enable, delete, update

`modLifecycle.spec.ts` drives Fabric API on Modrinth for all four tests.
Disable/enable/delete are deliberately not platform-specific:
`enable_mod`/`delete_mod` operate purely on the cached DB row and its
on-disk filename, with no branch on CurseForge vs. Modrinth, so one
well-understood project is enough to exercise the mechanism without paying
for a second platform's search-and-install round trip in every test.

The update test is the awkward one. A real update needs a file genuinely
older than whatever the platform currently offers, so it installs one
specific, deliberately-not-newest build through the addon page's **Versions
tab** rather than asserting anything weaker. Each row there gives
`ModDownloadButton` a `fileId`, which routes the click through
`instance.installMod` (`INSTALL_MOD`, a specific version id) rather than
`instance.installLatestMod` (`INSTALL_LATEST_MOD`) — the mechanism that
makes installing something other than latest possible at all
(`helpers/mods.ts`'s `installAddonVersion` and `openAddonVersions`).
`pickOlderVersion` sorts the returned list by its own `datePublished` field
(never trusting API order) and picks the second-newest, so exactly one
already-confirmed-newer build exists for the later update to move to.

**Limitation, stated honestly rather than papered over**: the Versions tab's
list is virtualized (`@tanstack/solid-virtual`), so only rows near the
viewport are mounted in the DOM. `installAddonVersion` asserts the target
row is actually visible before doing anything else, with a message that
names this exact risk, but there is no scrolling fallback — a version that
sorts outside the initial mount-plus-overscan window will fail there. This
works today because every version list this suite reaches is small (~16–27
rows once scoped to the instance's own Minecraft version and loader) and
mounts without scrolling; reaching further back into a project's history
would need a scrolling helper that does not exist yet.

**The CurseForge equivalent of this test is deliberately not implemented.**
`openAddonVersions`'s CurseForge branch (`modplatforms.curseforge.getModFiles`)
was written symmetrically alongside the Modrinth path from source, but never
once driven live by anything in this suite. On review it was judged
plausibly wrong rather than merely untested: CurseForge's `getModFiles` is
actually paginated (`ModFilesParametersQuery`'s `index`/`pageSize`), unlike
Modrinth's single-response `getProjectVersions`, so the dual-request
settle-window race `openAddonVersions` uses — tuned specifically against
Modrinth's own two-fetch timing — is not a safe assumption to carry over. The
branch was deleted rather than kept as unexercised, symmetric-looking
coverage; `openAddonVersions` is Modrinth-only today, with no platform
parameter. A future CurseForge update test needs that branch written fresh
against CurseForge's real paginated behavior, confirmed live.

### Known product bug: `enable_mod`/`disable_mod` is not transactional

Found incidentally while building this suite, not fixed here — flagged so it
is not rediscovered from scratch.

`enable_mod` (`crates/carbon_app/src/managers/instance/mods.rs:300`) commits
the on-disk rename (`tokio::fs::rename` at :348 for enabling, :358 for
disabling) **before** the `ModFileCache.enabled` database write
(`mfcdb::update_mod_file_enabled`, :361-367). A crash in that window leaves
disk and DB disagreeing: the mod is genuinely excluded from the game (its
file is renamed to `<filename>.disabled` on disk), while `list_mods` — which
reads `ModFileCache.enabled` — still reports it enabled in the UI. Clicking
disable again does not fix it in the moment: `enable_mod`'s own preconditions
check disk (`disabled_path.exists()` at :350-352), so it hits `bail!("mod is
already disabled")` and refuses.

**This is not permanent.** `cache_local`'s per-instance disk scan
(`managers/metadata/cache/mod.rs`) compares every cached row against disk,
including the enabled flag: the skip condition at :1723-1731 is `*real_size
== entry.filesize && *enabled == entry.enabled && *addon_type ==
entry.addon_type`, where `enabled` comes from the disk scan itself (derived
from the `.disabled` suffix, not the DB — the same source `modVerify.ts`
reads). A mismatch fails that condition, so the entry is left in `modpaths`
rather than skipped, and gets re-upserted at :1790-1798
(`cache_mod_file_unchecked` → `upsert_mod_file_cache`, :1362-1372) with the
disk-derived value, overwriting the stale DB row. This scan is queued for
every instance at startup (`managers/instance/mod.rs:272-278`) and
prioritised whenever the frontend opens an instance
(`PRIORITIZE_INSTANCE_CACHE`, `api/instance/mod.rs:276-283`). So the
disagreement is real but **transient**, and self-heals on the next local scan
of that instance — in practice, the next time the user opens it. The
user-visible symptom is a briefly-wrong toggle and one failed disable click,
not a permanently wedged mod.

`modLifecycle.spec.ts`'s disable/enable/delete tests read disk only (never
the DB-backed `enabled` field the UI's switch reflects) specifically because
of the non-atomic window above — see the spec file's own module doc comment.
No test in this suite can be made flaky by it, either: `enable_mod` performs
the rename and the DB write sequentially inside one handler before
responding, and `toggleModEnabled` (`helpers/mods.ts`) awaits that response
before returning, so there is no window in which a test's own assertion could
observe the stale midpoint.

## Suite wall-clock

Measured on this branch at `workers: 1` — what CI actually runs:
`playwright.config.ts` sets `workers: process.env.CI ? 1 : undefined`, and
GitHub Actions sets `CI=true` on every job, so every CI run is single-worker
regardless of local defaults.

- Full e2e suite (`init`, `login`, `instanceInstall`'s 8-entry vanilla
  matrix, `loaderInstall`'s 5-entry loader matrix, `modInstall`'s 2 tests,
  `modLifecycle`'s 4 tests — **25 tests total**): **187–200s (3.1–3.3
  minutes)** across repeated full runs at `CI=true`.
- Unit suite (`pnpm test:unit`, 145 tests across 16 files): **~2–3s**.
- Combined: **roughly 3.1–3.3 minutes** of test time per OS.

`.github/workflows/all_os.yml` runs this on three OS jobs (`ubuntu-22.04`,
`windows-2022`, `macos-14`) **in parallel**, each with its own 80-minute job
timeout, each forcing `workers: 1` the same way. Only Linux was directly
measured for this document; Windows and macOS are expected to land in the
same order of magnitude — the suite is network- and install-bound against
the same seeded matrix and the same live mod platforms, not CPU-bound — but
that is an expectation, not a second measurement.

The arithmetic that matters for a PR: because the three OS jobs run in
parallel rather than in series, the wall-clock this suite adds to a PR's
critical path is **one** OS's ~3.1–3.3 minutes, not three times it. The 3×
only shows up as total CI compute — three runners each spending ~3.1–3.3
minutes on the test step, on top of their own build and lint steps — roughly
9.4–10 minutes of test-step compute across the three jobs combined.

**At ~3.1–3.3 minutes against an 80-minute per-job timeout, the suite
comfortably fits a per-PR run today**, the mod suites' added ~40–50s
included. That is a statement about the current measured duration, not a
recommendation — whether to split PR vs. nightly runs as the matrix grows is
a call for a human, not this document.

### Third-party flakiness is observed, not theoretical

One full-suite run on this branch hit a real failure: `modInstall.spec.ts`'s
"installs Sodium from CurseForge" failed on `addon-install-button` never
becoming visible, in code that round's changes never touched and that had
been green in every prior run that day. A re-run of the full suite
immediately after came back clean (25/25), which points at a live-network
flake in CurseForge's search rather than a regression.

With `retries: 0` (`playwright.config.ts`), a flake like that is a genuine
red build, not a retried-away yellow one. That is a deliberate trade-off,
not an oversight: this suite installs real mods from real, live third-party
platforms specifically so a break like the CDN key incident shows up, and
that same design accepts that a third-party outage or transient failure
occasionally shows up too, indistinguishable from a real regression until
someone re-runs it. State this plainly rather than discover it by surprise:
an occasional red CI run on this suite that a re-run clears is expected
behavior, not evidence the suite itself is broken.

## Troubleshooting

- **Login hangs on "Continue" with a red toast covering the button** — the
  packaged build's `app-update.yml` points its update check at
  `http://localhost:9000/` by default. Anything already listening there (a
  local ClickHouse container is one real example this has hit) answers with
  a response `electron-updater` treats as an error, which surfaces as a
  `toast.error` that can land on top of the welcome screen's continue button
  and block the click (`packages/mainWindow/src/utils/updater.tsx`'s
  `case "error"`). Fix locally by editing the **build artifact** (not
  source — it's gitignored and regenerated by the next `pnpm build:*-e2e`) at
  `release/<platform>-unpacked/resources/app-update.yml` (or
  `.../GDLauncher.app/Contents/Resources/app-update.yml` on macOS) and
  pointing `url:` at an address that won't answer at all (e.g.
  `http://192.0.2.1:9999/`, a reserved TEST-NET address) so the check times
  out instead of erroring fast.
- **`apps/desktop/package.json`'s `"version"` changes from `0.0.0`** —
  `electron-builder` bumps it to the release version as an incidental
  packaging side effect of `pnpm build:*-e2e`. Hit repeatedly across separate
  tasks on this branch. Revert it (`git diff apps/desktop/package.json`)
  before staging anything else — it is not a real change and should never
  end up in a commit.

## Known gaps

- A run killed between provisioning and teardown leaves an orphan row on
  api-test. The deletion sweep only claims rows deleted over seven days ago, so
  it will not collect them.
- Skin fetches still reach `textures.minecraft.net` and will fail for the
  synthetic profile. Harmless — they only log.
- The browser-OAuth enrollment path is not covered; the suite uses device code.
