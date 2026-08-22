# E2E tests

Playwright drives the **packaged** launcher, so a build must exist before the
suite can run.

```bash
GDL_E2E=1 pnpm build:linux-x64      # or build:win-x64 / build:mac-universal
pnpm test:e2e
```

`GDL_E2E` adds the `e2e` cargo feature, which is what makes
`--gdl_e2e_auth_base` and `--gdl_e2e_entitlement_key` do anything. Without it
the same command builds the shipping artifact, which ignores both flags — no
published artifact carries the feature.

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

## Modpack tests

Nine spec files cover the modpack lifecycle end to end. All nine verify on
disk — against the pack's own `.mrpack` index, `packinfo.json`, the install
audit, or a twin instance — never by trusting what the app renders about the
same fact.

| File | Tests | What it proves | Wall-clock |
|---|---|---|---|
| `modpackInstall.spec.ts` | 2 | A first install from each platform lays every declared file and override on disk and writes a correct `packinfo.json`. | ~1.2 min |
| `modpackLifecycle.spec.ts` | 1 | An upgrade *and* a downgrade against a **dirtied** instance preserve user and game data while correctly replacing, deleting and creating pack files. | ~1.5 min |
| `modpackSaveGuard.spec.ts` | 2 | `PlanReason::InSaveFolder` actually stops a version change *and* a repair from touching a pack-tracked file under `saves/`. | ~1.1 min |
| `modpackLock.spec.ts` | 4 | A fresh install starts locked; unlocking flips the flag and unblocks Addons; unpairing drops the association entirely. | ~4 min |
| `modpackReinstall.spec.ts` | 2 | Repair restores a *deleted* pack file **and** repairs a *damaged* one — the same treatment for both — and is refused outright while the game is running. | ~1.6 min |
| `modpackRepairPreview.spec.ts` | 2 | The repair preview's counts and per-file verdicts match what a real repair then does, for a deleted/truncated/edited file and a disabled pack mod, both left disabled and re-enabled. | ~1.1 min |
| `modpackCurseforgeVersion.spec.ts` | 1 | A **CurseForge** version change lands the instance byte-identical to a fresh install of the same target file. | ~1.2 min |
| `modpackChangeVersionGuard.spec.ts` | 1 | A version change started mid-game is **refused outright, not deferred** — `change_modpack` now carries the same `LaunchState` guard `repair_modpack` always had. | ~1.1 min |
| `modpackInterruptedStaging.spec.ts` | 2 | A download killed by a core crash resumes on the next launch; a lost packinfo promotion strands the new files as untouchable. | ~2.1 min |

Four of the nine own their harness instead of using the shared
`authenticatedApp` fixture, for two different reasons. Three —
`modpackLifecycle`, `modpackReinstall`'s second test, and
`modpackChangeVersionGuard` — leave a real JVM running and must be free to
kill it without disturbing an app other specs share. The fourth,
`modpackInterruptedStaging`'s first test, owns its harness for an unrelated
reason: it `SIGKILL`s the core process itself mid-download, which is not
something a fixture other specs still need alive afterward could survive.
All four copy `gameLaunch.spec.ts`'s inline `startHarness`/`stopHarness`
try/finally rather than importing it — importing any value from a `.spec.ts`
re-registers that file's own `test()` calls, the same reason
`helpers/resolutionFixtures.ts` exists.

### The fixture pack, and why this one

**Modrinth `remarkably` (`MNW3LUwK`), Fabric 1.20.1**, three pinned version
ids:

| id | name | mods | mod bytes | overrides |
|---|---|---|---|---|
| `eGIPjEwN` | 1.15.11 (NEW) | 25 | 28.4 MiB | 9 (8 config) |
| `8QjqOzvP` | 1.15.9 (MID) | 24 | 28.1 MiB | 9 (8 config) |
| `PVccZjDs` | 1.13 (OLD) | 27 | 25.6 MiB | 12 (11 config) |

Deltas, measured 2026-08-01: MID → NEW is +4 / −5 / =20, a small bump;
MID → OLD is +16 / −13 / =11, a large jump.

Chosen by screening 50 Fabric 1.20.1 packs. It is the only viable candidate
under 30 MiB per version: every other small pack either weighs 400–570 MiB —
which no test wants to download three times — or ships **VulkanMod, Distant
Horizons or OptiFine**, any of which breaks `modpackLifecycle.spec.ts`'s real
launch for reasons unrelated to what is under test. `remarkably` ships Sodium
and Iris only, both plain OpenGL, which `gameLaunch.spec.ts` already proves
works here. 1.20.1 + Fabric also matches `fixtures/installedInstance.ts`, so
the Minecraft substrate (assets, libraries, JRE) is already warm.

Version ids are hardcoded because Modrinth version ids are immutable —
publishing a new release never changes an existing id — which is what makes
the deltas above stable inputs rather than a moving target. If one 404s the
author deleted a version: re-pin all three against the same criteria and
re-measure, rather than dropping to two.

**CurseForge: `boosted-fps` (`520990`), file `4713831`** — only ever
installed, never upgraded. The version-change depth runs on Modrinth, whose
version API is unpaginated. `MODPACK_CF_QUERY` is deliberately longer than the
pack's own name: `"boosted fps"` does not rank `520990` first in CurseForge's
own search (it opens `702170`), and `openModpackPage` always clicks the first
result with no project id to disambiguate against, so an ambiguous query
silently installs the wrong pack and fails several steps later on unrelated
data — which is exactly what happened the first time. `helpers/modpackFixtures.ts`
records the live rankings behind both queries.

**Two fixture facts worth knowing before re-pinning anything.** First, **zero
mods differ in bytes between MID and NEW** — the bump is pure add/remove,
unsurprising since a mod update ordinarily changes its own versioned filename
rather than keeping it and changing the bytes underneath. That is why
`modpackLifecycle.spec.ts`'s `deleteReturning` case uses `/options.txt` rather
than a jar, and why `/options.txt` is currently the *only* spare pristine
non-mod candidate (`/config/lithium.properties` is taken by `editTarget`).
There is zero margin: a launch that happens to rewrite `options.txt` exhausts
both, and the test fails loudly naming that rather than silently weakening.
Second, the pack ships `overrides/options.txt` itself, so that file can never
appear in a post-launch tree diff's `added` set — `modpackLifecycle.spec.ts`
uses a new file under `instance/logs/` (Minecraft's own Log4j directory, not
the launcher's sibling per-launch capture) as its "the game really ran"
premise instead.

### The three artifacts, and which helper reads each

The modpack pipeline leaves three on-disk records. Each has its own pure-Node
reader — no Playwright, no DOM — so each is unit-tested directly:

- **`<root>/packinfo.json`** — the pipeline's record of which files the pack
  owns and what they hashed to when it installed them. Written by
  `packinfo::scan_dir` from a scan of the staging tree; read back by
  `process_modpack_staging` on the next version change to decide, file by
  file, what may be replaced. `helpers/packinfo.ts` reads it, and
  `classifyPackinfo` computes **exactly the partition that code will
  compute** — pristine / modified / missing — which is what lets a test
  predict a version change's decisions instead of merely observing them. The
  layout is easy to get wrong: the file lives at the instance **root**, its
  keys are relative to `<root>/instance` and carry a leading slash, and the
  scanner strips a trailing `.disabled` before recording a key, so a disabled
  pack mod is tracked under its enabled name and sits on disk under the
  suffixed one.
- **`<root>/.install_audit/audit.txt`** — a plain-text record of every
  decision the pass made: per file, whether it replaced, deleted, created, or
  refused to touch it and why. This is the single best oracle in the feature,
  because it lets a test prove *why* a file survived rather than only that it
  did. The directory is deleted and recreated on every pass, so it always
  describes the most recent one only. `helpers/installAudit.ts` parses it,
  with two deliberate non-normalisations: `null` (no audit directory, "the
  pass never ran") is kept distinct from four empty sections ("the pass ran
  and decided nothing"), and `Files created:` carries staging-relative,
  `instance/`-prefixed paths with no leading slash while the other three
  sections carry packinfo's own leading-slash keys. That difference is real,
  so the parser preserves it and each comparison site normalises.
- **`<root>/.setup/`** — present only while an install or version change is in
  flight (`run/mod.rs:527-528` removes it once the setup path completes), which
  is what makes its *absence* the assertion in `modpackReinstall.spec.ts`'s
  refused-while-running test.

`helpers/instanceTree.ts`'s `snapshotTree` is the fourth reader: a recursive
path → `{size, sha256}` map of the whole instance data directory, which is how
"nothing else changed" is asserted as a set difference rather than as a list
of files someone remembered to check. It neither follows nor records symlinks;
no pack in the fixture set ships one.

### The apply-staging decision table

`process_modpack_staging`
(`crates/carbon_app/src/managers/instance/run/modpack.rs:747-823`) decides in
**two independent passes**, and almost every surprise in this feature comes
from the fact that they do not talk to each other:

1. a loop over `packinfo.files`, which can *skip*, *delete* or *replace* an
   already-known file, and
2. a walk over whatever physically landed in the staging directory, which
   moves a staged file into any path that is currently **empty**. It iterates
   *staged* entries, not on-disk ones, so a path nothing was staged for is
   invisible to it — which is why a wholly-untracked user file is structurally
   unreachable by this pass, not merely skipped by it.

**On a fresh install only pass 2 runs at all.** `process_modpack` writes its
scan to `tmp-packinfo.json` (`run/modpack.rs:638`) and renames it to
`packinfo.json` only at :899, *after* the staging apply. So the first time
through, `process_modpack_staging` reads no packinfo, pass 1 is skipped
entirely, and the staging walk places 100% of the pack's files. That matters
for anyone sabotaging this code to check a test really fails: disabling the
walk's creation does not weaken a version change, it prevents the instance
from ever being installed, and the test dies at its own fixture setup instead
of on the assertion under test. Gate such a sabotage on
`instance_root.join("packinfo.json").exists()` to confine it to the
second-and-later passes.

| On-disk state of a pack file | Pass 1 | Pass 2 | Net result |
|---|---|---|---|
| Untouched, target version ships it | replace | path occupied | updated |
| Untouched, target version drops it | delete | nothing staged | removed |
| Untouched, under `/saves` | skip `in-save-folder` | nothing staged | **kept** |
| Edited by user | skip `modified-by-user` | path occupied | **kept as edited** |
| Truncated by user | skip `modified-by-user` | path occupied | **corruption preserved** |
| Deleted by user, staged copy exists | skip `deleted-by-user` | path empty → created | **silently reinstated** |
| Deleted by user, nothing staged | skip `deleted-by-user` | nothing staged | stays deleted |
| Not in packinfo at all (a user's own file) | never visited | never staged | untouched |

**What gets staged is itself not obvious**, and the last three rows turn on
it. `prepare_modpack_from_mrpack`
(`crates/carbon_app/src/managers/minecraft/modrinth.rs:277-289`) skips
downloading any `files[]` entry whose target-version sha512 already matches
what the *old* packinfo recorded — so a mod whose bytes are unchanged across
the bump never gets a fresh copy placed in staging, and pass 2 has nothing to
recreate a deleted one from. Overrides have no such optimisation: they are
re-extracted unconditionally on every pass. That asymmetry is why
`modpackReinstall.spec.ts` draws all three of its target files from the pack's
overrides rather than its mods — on a *same-version* reinstall the comparison
is packinfo-against-itself and therefore always matches, so **every**
`.files`-declared mod is skip-optimised unconditionally and a deleted mod can
never come back, however the UI labels the action.

### The twin-instance oracle, and why CurseForge needs one

Every other assertion in this suite has an oracle external to the app: the
Modrinth tests fetch the version's own `.mrpack` index live and compare bytes
against it. That does not port to CurseForge, for two independent reasons.

First, `packinfo.json` cannot substitute. `packinfo::scan_dir` hashes the
**staging** copy of each file, *before* the rename into its final location, so
comparing on-disk bytes against packinfo is self-referential — it can catch a
change made after the install (a user edit), never a bug that staged the wrong
bytes in the first place. Second, CurseForge file downloads require an
`x-api-key` (see the 2026-07-19 CDN incident) that this suite does not hold in
standalone mode, so there is no index to fetch.

`modpackCurseforgeVersion.spec.ts` solves it with a **twin instance**: install
the target file fresh, snapshot it, delete it, then build a second instance at
the older file and version-change it onto the same target. The twin was
produced by the *install* path, so it is genuinely external to the
*version-change* path under test, and it needs no API key. What that buys is
the property that matters most about the whole feature and that nothing here
proved before — **a version change lands you where a fresh install of the
target would have** — confirmed byte-for-byte across 60-odd paths.

The twin is built and torn down *before* the subject exists, which is not
fussiness. `next_folder` (`managers/instance/mod.rs:1563`) de-duplicates an
instance's **shortpath**, but nothing de-duplicates its **display name**, so
two installs of one modpack produce two rows with an identical `name`. That
breaks `newestTileName` (it diffs tile names and would see no new one),
`byInstanceName` (two tiles under strict mode), and `readInstanceByName`
(which throws outright on a duplicate, deliberately).

### Why the interrupted-apply case is reconstructed, not raced

`modpackInterruptedStaging.spec.ts` crashes the core for real in its first
test — `SIGKILL` mid-download, no cleanup — because that window is seconds
wide for a 28 MiB pack. It does **not** do the same for the apply phase, which
is one md5 pass plus a handful of renames. Racing that would be flaky, and
`retries: 0` turns flaky into a red build; there is also no log signal to time
against, since the core's `debug!`/`trace!` output does not reach this suite's
stdout capture — only `_STATUS_:` lines do.

So the second test reproduces the *consequence* instead, exactly and without a
synthetic `.setup/`. Promotion is the last step: `process_modpack` writes its
scan to `tmp-packinfo.json` and only renames it over `packinfo.json` at
`run/modpack.rs:899`, after the apply. A crash during the apply therefore
leaves the **old** packinfo describing files that are already the **new**
version on disk — which is what you get by completing a version change and
then restoring the packinfo you saved beforehand.

That test runs on **CurseForge** while its sibling runs on Modrinth, and the
split is load-bearing rather than incidental. It needs at least one file whose
bytes genuinely differ between the two versions, since that is the only kind
that can sit in the stale record under one hash and on disk under another.
`remarkably` has none — its delta is pure add/remove — so against that pack the
misclassification set would come back empty and the assertion would be vacuous.
`boosted-fps` `4595849` → `4713831` has six measured such paths.

### Why `modpackSaveGuard` seeds state

`SkipReplaceReason::InSaveFolder` only fires for a path that is **already in
packinfo** *and* starts `/saves`. No pack in this fixture set ships
`overrides/saves/**`, so nothing about a real install ever puts a save under
packinfo's tracking, and the branch is unreachable through any realistic
fixture. That test seeds the state deliberately — the same class of tampering
`helpers/dbSeed.ts` already does for the DB-recovery suite — and lives in its
own file so `modpackLifecycle.spec.ts` stays a description of real user
behaviour with nothing hand-planted in it.

Writing `packinfo.json` back has one trap worth restating: the real file is
`{"_version":"1","files":{…}}` (`PackInfoWrapper` is
`#[serde(tag = "_version")]`), so the test mutates `raw.files` in place rather
than reserialising a fresh `{files}` object. The latter would silently drop
the tag and make the *next* read fail to parse at all — a `serde_json` error
on an unrelated line instead of a clean red on the assertion the test is
actually about.

### A real launch marks pack configs as modified

Sodium and Iris normalise their own configs on first run, and this pack ships
both as overrides. Their packinfo entries still carry the pack's original md5,
so once a launch has rewritten them on disk they legitimately classify as
`modified by user` for the *next* version change. That is correct product
behaviour — preserve, don't replace — not a bug, but it means any test that
launches the game must partition with `classifyPackinfo` **after** the launch
and **before** any mutation, and pick its edit target from the pristine list
at runtime rather than assuming one survived.

### Product findings these tests pinned, now all fixed

Eight, all found by building this suite. None were fixed as they were found —
each was pinned by an assertion instead, deliberately: writing tests is the
wrong place to change product behaviour, and each needed its own product
decision first. Every one has since been fixed; each entry below still names
the assertion that pins it, which now proves the fix holds rather than merely
recording the bug. Two are different in kind: #7 is a regression these tests
introduced into themselves, and #8 a pre-existing product bug outside the
modpack surface entirely. Both surfaced in a full packaged-build run rather
than while the tests were being written, and both are listed here for the
same reason as the rest — pinned by an assertion, fixed, verified green.

1. **`change_modpack` leaks a `.setup/` that defers the version change to the
   next launch.** `repair_modpack` refuses outright while the instance is
   launching, queued, running or being deleted
   (`managers/instance/modpack/mod.rs:210-217`); `change_modpack` has no such
   guard. The consequence is *not* a mid-game `mods/` rewrite — both route
   through `prepare_game`, which bails on `LaunchState::Running` of its own
   accord (`run/mod.rs:194-196`), so no staging ever runs under a live JVM.
   It is that `change_modpack` has already created `.setup/` and written
   `change-pack-version.json` into it (`modpack/mod.rs:161-176`) *before*
   reaching that refusal. So a version change started mid-game is not
   cancelled, it is **deferred**: the next legitimate launch finds the file
   and applies it. Until then every further `change_modpack` call bails with
   "Instance has not completed the setup phase", because `.setup` now exists.
   `repair_modpack`'s guard is exactly what prevents the same leak on its
   own path — which is why `modpackReinstall.spec.ts`'s refusal test asserts
   on `.setup/`'s absence rather than only on the instance still running.
   Both halves are now covered: the guarded path by `modpackReinstall.spec.ts`,
   the unguarded one by `modpackChangeVersionGuard.spec.ts`, which observes the
   whole sequence live — the leak, the second call bailing on
   "Instance has not completed the setup phase", and the change applying itself
   unprompted on the next launch.
   **Fixed:** `change_modpack` now carries the same `LaunchState` guard as
   reinstall and cleans up `.setup` on failure; refusals render an inline
   error in the version modal (`modpack-version-update-error`).
2. **A user-deleted pack file is silently reinstated** when the target version
   still ships it and something was staged for that path — the two passes are
   independent, so pass 1's `deleted by user` decision does not stop pass 2
   recreating it. The same path then lands in **two contradictory audit
   sections at once**, `Files that could not be replaced:` *and*
   `Files created:`. "User-deleted stays deleted" is only true for a file the
   target version drops.
   **Fixed:** `process_modpack_staging` now reconciles every path through a
   single planner (`apply_plan::plan`) that decides each path exactly once,
   instead of two independent passes. A user-deleted path decides
   `Keep`/`DeletedByUser` and stays that way — there is no second,
   independent pass left to recreate it, and the audit records it in one
   section only.
3. **Reinstall repairs a missing file but not a damaged one.** A truncated
   file still exists and its md5 no longer matches packinfo, so pass 1 classes
   it `ModifiedByUser` and pass 2 skips it because the path is occupied. A
   *deleted* file is restored. So the repair a user is most likely to want —
   "this jar is broken, give me a clean copy" — is the one reinstall does not
   perform, while the UI presents it as a general repair action.
   **Fixed:** `repair_modpack` now writes a `.setup/repair` marker
   (`RepairMarkerFile`) that switches the whole pipeline into a true repair,
   rather than reconciling as an ordinary version change onto the same
   version. `process_modpack` swaps its skip-optimisation oracle for a live
   disk scan (`disk_scan::scan_instance_as_packinfo`) whenever the marker is
   present, so a damaged or deleted file's real bytes — not the record —
   decide whether a fresh copy needs fetching; a corrupt or missing file can
   no longer skip-optimise away. `process_modpack_staging` then selects
   `apply_plan::ApplyMode::Repair`, which reconciles every pack-tracked path
   against the target version alone: present-but-wrong becomes
   `Replace`/`RepairOverwrote`, missing becomes `Create`/`RepairRestored` —
   the same treatment for both, restoring a missing file and repairing a
   damaged one, with no remaining asymmetry. Pinned by
   `modpackReinstall.spec.ts`.
4. **`packinfo::scan_dir` drops files unchanged between versions.** This is
   the serious one, because its consequence is closest to data loss. The
   skip-if-unchanged optimisation above means an unchanged file is never
   staged, and `scan_dir` builds the new `packinfo.json` by walking **physical
   staging files**, so every unchanged file falls out of the record: all 20
   mods unchanged MID → NEW vanished from packinfo after the upgrade. **Nine
   of those are paths OLD does not ship**, so on the downgrade pass 1 never
   visits them (not in packinfo) and pass 2 never stages them (not in OLD's
   manifest) — **they survive permanently, with no audit trace at all.**
   `modpackLifecycle.spec.ts` pins those nine by name in
   `KNOWN_STALE_SURVIVORS_AFTER_DOWNGRADE`, as a bidirectional `toEqual`, so
   the assertion catches that set *growing* and also catches one spuriously
   disappearing. **Platform-independent**, confirmed on CurseForge by
   `modpackCurseforgeVersion.spec.ts`: the version-changed instance's packinfo
   is missing entries a fresh install records, while every one of those files
   is still on disk and byte-correct. It is a property of `scan_dir` itself,
   not of one platform's downloader.
   **Fixed:** `process_modpack`'s snapshot block (`run/modpack.rs`) now
   merges the skip-oracle's hash back into the freshly scanned packinfo for
   every skip-optimised path, right after the `scan_dir` call. A "skipped"
   path is, by construction of the skip condition itself, one where the
   oracle's recorded hash already equals the target version's declared hash
   — so the merge is not a guess, it is the target's own hash. packinfo.json
   is complete after every version change, in both directions, confirmed on
   both Modrinth (`modpackLifecycle.spec.ts`, `KNOWN_STALE_SURVIVORS_AFTER_DOWNGRADE`
   removed — nothing survives undeleted any more) and CurseForge
   (`modpackCurseforgeVersion.spec.ts`).
5. **A refused version change tells the user nothing.** `handleUpdate`
   (`ModPackVersionUpdate/index.tsx:126-151`) awaits the mutation and only
   afterwards calls `closeModal()` and `navigate("/library")`. There is no
   `catch` anywhere on that path, so when `change_modpack` rejects — which is
   what happens mid-game — both are skipped: the modal stays open, unchanged,
   with no toast, no inline error, and no hint that a change is now pending on
   disk. The user is looking at a dialog that appears to have ignored the
   button they pressed. Pinned by `modpackChangeVersionGuard.spec.ts`, which
   asserts the route never leaves the instance detail.
   **Fixed:** `change_modpack` now carries the same `LaunchState` guard as
   reinstall and cleans up `.setup` on failure; refusals render an inline
   error in the version modal (`modpack-version-update-error`).
6. **An interrupted apply strands the new files as untouchable.** Because
   `packinfo.json` is promoted last, after `execute_plan` applies the whole
   plan (`run/modpack.rs`), a crash during the
   apply leaves the old record describing files that are already the new
   version on disk. Their md5 no longer matches, so the next pass classes them
   `ModifiedByUser` and skips them — permanently. The instance sits
   half-upgraded, and the repair action a user would reach for is precisely the
   one that refuses to touch them. Pinned by
   `modpackInterruptedStaging.spec.ts`'s second test, which asserts both that
   the misclassification happens and that the skipped files' bytes are left
   untouched.
   **Fixed:** `repair_modpack`'s repair mode (see finding #3 above)
   resolves each path by deciding it against the *target* version alone —
   `apply_plan::decide_repair` never consults the stale `old` record for a
   path `target` still ships, unlike `ApplyMode::VersionChange`. A file the
   crash already landed correctly now decides `Keep`/`Unchanged` (disk
   already equals target) instead of `ModifiedByUser`, resolving the
   misclassification rather than merely detecting it. Running the repair
   also promotes a fresh `packinfo.json` at the end of the same pass, like
   any other reinstall, so the stale record itself is corrected too, not
   just papered over for one more launch. Pinned by
   `modpackInterruptedStaging.spec.ts`'s second test.
7. **A CurseForge modpack's `packinfo.json` could record files that were not
   on disk.** Not a pre-existing bug like #1-6 above — found in this same
   branch's own first full packaged-build run, and *caused* by this branch:
   a fresh-install regression introduced by the single-planner staging
   rewrite (`apply_plan::plan`, finding #2's fix above), caught by
   `modpackInstall.spec.ts`'s CurseForge test (`boosted-fps`, file
   `4713831`), and fixed before merge. `apply_plan::plan`'s `/saves` rule
   short-circuited every `/saves`-prefixed path to `Keep`/`InSaveFolder`
   unconditionally, in every disk state, including `Missing`. On a fresh
   install (`old = None`) a pack's own shipped world files are staged and
   hashed into `tmp-packinfo.json` by `packinfo::scan_dir` — which has no
   saves-specific branch at all, it unconditionally hashes whatever its
   whitelist names — *before* the planner ever runs; the planner then
   decided `Keep` for those `/saves` paths, so `execute_plan`'s no-op `Keep`
   arm never moved them out of staging, and the staging directory is deleted
   immediately after. packinfo promised three files
   (`saves/FPS Stress Test.zip`, `saves/FPS Test.zip`,
   `saves/FPS Winter Stress Test.zip`) that were never written to disk,
   reproducible 3/3. The pre-planner two-pass code never had this failure
   mode — its saves guard only ever covered the *delete* branch, not
   creation.
   **Fixed:** the `/saves` rule is now conditional instead of absolute.
   Existing save bytes (disk `Present` or `Disabled`) are still protected
   unconditionally, in both modes — never overwritten, replaced, re-enabled,
   or deleted. A `Missing` save `old` already recorded is still protected in
   both modes, including repair — a deleted world is never resurrected. A
   `Missing` save `old` never recorded (a from-scratch install, or a pack
   version newly shipping a world) now falls through to the normal rows, so
   it gets created like any other pack-staged file instead of being promised
   in packinfo and silently dropped. See `apply_plan::plan`'s doc comment
   for the exact contract. Pinned by `apply_plan.rs`'s planner unit tests
   (the "saves folder, disk missing" section) and by
   `modpackInstall.spec.ts`'s CurseForge test, both green.
8. **A fire-and-forget post-login navigate could clobber whatever the app had
   since navigated to.** `AuthFlow.tsx`'s `handleExit`
   (`pages/Login/AuthFlow.tsx:638-672`) runs once as a side effect of the
   login flow's own state machine reaching its `exiting` phase, and ends with
   `navigator.navigate("/library")` — but only *after* awaiting a sidebar
   slide-out, an optional seasonal splash, and (on every fresh runtime path,
   since `flow.data.isFirstLaunch` is always true here) a ~2.6s welcome-text
   fade sequence. Nothing synchronized that final `navigate()` against
   whatever the app was doing by the time it actually fired — measured live
   (instrumenting `history.pushState`/`document.startViewTransition` and
   resolving the minified production bundle's stack through the Vite-emitted
   `.js.map`) at a consistent ~4.0-4.1s after `completeLogin` returns. A test
   (or a real user) that navigated away and interacted with the app for a few
   seconds right after login finished got silently yanked back to `/library`
   mid-action: no error, no console warning, nothing. Reproduced two distinct
   ways — `installModpackVersion`'s "the Versions tab click reached the route
   and then left it" bounce, and `scrollVersionRowIntoView`'s rows-disappear
   failure while scrolling the same page — against code neither the original
   diagnosis nor any earlier commit on this branch touched
   (`InfiniteScrollVersionsQueryWrapper`, `AddonViewPage`). This was blocking
   enough of this suite's own modpack-version specs (anything that reaches a
   pack's Versions tab shortly after login) that `fixtures/login.ts`'s
   `dismissStartupModals` carried a `POST_LOGIN_SETTLE_MS` wait to work
   around it.
   **Fixed:** `handleExit` now reads `useLocation()`'s `pathname` fresh at
   the moment it is about to navigate — after all the awaits, not captured
   at closure creation — and skips the navigate (logging a `console.debug`)
   whenever the login route (`"/"`) is no longer current. Both the ordinary
   `/library` destination and the settings-return path
   (`flow.data.returnPath`) go through the same guard, so a second login
   after logout (which leaves the app back at `"/"`) still navigates
   normally. `POST_LOGIN_SETTLE_MS` and the wait it drove are gone from
   `fixtures/login.ts` — the suite no longer pads around this window, it
   exercises it directly on every spec that navigates shortly after login.

### Search timeouts are measured, not guessed

`SEARCH_RESULTS_TIMEOUT` (`helpers/mods.ts`) and `MODPACK_SEARCH_TIMEOUT`
(`helpers/modpacks.ts`) are both **90s**. Measured directly on 2026-08-01, a
cold Modrinth search takes **30.204s** and returns HTTP 200, then 0.084s and
0.078s warm. Both constants used to be `30_000` — sitting exactly on the
measured cold path, so the *first* search of any suite run failed
deterministically on a perfectly good response. That cost four spec runs
before it was diagnosed. Re-measure before lowering either; `mods.ts`'s old
comment claimed "a couple of seconds", which was stale by an order of
magnitude.

A fast repeat is not necessarily a cached *success*. Modrinth caches its own
5xx too: the literal query `"fabric api"` returned HTTP 500
(`Typesense search failed: Request Timeout`) cold at 30.2s and then served
that same 500 from cache at 0.08s, while a control query returned 200. No
client-side timeout can fix that, and with `retries: 0` it is a genuine red
build — exactly the third-party failure that policy exists to surface.

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

`playwright.config.ts` sets `workers: 1` unconditionally — not
`process.env.CI ? 1 : undefined` — so a local run and a CI run execute the
same way and a local measurement is directly comparable.

**Measured end to end on 2026-08-01, Linux, standalone mode, after the
modpack suite landed: 54 tests collected, `53 passed, 1 skipped` in `16.8m`.**
The one
skip is `login.spec.ts`'s "reached the real backend for the user profile",
which needs `TEST_BASE_API` and `E2E_INTERNAL_AUTH_TOKEN` and therefore runs
only in proxy mode — that is CI's mode, so CI executes 54.

**Re-measured end to end on 2026-08-07, Linux, standalone mode, against a
real packaged `build:linux-x64-e2e` build — the first full run of everything
this branch changed, `repair`/preview/disabled-mods/install-feedback included:
73 tests collected, `67 passed, 4 failed, 1 skipped, 1 did not run` in
`28.5m`.** Grew from 54 to 73 — more than the +3 added here
(`modpackRepairPreview.spec.ts`'s 2, +1 in `modpackSaveGuard.spec.ts`'s
repair leg) accounts for, since 54 was measured 2026-08-01 and other work
has added coverage in the days since; not fully re-audited here. Of the 4
failures: 2 (`modpackChangeVersionGuard.spec.ts`'s guard hazard,
`modpackLifecycle.spec.ts`/`modpackLock.spec.ts`'s spec-side bugs) are fixed
and verified green; 2 (`modLifecycle.spec.ts`, `persistence.spec.ts`) were
transient live-Modrinth-CDN flakes that did not reproduce on individual
re-run. One further failure (`modpackInstall.spec.ts`'s CurseForge test) was
a genuine, reproducible (3/3) regression this branch introduced into itself
via its own single-planner staging rewrite — see "Product findings these
tests pinned, now all fixed" above (item #7) for the full diagnosis — since
fixed
and verified green. The "did not run" was a cascade of `modpackLock.spec.ts`'s
failure in a serial test block, resolved by the same fix.

Per-file figures below are **isolated** runs, several re-measured
2026-08-07 alongside the new spec. Do not add them up and expect the total
above:

| File | Tests | Isolated |
|---|---|---|
| `modpackInstall.spec.ts` | 2 | ~1.2 min |
| `modpackLifecycle.spec.ts` | 1 | ~1.5 min |
| `modpackSaveGuard.spec.ts` | 2 | ~1.1 min |
| `modpackLock.spec.ts` | 4 | ~4 min |
| `modpackReinstall.spec.ts` | 2 | ~1.6 min |
| `modpackRepairPreview.spec.ts` | 2 | ~1.1 min |
| `modpackCurseforgeVersion.spec.ts` | 1 | ~1.2 min |
| `modpackChangeVersionGuard.spec.ts` | 1 | ~1.1 min |
| `modpackInterruptedStaging.spec.ts` | 2 | ~2.1 min |
| `persistence.spec.ts` | 1 | ~66s |
| `dbRecovery.spec.ts` | 9 | ~46–48s |
| `modResolution.spec.ts` | 4 | ~9 min |

Those sum to well over the measured 16.8 minutes, and the discrepancy is the
point: **an isolated run pays for the whole Minecraft substrate itself**,
while a full run pays once. The worker-scoped `installedInstance` and
`forgeInstance` fixtures install one instance each per worker and every later
test reuses them, and the assets, libraries and JRE any spec needs are already
on disk after the first install in the run. `modResolution.spec.ts`'s ~9
minutes in isolation is dominated by two full loader installs that a
full-suite run has already done. Treat the per-file numbers as "what this file
costs if you run just it while iterating", not as a share of the total.

- Unit suite (`pnpm test:unit`, **256 tests across 25 files**): **~1.2s**.
- Combined: **roughly 17 minutes** of e2e test time per OS, plus the unit
  suite's second or so.

`.github/workflows/all_os.yml` runs this on three OS jobs (`ubuntu-22.04`,
`windows-2022`, `macos-14`) **in parallel**, each with its own 80-minute job
timeout, each forcing `workers: 1` the same way. Only Linux was directly
measured for this document; Windows and macOS are expected to land in the
same order of magnitude — the suite is network- and install-bound against
the same seeded matrix and the same live mod platforms, not CPU-bound — but
that is an expectation, not a second measurement.

The arithmetic that matters for a PR: because the three OS jobs run in
parallel rather than in series, the wall-clock this suite adds to a PR's
critical path is **one** OS's ~17 minutes, not three times it. The 3× only
shows up as total CI compute — three runners each spending ~17 minutes on the
test step, on top of their own build and lint steps — roughly 50 minutes of
test-step compute across the three jobs combined.

**At ~17 minutes against an 80-minute per-job timeout, the suite still fits a
per-PR run**, but the margin is no longer generous: it was ~5 minutes before
the mod-resolution and modpack suites, and it is now a little over a fifth of
the budget. Two things would eat the rest faster than test count alone
suggests — a third-party slowdown, since each *distinct* search query in the
suite pays Modrinth's ~30s cold path once and there are roughly half a dozen
of them, and any spec that launches the real game, which the modpack suite
added two of. That is a statement about the current measured
duration, not a recommendation; whether to split PR vs. nightly runs as the
matrix grows is a call for a human, not this document.

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
- **Modpack export and import are uncovered.** Both ship real surfaces
  (`InstanceExport`'s modal and `ImportEntity`'s scan/target mutations), and
  no test in this suite drives either. That includes `.gdlpack`, GDL's own
  format, which has a real importer (`importer/gdlpack.rs`) and a real export
  path.
- **Server packs are uncovered.** `ServerPackDownloadButton` renders beside
  the download button on CurseForge version rows — `selectors.ts` already
  documents it as the reason the row button needs its own anchor — and nothing
  ever clicks it.
- **The modpack update-check appears to be dead code.**
  `check_curseforge_modpack_updates` / `check_modrinth_modpack_updates` are
  spawned per instance on refresh (`managers/instance/mod.rs:312-324`) with
  errors explicitly discarded, and they write `modpack_update_curseforge` /
  `modpack_update_modrinth`. Neither field appears in `api/`, in `domain/`, or
  anywhere in `mainWindow` — the `has_update` that does reach the API is on the
  *Mod* struct, a different feature. So this fires two live platform requests
  per modpack instance and nothing consumes the result. Not a coverage gap:
  there is no UI to test. Worth confirming before it grows one.
- **Interrupting the *apply* phase is uncovered**, as opposed to the download
  phase, which `modpackInterruptedStaging.spec.ts` does crash for real. Its
  consequence is covered by reconstruction (see above), but no test kills the
  core between `staging-packinfo.json` appearing and `modpack-complete` being
  written. Doing so reliably would need a test-only hook in product code.
- **Further addon topics are unstarted**: dependency resolution inside a
  modpack instance, disk/DB consistency for pack-owned mods, and metadata
  caching. Those need their own design.
- **A symlinked file under an instance is invisible to `snapshotTree`**,
  which neither follows nor records them, so every "nothing else changed" set
  difference in the modpack specs would silently ignore one. No pack in the
  fixture set ships a symlink.
