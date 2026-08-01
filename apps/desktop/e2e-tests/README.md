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
instrumenting the production verifier live against a real install. **`1.7.10` is
not the legacy case**, despite being the
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

**Reaching a version outside the initial viewport**: the Versions tab's list
is virtualized (`@tanstack/solid-virtual`), so only rows near the viewport
are mounted in the DOM. `installAddonVersion` calls `helpers/mods.ts`'s
`scrollVersionRowIntoView` first, which scrolls the virtualizer's own scroll
parent — found the same way `Versions/index.tsx` finds it, by walking up for
the first ancestor with `overflow(-y): auto|scroll` — a viewport at a time,
driving both the virtualizer and, once it bottoms out, the infinite query's
next page, until the target row mounts. Bounded, not unlimited: it gives up
after 40 scroll steps, or immediately once the list's real bottom is
confirmed (a scroll that doesn't move, plus one settle window for a further
page that never arrives) with the row still unfound — either way with a
message naming which of the two happened, how many rows were mounted, and
how far down the container it got, so a genuinely absent version fails
distinctly from one merely still out of reach. Reaching further back into a
project's history is now a matter of that 40-step bound, not of a missing
mechanism — every version list this suite reaches today (~16–27 rows once
scoped to the instance's own Minecraft version and loader) is well within
it.

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

### The mod resolution suite (`modResolution.spec.ts`)

Four tests prove the app resolves "which build to fetch" correctly along two
different paths — installing an addon with no version picked, and updating
an already-installed one — against **Cloth Config API**
(the full, live-verified rationale: Modrinth `9s6osm5g` / CurseForge `348521`,
`project_type: "mod"` so the loader filter actually applies, zero declared
dependencies at 1.20.1 on either loader so `helpers/mods.ts`'s documented
dependency-jar cleanup gap never bites, and — the property that matters most
for the update-path tests below — its 1.20.1 line is **entirely stable**, on
both loaders, with no beta/alpha noise to fight).

**Install path** (two tests, both loaders): "resolves the newest compatible
Modrinth build for each instance's loader" and its CurseForge counterpart.
Each drives the *same* project through **two** instances pinned to the same
Minecraft version but different loaders — `installedInstance` (Fabric) and
the `forgeInstance` fixture below (Forge) — and asserts the two resolve to
**different** version/file ids. That cross-instance differ is the assertion
that actually catches a dropped loader filter: if `install_latest_modrinth_mod`
(or its CurseForge counterpart) ever stopped filtering by loader, both
instances would resolve to whichever build sorts newest overall instead of
their own loader's newest, and this project is confirmed live to publish fully
disjoint per-loader ids at 1.20.1 — so a collision here is never expected
from the fixture itself.

**Update path** (two tests, Modrinth/Fabric only): "updates to the newest
compatible build and then reports no further update" and "install-latest and
update converge on the same build". Modrinth-only because reaching a
deliberately-older, still-installable build needs the addon page's Versions
tab, and that path is only live-verified against Modrinth — the same
constraint `modLifecycle.spec.ts`'s own update test already documents.

#### The `forgeInstance` fixture

`fixtures/forgeInstance.ts` mirrors `installedInstance.ts` almost exactly —
worker-scoped (`{ scope: "worker", timeout: 300_000 }`), composing the same
worker's `authenticatedApp` rather than a second login, and reusing
`installedInstance.ts`'s `resolveModsDir`. The one thing that makes it worth
a second fixture rather than a parameter on the first: it installs **Forge**
at the same Minecraft version (**1.20.1**) `installedInstance` already pins
Fabric to. That pairing is deliberate, not incidental — the whole install-path
suite exists to compare what two different loaders resolve for one project at
one Minecraft version, so the two fixtures have to agree on the version and
differ only in loader for the comparison to mean anything. Both fixtures'
`MC_VERSION`/pinned-version constants are intentionally duplicated rather than
shared — the same small, accepted duplication
`modResolution.spec.ts`'s own copy of the same constant follows.

#### The three-oracle separation, and why the scoped list may never feed a compatibility assertion

Every assertion in this suite answers one of three genuinely different
questions, each from its own independent source — mixing them up is the
single mistake this whole suite is built to make impossible:

1. **Ordering** — "which build is newest" (install path) or "which build
   should an update move to" (update path) — comes from `helpers/resolutionCapture.ts`'s
   `scoped` list: the app's own `getProjectVersions`/`getModFiles` response,
   already filtered server-side to the instance's Minecraft version and
   loader. `helpers/resolution.ts`'s `newestByDate`/`newestUpdateCandidate`
   read `scoped` exclusively.
2. **Compatibility** — "is the build the app actually installed genuinely
   compatible with this instance" — comes from `unfiltered`: for Modrinth, a
   direct, unauthenticated fetch of every version the project has ever
   published (`GET /v2/project/:id/version`), asserted against on both the
   Minecraft-version and loader axes; for CurseForge, a
   loader-unfiltered-but-still-game-version-scoped `getModFiles` request
   driven through the addon page's "Override Filters" control (CurseForge has
   no unauthenticated equivalent of Modrinth's direct-fetch escape hatch).
   Because that CurseForge oracle is itself fetched scoped to the instance's
   own Minecraft version, only the loader axis is actually asserted there —
   the CurseForge test has **no assertion capable of catching a broken
   CurseForge `gameVersion` filter** (a file it wrongly admitted would be
   admitted into this oracle's own request too, since both share the same
   parameter). That axis of coverage is Modrinth-only.
3. **Loader** — parsed from the **downloaded jar's own manifest**
   (`fabric.mod.json`/`mods.toml`/`quilt.mod.json`), never from either
   platform's API — the one check that would still catch a broken filter even
   if both API-side oracles above somehow agreed with it.

**Why the rule matters**: a `scoped` entry is compatible with the instance
*by construction* — the query that produced it was already filtered to this
Minecraft version and loader — so asserting compatibility against it checks a
tautology that can never fail. This is not a hypothetical risk: pointing the
compatibility check at `scoped` instead of `unfiltered`, as a deliberate
inverted proof, leaves the check **green** regardless — the direct demonstration
that this exact mistake is silent, not
loud, if it's ever made. Every compatibility assertion in this suite reads
`unfiltered` exactly once and `scoped` never; every ordering assertion reads
`scoped` exactly once and `unfiltered` never. Grep for `versions.scoped`/
`versions.unfiltered` in `modResolution.spec.ts` to confirm this holds before
trusting any change to this file.

The update-path tests add a fourth wrinkle on top of the same rule: install
and update resolve "latest" by genuinely different logic, so they need two
different ordering predicates, never one shared between them.
`install_latest_modrinth_mod`/`install_latest_curseforge_mod` take the first
(newest-by-date) entry of the filtered response with **no channel filter at
all** — modelled by `newestByDate`. `find_mod_update`
(`managers/instance/mods.rs:616`) sorts newest-first and walks the instance's
allowed channels (`AppConfiguration.mod_channels`) in list order, taking the
first candidate at or above each channel's level in turn — under the shipped
default (`'stable:true,beta:true,alpha:true'`) the **stable** pass runs
first, so this resolves to the newest *stable* build, not merely the newest
outright — modelled by `newestUpdateCandidate`. On Cloth Config's all-stable
1.20.1 line the two predicates happen to agree (there's no beta/alpha to
diverge on), which is exactly why this project was picked and exactly why
"install-latest and update converge on the same build" is expected to pass —
see the next section for what a divergence there would actually mean.

#### The channel-divergence hypothesis behind "install-latest and update converge"

The fourth test's whole point is that these two paths **could** disagree.
Because install applies no channel filter at all while update prefers the
newest stable first, a project whose newest compatible build happens to be a
beta would install that beta via the addon page's main button, and then have
`find_mod_update` offer an *older stable* build as the "update" — a downgrade
presented as an upgrade. Exactly one candidate project (JEI) was rejected
for this reason: its entire 1.20.1 line is beta-only, which would have made
this test fail by design rather than by regression. If this test ever goes
red on Cloth Config specifically, treat it as a product finding to triage
first — not a test to patch to tolerate the divergence — since Cloth Config's
1.20.1 line was independently confirmed stable-only before it was chosen.

#### The pagination completeness guard (CurseForge)

`getModFiles` is paginated; a truncated page silently produces a "newest of
what we happened to see" answer instead of "newest that exists" — worse than
no assertion at all, since it fails silently rather than loudly.
`resolutionCapture.ts`'s `toCandidatesFromCurseforgeResponse` checks the
response envelope's own `pagination.totalCount` against how many file
entries actually came back and throws, naming both numbers, if they disagree,
before any candidate is ever handed to an assertion.
`install_latest_curseforge_mod` itself requests `page_size: 200`, so a
project whose filtered file count exceeds that is a real limit of the shipped
app, not merely of this test — the guard's job is to make that limit loud
rather than let it silently corrupt an oracle.

#### Known gaps

- **Channel-preference resolution is untested.** `find_mod_update` walks
  `AppConfiguration.mod_channels` (default
  `'stable:true,beta:true,alpha:true'`, a real column with an rspc surface in
  `api/settings.rs`) to decide which channel to prefer, but nothing in the
  shipped frontend renders a control for it — no test in this suite (or the
  app itself, as far as this suite can tell) ever drives a non-default
  channel configuration through the update path. The stable-first walk is
  exercised only via the default, never via an instance configured to prefer
  beta or alpha first.
- **`find_mod_update`'s CurseForge/Modrinth candidate fetch is asymmetric on
  multi-loader instances.** For CurseForge, it filters by only the *first*
  modloader in the instance's configured list
  (`version.modloaders.iter().next()`, a single `mod_loader_type` value —
  CurseForge's API only accepts one). For Modrinth, it sends the instance's
  *entire* modloader list (`loaders: Vec<String>`). An instance that only
  ever has one modloader configured never exercises this difference, and
  neither does this suite — no test here drives a multi-loader instance
  through the update path, so this asymmetry is real (read directly off
  `managers/instance/mods.rs:616-716`) but structurally unverified.
- **Backend filter parameters are unobservable from the renderer.** Every
  request-scoping check in this file (`resolveForInstance`'s/
  `resolveForInstanceCurseforge`'s `scopedRequestUrl`) only proves the
  frontend's own `modplatforms.*` rspc call carried the instance's Minecraft
  version/loader — never that the Rust core's own outbound HTTP call to
  Modrinth/CurseForge was filtered the same way. That call happens entirely
  inside the core process and never passes through the renderer, so it is
  structurally invisible to a Playwright test. The jar-parsed `modloaders`
  check (loader) and the cross-instance differ assertion (ordering) are what
  actually stand between a silently dropped backend-side filter and a green
  suite — not the request-scoping check, which only catches the frontend
  half of that chain.

## Persistence tests

`persistence.spec.ts` proves that what the launcher writes survives a real
process restart — not a page reload, not a re-mounted component tree, but the
Rust core actually exiting and a fresh one reading the same runtime path back
off disk. Every other spec in this suite either never closes the app or
closes it only in teardown; this is the one file that would catch a bug that
wrote nothing to disk, wrote it somewhere the next boot can't find, or wrote
it in a form the startup reconciliation path corrupts.

### `relaunchApp`, and why waiting for a real core exit matters

`fixtures/electronApp.ts`'s `relaunchApp` closes the current
`ElectronApplication` and launches a fresh one against the same runtime path
— but only after confirming the Rust core process the old app owned has
genuinely exited, not merely that `app.close()` resolved.

Those are different events. `window-all-closed` in `main/index.ts` calls
`coreModule.kill()` and `app.quit()` without awaiting the core's own `exit`
event, so `app.close()` resolving only proves the *Electron* main process is
gone — the core (holding the SQLite connections everything this spec reads
back depends on) can still be alive. Relaunching before it has released those
handles races the old process's open file/DB handles against the new one:
depending on timing, the new core can find a lock it can't immediately
acquire. Left unhandled, that surfaces as a failure that looks exactly like
database corruption but has nothing to do with what this suite is actually
testing.

`relaunchApp` closes that gap by reading the outgoing core's pid off
`globalThis.__gdlCoreProcessId` (published by `main/index.ts` right after
`spawn()`) and polling for its real exit with `process.kill(pid, 0)` — Node's
cross-platform existence probe, which sends no signal and just asks the OS
whether the pid is still addressable. `ESRCH` is treated as "gone"; **every
other errno, including `EPERM`**, is treated as "still alive" and falls
through to the next poll, so a permissions quirk can never be misread as a
clean exit. The poll is bounded at 15s, comfortably above the Rust
termination handler's own ~3s-bounded graceful shutdown
(`crates/carbon_app/src/main.rs`, ~line 317-340), and throws a diagnosable
error naming the pid if the bound is ever hit rather than launching anyway.
SQLite's own `busy_timeout = 5000` (`crates/carbon_repos/src/db_exec.rs`) is
a second, independent layer underneath this, not a substitute for it: it
absorbs a brief residual overlap (documented for Windows, where
`TerminateProcess` is not a catchable signal so the graceful Rust shutdown
never runs), but it cannot absorb the case this function actually prevents —
relaunching while the old core is still fully alive and holding the database
open.

### What persistence coverage means here, and why both channels

Four things are written through four different code paths during setup, then
asserted **through two independent channels** after one real relaunch — the
app's own UI/API response, and the on-disk state directly:

1. **An instance** (name + Minecraft version) — UI: a fresh
   `instance.getInstanceDetails` response. Disk: both the `Instance` DB row
   *and* the instance's own on-disk `instance.json`
   (`helpers/instanceConfig.ts`) — two disk-side checks, deliberately, since
   this spec's whole point is SQLite survival specifically, not just "some
   file exists somewhere".
2. **An app setting** (`reducedMotion`, Settings > General's "Potato mode")
   — UI: the switch's own `checked` state. Disk: the
   `AppConfiguration.reducedMotion` column.
3. **An installed mod**, left enabled — UI: `instance.getInstanceMods`.
   Disk: `helpers/modVerify.ts`'s `verifyModInstalled`/`verifyModEnabled`
   against the real jar. See "Mod state in the database is a cache, not a
   persistence store" below for why this one is read honestly rather than
   taken as proof of anything SQLite-specific.
4. **A disabled mod** — same two channels, `enabled: false`. Genuinely
   persistence-adjacent in a way (3) is not: the `.disabled` filename suffix
   *is* the ground truth for this fact, not a proxy for something else, and
   there is no network fallback that can reconstruct it.

Checking only the UI would pass on stale in-memory state the running process
never actually persisted — a query cache that survived the restart in memory
would satisfy it for the wrong reason. Checking only disk would pass even if
the app never loaded what it wrote back on boot — a write-only bug, where the
next launch silently ignores its own data, would go undetected. Both
channels have to agree on the *pre-restart* captured value for an assertion
to mean what it claims.

### Mod state in the database is a cache, not a persistence store

**This finding took three separate sabotage rounds to establish and is
recorded here so nobody has to re-derive it.** It changes what assertion (3)
above is actually allowed to claim.

The first attempt at the installed-mod assertion deleted the mod's
`ModFileCache` row and expected the suite to go red on restart. It didn't:
`cache_local`'s boot-time per-instance disk scan
(`crates/carbon_app/src/managers/metadata/cache/mod.rs`, queued for every
instance at startup — `managers/instance/mod.rs:272-278`) rebuilds
`ModFileCache` from the jar file and its content hash alone, entirely
locally, the instant the row is missing.

The second attempt deleted only `ModrinthModCache` instead, reasoning that
the platform association (which project a jar corresponds to on Modrinth)
must be a real DB-only fact. Also green: a second, independent background
task (`cache_modplatform::<ModrinthModCacher>`, driven by
`instance_mods_needing_mr_refresh`) makes an **unconditional Modrinth
hash-lookup API call on every boot** for any mod lacking a
`ModrinthModCache` row, and re-derives the association before the assertion
ever runs.

The third attempt deleted `ModFileCache` **and** `ModMetadata` together
(cascading to both `ModrinthModCache` and `CurseForgeModCache` — the entire
per-mod DB footprint for that file). Still green.

**Conclusion: nothing about an installed mod's observable state — presence,
filename, size, enabled flag, or platform association — is uniquely tied to
any SQLite row surviving a restart.** The launcher fully rebuilds it from the
jar alone: a local disk scan for the file-level facts, plus an unconditional
background network lookup for the platform association. Mod state in the
database is a *cache* of what disk (and, for platform association, the
network) already says, not the thing that makes it durable.

Assertion (3) is kept, but documented — in the spec's own module and
assertion-level doc comments, and here — as a **regression check on that
reconciliation pipeline surviving a restart**, not as proof that any
SQLite row persisted. A bug that silently broke the local scan, or that
stopped the background re-association task from running at all, would still
be caught by this assertion; a bug that broke *only* `ModFileCache`/
`ModrinthModCache` persistence itself, with the reconciliation pipeline
intact, would not be.

**Open question this raises, not chased down here:** if the platform
association is re-derived by a live Modrinth API call on every boot, what
does an *offline* launch show for an installed mod? That lookup cannot run
without a network, and nothing in this suite exercises a launch with the
network unavailable.

## Database recovery tests

`dbRecovery.spec.ts` drives the database-open recovery ladder end to end:
plant a damaged or future-versioned `gdl_conf.db` with `helpers/dbSeed.ts`,
launch the real packaged app against it, and assert **both** halves of the
contract every launch-time status relies on — that the core actually emitted
the expected `_STATUS_:<EVENT>` line
(`crates/carbon_app/src/managers/db_bootstrap.rs`'s `DbStatus` funnel), and
that `apps/desktop/packages/main/index.ts` parsed that line and drove the
*correct* rung of the recovery screen
(`packages/preload/loading.ts`'s `fatalError`/`backwardsMigrationError`).
Asserting only the log line would pass even if the UI rendered nothing;
asserting only the UI would pass if the shell happened to guess right for the
wrong reason.

All six states `db_bootstrap.rs` can emit are covered, each in its own test:
`DB_CORRUPT`, `BACKWARDS_MIGRATION`, `DB_DIVERGED`, `DB_DOWNGRADE_FAILED`,
`DB_DOWNGRADED`, and `DB_MIGRATION_FAILED`. A seventh test seeds nothing —
a genuinely healthy, unseeded first launch — and asserts every one of those
same checks goes **negative** against it (`getCoreModule().type` reports
`"success"`, not `"error"`; none of the six `_STATUS_:` events above appear
in a healthy boot's log). This negative control is what proves the six tests
discriminate a real failure state rather than passing by construction — a
version of this suite whose recovery assertions were unconditionally true
would still fail this one.

Two further tests click a real recovery-screen button rather than only
asserting its presence: one drives "Restart" and confirms a brand-new OS
process for the app/core binary actually appeared (not just that the
handler's log line printed); the other drives "Reset Database & Restart" and
confirms the seeded database file is genuinely gone from disk afterward.

**The one deliberate omission.** `DB_DOWNGRADE_FAILED`'s "Restore Previous
Database" rung is asserted **absent**, never exercised. `compat.rs`'s
`down_run` runs every stored-down migration inside one transaction and rolls
the *whole* thing back on any failure, including the "no stored down" branch
the seed for this state drives — so the on-disk database after a failed
down-run is, byte for byte, identical to the pre-down-run snapshot `down_run`
just took. `snapshot_if_restorable` finds no difference and reports no
snapshot path, so the restore rung never renders. No seed that honestly
reaches `DB_DOWNGRADE_FAILED` can leave a differing, restorable snapshot
behind — under the current fully-transactional `down_run`, it isn't obvious
any real-world condition can either. The test asserts this rung's absence
explicitly rather than skipping the check.

### Correction: `DB_DOWNGRADED` needs a breaking tail, not an "additive" one

Read `compat.rs`'s `handle_ahead` directly rather than assume: a
migration-count tail that is entirely additive overlays **silently** —
`Proceed`, a `tracing::info!` line, no `_STATUS_:` event at all. `Downgraded`
only fires once a tail containing a *breaking* migration has its stored
`down_sql` run successfully and the resulting schema matches this binary's
own reference schema byte-for-byte. `dbSeed.ts`'s `DB_DOWNGRADED` seed
reflects this: it replays the checked-in `baseline.sql` and adds one
synthetic, independently-reversible breaking migration one version ahead,
because that is the only construction that can honestly reach `Downgraded`
rather than a silent additive overlay.

## Fixed defect: fatal database exits could lose their own diagnostic

Every fatal database-open path used to terminate with a direct
`std::process::exit(2)` call (`crates/carbon_app/src/managers/mod.rs`).
`std::process::exit` skips destructors, including the release build's
`tracing_appender::non_blocking` `WorkerGuard` — the object whose `Drop`
blocks until the background worker thread has actually flushed pending log
lines to `__gdl_logs__/*.log`. On an idle machine that worker thread
typically gets scheduled before the process dies anyway; under CPU
contention it does not, so the one line naming *why* the process was exiting
could be silently lost, producing an attached core log that is empty at
exactly the moment it mattered most.

Fatal DB exits now go through `logger::flush_and_exit(2)`
(`crates/carbon_app/src/logger.rs`), which takes the process-lifetime
`WorkerGuard` out of a static and drops it — forcing the flush — before
calling `std::process::exit`.

**Evidence standard used to confirm this was a genuine race, not a
hypothetical one:** `DB_CORRUPT` was seeded and the release `carbon_app`
binary run directly under `taskset -c 0` (pinning it to a single core, which
forces the tracing-appender worker thread to compete for the same core as
the exiting main thread and lose the scheduling race). Before the fix: 0 of
40 pinned runs had the fatal-error diagnostic in the log file. After the
fix: 40 of 40 did. Unpinned, on an otherwise-idle multi-core machine, the bug
did not reproduce either way (30/30 hit both before and after) — confirming
this was a real, timing-dependent race rather than a deterministic bug that
a small sample happened to miss. A `taskset`-pinned, tens-of-runs comparison
like this — not a handful of manual runs — is the bar for claiming a race
condition is actually fixed.

## No third-party surface

Unlike the install and mod suites — which download real Minecraft versions
from Mojang, real loader builds from meta.gdl.gg, and real mods from
CurseForge and Modrinth, and have all previously hit genuine live-service
flakiness (see "Third-party flakiness is observed, not theoretical" below)
— `persistence.spec.ts` and `dbRecovery.spec.ts` touch no third-party
service at all. The only network endpoint either file talks to is the local
mock IdP (`mock-idp/`); `dbRecovery.spec.ts` never even calls `completeLogin`
— every one of its tests launches against a seeded database and asserts on
the resulting startup status or screen, well before login would ever be
reached (and for the fatal states, before the app gets anywhere near the
login page at all). There is no live external dependency
either file's pass/fail can be blamed on, and neither has produced a flake
in any run so far.

## Suite wall-clock

Measured on this branch at `workers: 1` — what CI actually runs:
`playwright.config.ts` sets `workers: process.env.CI ? 1 : undefined`, and
GitHub Actions sets `CI=true` on every job, so every CI run is single-worker
regardless of local defaults.

- Full e2e suite (`init`, `login`, `instanceInstall`'s 8-entry vanilla
  matrix, `loaderInstall`'s 5-entry loader matrix, `modInstall`'s 2 tests,
  `modLifecycle`'s 4 tests, `persistence`'s 1 test, `dbRecovery`'s 9 tests
  — **35 tests total**): projected **~5.0 minutes (300s)** — the last
  full-suite measurement predates this file's negative-control test
  (`dbRecovery`'s ninth), which isolated re-measurement below places at a few
  more seconds; not re-measured end to end this round, since re-running the
  full install/mod matrix costs real time against Mojang/CurseForge/Modrinth
  independent of anything this wave touched.
- `persistence.spec.ts` alone: **~66s** for its 1 test (isolated run,
  re-measured this round).
- `dbRecovery.spec.ts` alone: **~46–48s** for its 9 tests (isolated run,
  re-measured this round after adding the negative control and fixing the
  retry test's sibling-detection races — up from ~24s for 8, consistent with
  one more full app launch plus `cleanupRelaunchSiblings`'s wider settle
  window on the two relaunch tests).
- The pre-existing 25-test install/mod suite (`init`, `login`,
  `instanceInstall`, `loaderInstall`, `modInstall`, `modLifecycle`) was
  previously measured at 187–200s (3.1–3.3 minutes) on its own; this wave
  now adds roughly 112–114s of test time (~66s + ~46–48s) on top of that when
  run in isolation — this wave's specs run sequentially after the
  install/mod specs under `fullyParallel: false`, `workers: 1`, so their
  cost adds rather than overlaps.
- Unit suite (`pnpm test:unit`, 190 tests across 19 files): **~1s**.
- Combined: **roughly 5.0 minutes** of e2e test time per OS, plus the unit
  suite's few seconds.

`modResolution.spec.ts`'s four tests (added after this section was last
measured) bring the suite to **39 tests total**. Not folded into the
bullets above as a precise re-measurement — this wave's own isolated runs
(this branch, this session) put the whole file at roughly **9 minutes**
under reasonable network conditions (two install-path tests against real
Modrinth/CurseForge searches and two full loader installs, plus two
update-path tests each doing a real install-then-update round trip), so the
full-suite total is closer to **~14 minutes** than the ~5.0 minutes above.
Real third-party network variance dominates this file's wall-clock far more
than anything else in the suite — see "Third-party flakiness is observed,
not theoretical" below, which predates this wave but applies to it at least
as much as to `modInstall.spec.ts`.

`.github/workflows/all_os.yml` runs this on three OS jobs (`ubuntu-22.04`,
`windows-2022`, `macos-14`) **in parallel**, each with its own 80-minute job
timeout, each forcing `workers: 1` the same way. Only Linux was directly
measured for this document; Windows and macOS are expected to land in the
same order of magnitude — the suite is network- and install-bound against
the same seeded matrix and the same live mod platforms, not CPU-bound — but
that is an expectation, not a second measurement.

The arithmetic that matters for a PR: because the three OS jobs run in
parallel rather than in series, the wall-clock this suite adds to a PR's
critical path is **one** OS's ~5.0 minutes, not three times it. The 3× only
shows up as total CI compute — three runners each spending ~5.0 minutes on
the test step, on top of their own build and lint steps — roughly 15 minutes
of test-step compute across the three jobs combined.

**At ~5.0 minutes against an 80-minute per-job timeout, the suite
comfortably fits a per-PR run today**, this wave's added ~78s (persistence +
recovery) and the mod suites' earlier ~40–50s both included. That is a
statement about the current measured duration, not a recommendation —
whether to split PR vs. nightly runs as the matrix grows is a call for a
human, not this document.

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
- Mod update-path channel-preference resolution is untested (no shipped UI
  for `AppConfiguration.mod_channels`), and `find_mod_update`'s CurseForge
  candidate fetch is asymmetric with its Modrinth counterpart on multi-loader
  instances — see "Known gaps" under `modResolution.spec.ts`'s own section
  above for both.
