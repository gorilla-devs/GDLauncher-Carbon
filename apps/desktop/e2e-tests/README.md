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
    - 1.8.4 (random)
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
- **13 minutes** is `waitForInstallComplete`'s install bound
  (`helpers/instances.ts`) — under the ceiling on purpose, so a genuinely
  stuck install throws its own diagnosable message instead of being cut off
  by Playwright with no explanation.

### Why later installs in the same run are fast

`authenticatedApp` is worker-scoped (see `fixtures/index.ts`), so every
matrix entry in a worker shares one launched app and one runtime path by
design — this is not incidental, and `deleteInstanceViaUi` only ever removes
the instance's own folder, never `assets/`, `libraries/`, or
`managed_javas/`. That's what makes the timings look the way they do: at
seed `469278827`, all 7 versions installed in 91.1s total. The first (cold)
install took ~20s and left ~830 MB on disk; every install after it took
around 5s. Minecraft's assets are content-addressed, and consecutive
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

A screenshot of the library grid rarely explains why a 12-minute install
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

## Known gaps

- A run killed between provisioning and teardown leaves an orphan row on
  api-test. The deletion sweep only claims rows deleted over seven days ago, so
  it will not collect them.
- Skin fetches still reach `textures.minecraft.net` and will fail for the
  synthetic profile. Harmless — they only log.
- The browser-OAuth enrollment path is not covered; the suite uses device code.
- A first launch into the library queues the `onBoarding` and `changelogs`
  modals on top of it, the same way the beta prompt does. They surface as
  `m[N]` query params on the URL rather than as page state, so `completeLogin`
  does not dismiss them — a test that needs to interact with the library will
  have to.
