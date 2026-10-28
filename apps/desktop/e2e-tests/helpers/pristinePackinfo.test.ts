import { afterEach, beforeEach, expect, it } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { createHash } from "node:crypto"
import {
  capturePristinePackinfo,
  readPackinfo,
  verifyPristinePackinfo,
  type Packinfo
} from "./packinfo.js"
import { snapshotTree } from "./instanceTree.js"
let root: string
const pristineBytes = Buffer.from("independent verified target bytes")
const hash = (algorithm: string, bytes: Buffer) =>
  createHash(algorithm).update(bytes).digest("hex")
const entry = {
  md5: hash("md5", pristineBytes),
  sha512: hash("sha512", pristineBytes)
}
const target: Packinfo = new Map([["/config/a.txt", entry]])
const stale: Packinfo = new Map([
  [
    "/config/a.txt",
    {
      md5: hash("md5", Buffer.from("old")),
      sha512: hash("sha512", Buffer.from("old"))
    }
  ]
])
beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-pristine-record-"))
  fs.mkdirSync(path.join(root, "instance/config"), { recursive: true })
  fs.writeFileSync(path.join(root, "instance/config/a.txt"), pristineBytes)
})
afterEach(() => fs.rmSync(root, { recursive: true, force: true }))
function publish(info: Packinfo) {
  fs.writeFileSync(
    path.join(root, "packinfo.json"),
    JSON.stringify({ files: Object.fromEntries(info) })
  )
}
it.each(["subject-wrong", "identical-wrong-twins", "missing", "extra"])(
  "twin verifier rejects %s with unchanged correct physical bytes",
  async (mode) => {
    const tree = await snapshotTree(path.join(root, "instance")),
      pristine = await capturePristinePackinfo(root, tree)
    const bad = new Map(target)
    if (mode === "missing") bad.delete("/config/a.txt")
    else if (mode === "extra") bad.set("/extra", entry)
    else
      bad.set("/config/a.txt", {
        ...entry,
        md5: hash("md5", Buffer.from("wrong"))
      })
    publish(bad)
    const actual = await readPackinfo(root)
    expect(() =>
      verifyPristinePackinfo(
        actual,
        mode === "identical-wrong-twins" ? bad : target,
        pristine
      )
    ).toThrow()
    expect(await snapshotTree(path.join(root, "instance"))).toEqual(tree)
  }
)
it.each(["missing", "third-md5", "wrong-sha512", "old-md5"])(
  "promotion rejects %s after real stale publication/readback",
  async (mode) => {
    const tree = await snapshotTree(path.join(root, "instance")),
      pristine = await capturePristinePackinfo(root, tree)
    publish(target)
    const retained = await readPackinfo(root)
    publish(stale)
    expect(await readPackinfo(root)).toEqual(stale)
    const promoted = new Map(target)
    if (mode === "missing") promoted.delete("/config/a.txt")
    else
      promoted.set("/config/a.txt", {
        ...entry,
        ...(mode === "wrong-sha512"
          ? { sha512: hash("sha512", Buffer.from("other")) }
          : {
              md5:
                mode === "old-md5"
                  ? stale.get("/config/a.txt")!.md5
                  : hash("md5", Buffer.from("third"))
            })
      })
    publish(promoted)
    const actual = await readPackinfo(root)
    expect(() =>
      verifyPristinePackinfo(actual, retained, pristine, {
        keys: ["/config/a.txt"],
        baseline: stale
      })
    ).toThrow()
    expect(await snapshotTree(path.join(root, "instance"))).toEqual(tree)
  }
)
it("correct twin and promoted full hashes succeed", async () => {
  const tree = await snapshotTree(path.join(root, "instance")),
    pristine = await capturePristinePackinfo(root, tree)
  publish(target)
  const actual = await readPackinfo(root)
  verifyPristinePackinfo(actual, target, pristine)
  verifyPristinePackinfo(actual, target, pristine, {
    keys: ["/config/a.txt"],
    baseline: stale
  })
})
it("missing twin key fails independently against retained pristine bytes", async () => {
  const tree = await snapshotTree(path.join(root, "instance")),
    pristine = await capturePristinePackinfo(root, tree)
  expect(() => verifyPristinePackinfo(new Map(), new Map(), pristine)).toThrow(
    /key set/
  )
})
it("changed physical bytes cannot supply retained pristine hashes", async () => {
  const tree = await snapshotTree(path.join(root, "instance"))
  fs.writeFileSync(path.join(root, "instance/config/a.txt"), "changed")
  await expect(capturePristinePackinfo(root, tree)).rejects.toThrow(
    /fixture changed/
  )
})
