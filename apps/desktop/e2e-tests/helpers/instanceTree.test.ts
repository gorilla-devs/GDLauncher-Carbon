import { describe, expect, it, beforeEach, afterEach } from "vitest"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { diffTrees, snapshotTree, type Tree } from "./instanceTree.js"

let root: string

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-tree-"))
})
afterEach(() => {
  fs.rmSync(root, { recursive: true, force: true })
})

function write(rel: string, body: string) {
  const target = path.join(root, rel)
  fs.mkdirSync(path.dirname(target), { recursive: true })
  fs.writeFileSync(target, body)
}

describe("snapshotTree", () => {
  it("keys every file by its forward-slash relative path", async () => {
    write("options.txt", "fov:70")
    write("config/sodium.json", "{}")
    write("mods/a.jar", "jar")

    const tree = await snapshotTree(root)

    expect([...tree.keys()].sort()).toEqual([
      "config/sodium.json",
      "mods/a.jar",
      "options.txt"
    ])
  })

  it("records size and sha256 per file", async () => {
    write("a.txt", "abc")
    const tree = await snapshotTree(root)
    expect(tree.get("a.txt")).toEqual({
      size: 3,
      // sha256("abc")
      sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    })
  })

  it("does not record directories", async () => {
    fs.mkdirSync(path.join(root, "empty"), { recursive: true })
    const tree = await snapshotTree(root)
    expect(tree.size).toBe(0)
  })

  it("returns an empty tree for a directory that does not exist", async () => {
    const tree = await snapshotTree(path.join(root, "nope"))
    expect(tree.size).toBe(0)
  })
})

describe("diffTrees", () => {
  const entry = (sha: string): { size: number; sha256: string } => ({
    size: 1,
    sha256: sha
  })

  it("partitions paths into added, removed, changed and same", () => {
    const before: Tree = new Map([
      ["keep", entry("aa")],
      ["edit", entry("bb")],
      ["gone", entry("cc")]
    ])
    const after: Tree = new Map([
      ["keep", entry("aa")],
      ["edit", entry("dd")],
      ["new", entry("ee")]
    ])

    expect(diffTrees(before, after)).toEqual({
      added: ["new"],
      removed: ["gone"],
      changed: ["edit"],
      same: ["keep"]
    })
  })

  it("sorts every bucket, so assertions are order-independent", () => {
    const before: Tree = new Map()
    const after: Tree = new Map([
      ["z", entry("1")],
      ["a", entry("2")]
    ])
    expect(diffTrees(before, after).added).toEqual(["a", "z"])
  })
})
