import { createHash } from "node:crypto"
import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import {
  listModFiles,
  verifyModEnabled,
  verifyModInstalled
} from "./modVerify.js"

function sha1(data: Buffer | string): string {
  return createHash("sha1").update(data).digest("hex")
}

let modsDir: string
let tmpRoot: string

beforeEach(async () => {
  tmpRoot = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-e2e-modverify-"))
  modsDir = path.join(tmpRoot, "mods")
  await fs.mkdir(modsDir, { recursive: true })
})

afterEach(async () => {
  await fs.rm(tmpRoot, { recursive: true, force: true })
})

describe("verifyModInstalled", () => {
  it("verifies clean when the mod file is present", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "mod bytes")

    const result = await verifyModInstalled(modsDir, { filename: "sodium.jar" })
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports a missing mod by name", async () => {
    const result = await verifyModInstalled(modsDir, { filename: "absent.jar" })
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("absent.jar")
    expect(result.problems[0]).toContain(modsDir)
  })

  it("reports on an empty mods/ directory rather than passing vacuously", async () => {
    // modsDir exists (created in beforeEach) but has nothing in it — the
    // requested mod must still be reported missing, not silently ok.
    const entries = await fs.readdir(modsDir)
    expect(entries).toHaveLength(0)

    const result = await verifyModInstalled(modsDir, { filename: "sodium.jar" })
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
  })

  it("reports a size mismatch naming both the actual and expected size", async () => {
    const content = Buffer.from("mod bytes of a known length")
    await fs.writeFile(path.join(modsDir, "sodium.jar"), content)

    const result = await verifyModInstalled(modsDir, {
      filename: "sodium.jar",
      expectedSize: content.length + 100
    })

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(String(content.length))
    expect(result.problems[0]).toContain(String(content.length + 100))
  })

  it("reports a sha1 mismatch naming both the actual and expected hash", async () => {
    const content = Buffer.from("original mod bytes")
    await fs.writeFile(path.join(modsDir, "sodium.jar"), content)
    const expectedSha1 = sha1("something else entirely")
    const actualSha1 = sha1(content)

    const result = await verifyModInstalled(modsDir, {
      filename: "sodium.jar",
      expectedSha1
    })

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(expectedSha1)
    expect(result.problems[0]).toContain(actualSha1)
    expect(expectedSha1).not.toBe(actualSha1)
  })

  it("verifies clean when size and sha1 both match", async () => {
    const content = Buffer.from("matching bytes")
    await fs.writeFile(path.join(modsDir, "sodium.jar"), content)

    const result = await verifyModInstalled(modsDir, {
      filename: "sodium.jar",
      expectedSize: content.length,
      expectedSha1: sha1(content)
    })

    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("identifies a disabled mod as installed, not as missing", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar.disabled"), "mod bytes")

    const result = await verifyModInstalled(modsDir, { filename: "sodium.jar" })
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports both enabled and disabled variants present as an ambiguous state", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "enabled copy")
    await fs.writeFile(
      path.join(modsDir, "sodium.jar.disabled"),
      "disabled copy"
    )

    const result = await verifyModInstalled(modsDir, { filename: "sodium.jar" })
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("sodium.jar")
  })

  it("reports a clear problem naming the directory when mods/ does not exist, without throwing", async () => {
    const absentModsDir = path.join(tmpRoot, "does-not-exist-mods")

    const result = await verifyModInstalled(absentModsDir, {
      filename: "sodium.jar"
    })

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(absentModsDir)
  })

  it("reports a directory occupying the mod's filename as a problem rather than throwing EISDIR", async () => {
    // A stray directory (e.g. left behind by a botched extraction) sitting
    // where the mod file should be. Requesting a sha1 check is the specific
    // shape that used to throw: `fs.readFile` on a directory raises EISDIR.
    await fs.mkdir(path.join(modsDir, "sodium.jar"))

    await expect(
      verifyModInstalled(modsDir, {
        filename: "sodium.jar",
        expectedSha1: "deadbeef"
      })
    ).resolves.toMatchObject({ ok: false })

    const result = await verifyModInstalled(modsDir, {
      filename: "sodium.jar",
      expectedSha1: "deadbeef"
    })
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("sodium.jar")
    expect(result.problems[0]).toContain("directory")
  })

  it("reports a directory occupying the mod's filename even with no size/sha1 requested", async () => {
    await fs.mkdir(path.join(modsDir, "sodium.jar"))

    const result = await verifyModInstalled(modsDir, { filename: "sodium.jar" })
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("directory")
  })

  it("reports an empty-string expectedSha1 as a mismatch rather than skipping the check", async () => {
    // `if (expectedSha1)` would treat "" as "nothing to check" and pass
    // vacuously — the same falsy-but-present trap the brief calls out from
    // installVerify.ts's `parsed.objects ?? {}`. An empty string is a
    // caller error, not an opt-out, and must be reported.
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "mod bytes")

    const result = await verifyModInstalled(modsDir, {
      filename: "sodium.jar",
      expectedSha1: ""
    })

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("sha1")
  })

  it("reports a filename that resolves outside modsDir rather than verifying the wrong file", async () => {
    // A real file exists one level above modsDir — if path.join's escape
    // went unguarded, this would resolve to it and verify clean.
    await fs.writeFile(
      path.join(tmpRoot, "outside.jar"),
      "not actually inside mods/"
    )

    const result = await verifyModInstalled(modsDir, {
      filename: "../outside.jar"
    })

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("outside")
  })
})

describe("verifyModEnabled", () => {
  it("verifies clean when an enabled mod is expected enabled", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "mod bytes")

    const result = await verifyModEnabled(modsDir, "sodium.jar", true)
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("verifies clean when a disabled mod is expected disabled", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar.disabled"), "mod bytes")

    const result = await verifyModEnabled(modsDir, "sodium.jar", false)
    expect(result).toEqual({ ok: true, problems: [] })
  })

  it("reports a disabled mod expected enabled", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar.disabled"), "mod bytes")

    const result = await verifyModEnabled(modsDir, "sodium.jar", true)
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("disabled")
  })

  it("reports an enabled mod expected disabled", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "mod bytes")

    const result = await verifyModEnabled(modsDir, "sodium.jar", false)
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("enabled")
  })

  it("reports absence rather than 'disabled' for a file that does not exist at all", async () => {
    // The precise trap this pins: a verifier that infers "disabled" purely
    // from "the enabled path doesn't exist" would call this case clean
    // (enabled: false matches "not found at the enabled path"), silently
    // treating "never installed" as indistinguishable from "installed and
    // toggled off". They must stay distinct problems.
    const result = await verifyModEnabled(modsDir, "never-installed.jar", false)

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("never-installed.jar")
    expect(result.problems[0]).toContain("not found")
    // The path it checked is allowed to mention "disabled" (it names both
    // candidate paths it looked for) — what must never appear is a claim
    // that the mod *is* disabled, which would conflate "never installed"
    // with "installed and toggled off".
    expect(result.problems[0]).not.toMatch(/\bis disabled\b/)
  })

  it("also reports absence when the expectation was enabled", async () => {
    const result = await verifyModEnabled(modsDir, "never-installed.jar", true)
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
  })

  it("reports both enabled and disabled variants present as an ambiguous state", async () => {
    await fs.writeFile(path.join(modsDir, "sodium.jar"), "enabled copy")
    await fs.writeFile(
      path.join(modsDir, "sodium.jar.disabled"),
      "disabled copy"
    )

    const result = await verifyModEnabled(modsDir, "sodium.jar", true)
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
  })

  it("reports a clear problem naming the directory when mods/ does not exist, without throwing", async () => {
    const absentModsDir = path.join(tmpRoot, "does-not-exist-mods")

    const result = await verifyModEnabled(absentModsDir, "sodium.jar", false)

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(absentModsDir)
  })

  it("reports a filename that resolves outside modsDir rather than verifying the wrong file", async () => {
    await fs.writeFile(
      path.join(tmpRoot, "outside.jar"),
      "not actually inside mods/"
    )

    const result = await verifyModEnabled(modsDir, "../outside.jar", true)

    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain("outside")
  })
})

describe("listModFiles", () => {
  it("ignores directories and non-mod files", async () => {
    await fs.writeFile(path.join(modsDir, "real.jar"), "a")
    await fs.writeFile(path.join(modsDir, "disabled.jar.disabled"), "b")
    await fs.writeFile(path.join(modsDir, "notes.txt"), "not a mod")
    await fs.writeFile(path.join(modsDir, ".DS_Store"), "os metadata")
    // A directory, including one whose name would otherwise look like a jar,
    // must never be reported as a mod file.
    await fs.mkdir(path.join(modsDir, "subfolder.jar"))
    await fs.writeFile(
      path.join(modsDir, "subfolder.jar", "nested.jar"),
      "nested, not top-level"
    )

    const result = await listModFiles(modsDir)
    expect(result).toEqual(["disabled.jar.disabled", "real.jar"])
  })

  it("returns an empty array for an empty mods/ directory", async () => {
    const result = await listModFiles(modsDir)
    expect(result).toEqual([])
  })

  it("returns an empty array rather than throwing when mods/ does not exist", async () => {
    const absentModsDir = path.join(tmpRoot, "does-not-exist-mods")

    await expect(listModFiles(absentModsDir)).resolves.toEqual([])
  })
})
