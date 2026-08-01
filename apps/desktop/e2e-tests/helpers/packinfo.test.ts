import { describe, expect, it, beforeEach, afterEach } from "vitest"
import { createHash } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { classifyPackinfo, packinfoDataPath, readPackinfo } from "./packinfo.js"

let root: string

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-packinfo-"))
  fs.mkdirSync(path.join(root, "instance"), { recursive: true })
})
afterEach(() => {
  fs.rmSync(root, { recursive: true, force: true })
})

function hashes(body: string) {
  return {
    sha512: createHash("sha512").update(body).digest("hex"),
    md5: createHash("md5").update(body).digest("hex")
  }
}

function writePackinfo(files: Record<string, string>) {
  const entries = Object.fromEntries(
    Object.entries(files).map(([key, body]) => [key, hashes(body)])
  )
  fs.writeFileSync(
    path.join(root, "packinfo.json"),
    JSON.stringify({ files: entries })
  )
}

function writeData(rel: string, body: string) {
  const target = path.join(root, "instance", rel)
  fs.mkdirSync(path.dirname(target), { recursive: true })
  fs.writeFileSync(target, body)
}

describe("readPackinfo", () => {
  it("keeps the leading slash on every key", async () => {
    writePackinfo({ "/mods/a.jar": "a" })
    const info = await readPackinfo(root)
    expect([...info.keys()]).toEqual(["/mods/a.jar"])
    expect(info.get("/mods/a.jar")?.md5).toEqual(hashes("a").md5)
  })

  it("throws a named error when packinfo.json is absent", async () => {
    await expect(readPackinfo(root)).rejects.toThrow(/packinfo\.json/)
  })
})

describe("packinfoDataPath", () => {
  it("resolves a slash-prefixed key under <root>/instance", () => {
    expect(packinfoDataPath("/x", "/config/a.json")).toBe(
      path.join("/x", "instance", "config", "a.json")
    )
  })
})

describe("classifyPackinfo", () => {
  it("reports a file whose bytes still match as pristine", async () => {
    writePackinfo({ "/mods/a.jar": "a" })
    writeData("mods/a.jar", "a")
    expect(await classifyPackinfo(root)).toEqual({
      pristine: ["/mods/a.jar"],
      modified: [],
      missing: []
    })
  })

  it("reports a file whose bytes changed as modified", async () => {
    writePackinfo({ "/mods/a.jar": "a" })
    writeData("mods/a.jar", "CHANGED")
    expect(await classifyPackinfo(root)).toEqual({
      pristine: [],
      modified: ["/mods/a.jar"],
      missing: []
    })
  })

  it("reports an absent file as missing", async () => {
    writePackinfo({ "/mods/a.jar": "a" })
    expect(await classifyPackinfo(root)).toEqual({
      pristine: [],
      modified: [],
      missing: ["/mods/a.jar"]
    })
  })

  it("finds a disabled pack mod under its .disabled name", async () => {
    writePackinfo({ "/mods/a.jar": "a" })
    writeData("mods/a.jar.disabled", "a")
    expect(await classifyPackinfo(root)).toEqual({
      pristine: ["/mods/a.jar"],
      modified: [],
      missing: []
    })
  })
})
