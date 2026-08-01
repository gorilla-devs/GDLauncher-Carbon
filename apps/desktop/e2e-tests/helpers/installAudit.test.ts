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

Files deleted:
 - /mods/old.jar
 - /config/dropped.json

Files replaced:
 - /mods/common.jar

Files created:
 - instance/mods/new.jar
`

describe("parseInstallAudit", () => {
  it("parses all three skip reasons, with md5s on the modified one", () => {
    expect(parseInstallAudit(FULL).skipped).toEqual([
      { file: "/mods/gone.jar", reason: "deleted-by-user" },
      {
        file: "/config/edited.json",
        reason: "modified-by-user",
        originalMd5: "00112233445566778899aabbccddeeff",
        currentMd5: "ffeeddccbbaa99887766554433221100"
      },
      { file: "/saves/world/level.dat", reason: "in-save-folder" }
    ])
  })

  it("parses the three plain lists", () => {
    const audit = parseInstallAudit(FULL)
    expect(audit.deleted).toEqual(["/mods/old.jar", "/config/dropped.json"])
    expect(audit.replaced).toEqual(["/mods/common.jar"])
    expect(audit.created).toEqual(["instance/mods/new.jar"])
  })

  it("returns empty sections for an audit that decided nothing", () => {
    expect(
      parseInstallAudit("GDLauncher Modpack Install/Update Audit\n")
    ).toEqual({ skipped: [], deleted: [], replaced: [], created: [] })
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
      created: []
    })
  })
})
