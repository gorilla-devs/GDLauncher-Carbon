import { describe, expect, it, beforeEach, afterEach } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { parseInstallAudit, readInstallAudit } from "./installAudit.js"

const FULL = `GDLauncher Modpack Install/Update Audit

Files that could not be replaced:
 - /mods/gone.jar: deleted by user
 - /config/edited.json: modified by user
     original md5: 00112233445566778899aabbccddeeff
     current md5:  ffeeddccbbaa99887766554433221100
 - /saves/world/level.dat: files in /saves will never be modified
 - /mods/twin.jar: disabled by user
 - /config/already.json: already present

Files deleted:
 - /mods/old.jar
 - /config/dropped.json

Files replaced:
 - /mods/common.jar

Files created:
 - /mods/new.jar

Files unchanged:
 - /mods/steady.jar

Files re-enabled:
 - /mods/reenabled.jar

Files removed at user request:
 - /mods/pruned.jar
`

describe("parseInstallAudit", () => {
  it("parses all five skip reasons, with md5s on the modified one", () => {
    expect(parseInstallAudit(FULL).skipped).toEqual([
      { file: "/mods/gone.jar", reason: "deleted-by-user" },
      {
        file: "/config/edited.json",
        reason: "modified-by-user",
        originalMd5: "00112233445566778899aabbccddeeff",
        currentMd5: "ffeeddccbbaa99887766554433221100"
      },
      { file: "/saves/world/level.dat", reason: "in-save-folder" },
      { file: "/mods/twin.jar", reason: "disabled-by-user" },
      { file: "/config/already.json", reason: "already-present" }
    ])
  })

  it("parses the six plain lists", () => {
    const audit = parseInstallAudit(FULL)
    expect(audit.deleted).toEqual(["/mods/old.jar", "/config/dropped.json"])
    expect(audit.replaced).toEqual(["/mods/common.jar"])
    expect(audit.created).toEqual(["/mods/new.jar"])
    expect(audit.unchanged).toEqual(["/mods/steady.jar"])
    expect(audit.reEnabled).toEqual(["/mods/reenabled.jar"])
    expect(audit.userRemoved).toEqual(["/mods/pruned.jar"])
  })

  it("returns empty sections for an audit that decided nothing", () => {
    expect(
      parseInstallAudit("GDLauncher Modpack Install/Update Audit\n")
    ).toEqual({
      skipped: [],
      deleted: [],
      replaced: [],
      created: [],
      unchanged: [],
      reEnabled: [],
      userRemoved: []
    })
  })

  it("keeps a colon in a filename out of the reason", () => {
    const audit = parseInstallAudit(
      "GDLauncher Modpack Install/Update Audit\n\n" +
        "Files that could not be replaced:\n" +
        " - /config/weird: name.json: modified by user\n" +
        "     original md5: aa\n" +
        "     current md5:  bb\n"
    )
    expect(audit.skipped[0].file).toBe("/config/weird: name.json")
    expect(audit.skipped[0].reason).toBe("modified-by-user")
  })

  it("throws on an unrecognised skip reason rather than dropping it", () => {
    expect(() =>
      parseInstallAudit(
        "GDLauncher Modpack Install/Update Audit\n\n" +
          "Files that could not be replaced:\n" +
          " - /a: chewed by the dog\n"
      )
    ).toThrow(/chewed by the dog/)
  })

  it("throws on an orphaned md5 continuation line with no owning skip entry", () => {
    expect(() =>
      parseInstallAudit(
        "GDLauncher Modpack Install/Update Audit\n\n" +
          "     original md5: 00112233445566778899aabbccddeeff\n"
      )
    ).toThrow(/orphaned md5 continuation line/)
  })
})

it("keeps list semantics with retained cleanup recovery notes", () => {
  const withRecovery = FULL.replace(
    "GDLauncher Modpack Install/Update Audit\n",
    "GDLauncher Modpack Install/Update Audit\nCleanup recovery: .repair-recovery/cleanup-example/\nCleanup retained /config/retired.cfg: .repair-recovery/cleanup-example/entry/payload\n"
  )
  expect(parseInstallAudit(withRecovery)).toEqual(parseInstallAudit(FULL))
})

describe("readInstallAudit", () => {
  let root: string
  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-audit-"))
  })
  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true })
  })

  it("returns null when the pass never ran", async () => {
    expect(await readInstallAudit(root)).toBeNull()
  })

  it("reads and parses an audit that exists", async () => {
    fs.mkdirSync(path.join(root, ".install_audit"), { recursive: true })
    fs.writeFileSync(path.join(root, ".install_audit", "audit.txt"), FULL)
    expect((await readInstallAudit(root))?.replaced).toEqual([
      "/mods/common.jar"
    ])
  })

  it("distinguishes title-only audit from no audit", async () => {
    fs.mkdirSync(path.join(root, ".install_audit"), { recursive: true })
    fs.writeFileSync(
      path.join(root, ".install_audit", "audit.txt"),
      "GDLauncher Modpack Install/Update Audit\n"
    )
    expect(await readInstallAudit(root)).toEqual({
      skipped: [],
      deleted: [],
      replaced: [],
      created: [],
      unchanged: [],
      reEnabled: [],
      userRemoved: []
    })
  })
})

const INHERITED_REASONS = [
  "toString",
  "constructor",
  "__proto__",
  "hasOwnProperty"
]
it.each(INHERITED_REASONS)("rejects inherited skip reason %s", (reason) => {
  expect(() =>
    parseInstallAudit(`Files that could not be replaced:\n - /a: ${reason}\n`)
  ).toThrow(/unrecognised audit skip reason/)
})
it("preserves case alias and colon filename hashes", () => {
  expect(
    parseInstallAudit(
      "Files that could not be replaced:\n - /A: case-aliased with a tracked path\n - /config/a: b: modified by user\n     original md5: aa\n     current md5:  bb\n"
    ).skipped
  ).toEqual([
    { file: "/A", reason: "case-aliased-with-tracked-path" },
    {
      file: "/config/a: b",
      reason: "modified-by-user",
      originalMd5: "aa",
      currentMd5: "bb"
    }
  ])
})
it.each(INHERITED_REASONS)(
  "physical audit reader rejects inherited reason %s",
  async (reason) => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-audit-inherited-"))
    try {
      fs.mkdirSync(path.join(root, ".install_audit"))
      fs.writeFileSync(
        path.join(root, ".install_audit/audit.txt"),
        `Files that could not be replaced:\n - /a: ${reason}\n`
      )
      await expect(readInstallAudit(root)).rejects.toThrow(
        /unrecognised audit skip reason/
      )
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  }
)

const BAD_CONTINUATION_OWNERS = [
  "Files that could not be replaced:\n - /a: deleted by user\nFiles created:\n - /b\n",
  "Files that could not be replaced:\n - /a: modified by user\nFiles created:\n - /b\n",
  "Files that could not be replaced:\n - /a: modified by user\nFiles that could not be replaced:\n",
  ...[
    "deleted by user",
    "disabled by user",
    "already present",
    "files in /saves will never be modified",
    "case-aliased with a tracked path"
  ].map(
    (reason) =>
      `Files that could not be replaced:\n - /edited: modified by user\n - /a: ${reason}\n`
  )
]
for (const prefix of ["     original md5: aa", "     current md5:  bb"]) {
  it.each(BAD_CONTINUATION_OWNERS)(
    `rejects unowned ${prefix} after %s`,
    (text) => {
      expect(() => parseInstallAudit(text + prefix + "\n")).toThrow(
        /orphaned md5 continuation/
      )
    }
  )
  it("physical reader rejects a cross-section " + prefix, async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-audit-owner-"))
    try {
      fs.mkdirSync(path.join(root, ".install_audit"))
      fs.writeFileSync(
        path.join(root, ".install_audit/audit.txt"),
        BAD_CONTINUATION_OWNERS[0] + prefix + "\n"
      )
      await expect(readInstallAudit(root)).rejects.toThrow(
        /orphaned md5 continuation/
      )
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  })
}
