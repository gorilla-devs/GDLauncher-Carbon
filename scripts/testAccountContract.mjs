import { spawnSync } from "node:child_process"
import { existsSync, mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"

/**
 * Account invalidation contract between the core and the app.
 *
 * A carbon_app test drives the real account publication code and writes the
 * invalidation events it emits to `TASK19_ACCOUNT_EVENTS`. The desktop's
 * queryInvalidation.integration.ts then feeds those events to the real
 * frontend parser and QueryClient. Both sides get one fresh path, so the
 * frontend never reads a stale capture; it fails, never skips, when the file
 * is missing or wrong. Neither half opens a network connection or starts the
 * app.
 *
 * To run the two halves by hand, give both the same path:
 *
 *   fixture="$(mktemp -d)/task19-account-events.json"
 *   TASK19_ACCOUNT_EVENTS="$fixture" cargo test -p carbon_app --features e2e --bin carbon_app <PRODUCER> -- --exact
 *   TASK19_ACCOUNT_EVENTS="$fixture" pnpm --dir apps/desktop exec vitest run --config vitest.account-integration.config.mjs
 *
 * If the first command prints `running 0 tests`, the Rust test was renamed:
 * update PRODUCER below.
 */
const PRODUCER =
  "managers::account::publication::task19_publication_tests::captured_account_events_match_the_live_no_input_wire_contract"

const dir = mkdtempSync(join(tmpdir(), "gdl-account-contract-"))
const env = {
  ...process.env,
  TASK19_ACCOUNT_EVENTS: join(dir, "task19-account-events.json")
}

/** @param {string} command */
function run(command) {
  console.log(`> ${command}`)
  const result = spawnSync(command, { stdio: "inherit", shell: true, env })
  if (result.error) throw result.error
  return result.status ?? 1
}

let status = 1
try {
  // `--features e2e` matches the last carbon_app build `pnpm test:rust`
  // makes, so CI reuses it instead of compiling the core again. The feature
  // only adds test auth endpoints; account publication is the same.
  status = run(
    `cargo test -p carbon_app --features e2e --bin carbon_app ${PRODUCER} -- --exact`
  )
  // A renamed or moved producer test matches nothing, and cargo still
  // exits 0 after "running 0 tests".
  if (status === 0 && !existsSync(env.TASK19_ACCOUNT_EVENTS)) {
    console.error(
      "The Rust producer test did not write the account events fixture, so there is nothing to check. " +
        "Check that PRODUCER in scripts/testAccountContract.mjs still names an existing carbon_app test."
    )
    status = 1
  }
  if (status === 0) {
    status = run(
      "pnpm --dir apps/desktop exec vitest run --config vitest.account-integration.config.mjs"
    )
  }
} finally {
  rmSync(dir, { recursive: true, force: true })
}
process.exit(status)
