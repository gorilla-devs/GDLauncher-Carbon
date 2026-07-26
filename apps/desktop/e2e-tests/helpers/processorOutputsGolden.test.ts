import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"
import {
  requiredLibraryPaths,
  type Processor,
  type RequiredLibrary,
  type SidedDataEntry
} from "./processorOutputs.js"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

/**
 * The single source of truth this port is checked against, per Task 5's
 * Fix round 1: `processor_outputs.rs`'s
 * `required_files_matches_committed_golden` test computes the *real* Rust
 * `required_files`'s output for the committed input fixture below and, on
 * `UPDATE_GOLDEN_PROCESSOR_OUTPUTS=1`, writes it to `output.json` in the
 * same directory. This test reads those exact same two files rather than a
 * TS-local copy, so there is exactly one golden fixture in the repo, not
 * two that could quietly drift apart from each other.
 *
 * Why this exists at all: `required_files` is load-bearing production code
 * (`crates/carbon_app/src/managers/instance/run/minecraft.rs` calls it to
 * decide whether Forge/NeoForge processors need to re-run at launch), and
 * `requiredLibraryPaths` here is an independent reimplementation of it for
 * the e2e suite, which has no Rust binding into the Playwright process. Two
 * independent implementations of the same logic can silently disagree
 * without something forcing them to be compared. Before this test existed,
 * nothing did: `processorOutputs.test.ts`'s other tests pin this port
 * against a *frozen copy* of the Rust fixtures, which catches this port
 * diverging from what those fixtures encode, but not the Rust source
 * itself changing behavior later — that fixture copy never gets re-read
 * from Rust, so a real `required_files` behavior change would leave both
 * test files green while the two implementations quietly disagree in
 * production. Reading the Rust-generated file directly closes that gap: a
 * `required_files` change either regenerates `output.json` (and this test
 * goes red until `processorOutputs.ts` is updated to match) or the Rust
 * test itself fails first (nobody regenerated the golden), and either way
 * a human is told.
 */
const GOLDEN_DIR = path.resolve(
  __dirname,
  "../../../../crates/carbon_app/fixtures/processor_outputs_golden"
)

interface GoldenCase {
  name: string
  processors: Processor[]
  data: Record<string, SidedDataEntry>
}

interface GoldenOutputCase {
  name: string
  required: RequiredLibrary[]
}

/**
 * Matches the Rust golden test's own normalization: `Processor.outputs` is
 * a Rust `HashMap`, so multiple entries in one processor's `outputs` map
 * can be visited in a different order each time the Rust side regenerates
 * the golden — sorting here compares the *set* of required files, not
 * incidental ordering from either side.
 *
 * Also normalizes `expectedSha1: null` to `undefined`: Rust's
 * `Option::None` serializes to JSON `null`, which `JSON.parse` reads back
 * as literal `null`, not `undefined` — a real cross-language "absent value"
 * representation difference, not a behavioral disagreement, so it is
 * normalized away here rather than either side changing its own convention
 * (`RequiredLibrary.expectedSha1` stays `undefined`-for-absent, matching
 * every other optional field in this module).
 */
function sortedByPath(required: RequiredLibrary[]): RequiredLibrary[] {
  return [...required]
    .map((f) => ({ ...f, expectedSha1: f.expectedSha1 ?? undefined }))
    .sort((a, b) => a.relativePath.localeCompare(b.relativePath))
}

const inputPath = path.join(GOLDEN_DIR, "input.json")
const outputPath = path.join(GOLDEN_DIR, "output.json")

let cases: GoldenCase[] = []
let goldenOutput: GoldenOutputCase[] = []
let loadError: Error | undefined

try {
  cases = JSON.parse(fs.readFileSync(inputPath, "utf8")) as GoldenCase[]
  goldenOutput = JSON.parse(
    fs.readFileSync(outputPath, "utf8")
  ) as GoldenOutputCase[]
} catch (err) {
  loadError = err instanceof Error ? err : new Error(String(err))
}

describe("requiredLibraryPaths matches the Rust-generated golden", () => {
  it("golden fixture files are readable (run `cargo test -p carbon_app required_files_matches_committed_golden` first if not)", () => {
    if (loadError) throw loadError
    expect(cases.length).toBeGreaterThan(0)
    expect(goldenOutput.length).toBe(cases.length)
  })

  for (const goldenCase of cases) {
    it(`"${goldenCase.name}" matches the committed golden output`, () => {
      const expectedCase = goldenOutput.find((c) => c.name === goldenCase.name)
      expect(
        expectedCase,
        `no golden output case named "${goldenCase.name}" in ${outputPath}`
      ).toBeDefined()

      const computed = sortedByPath(
        requiredLibraryPaths(goldenCase.processors, goldenCase.data)
      )
      const expected = sortedByPath(expectedCase!.required)

      expect(computed).toEqual(expected)
    })
  }
})
