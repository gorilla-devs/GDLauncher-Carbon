import { describe, it, expect, vi, afterEach, beforeEach } from "vitest"
import fs from "node:fs/promises"
import fss from "node:fs"
import childProcess from "node:child_process"
import path from "node:path"
import os from "node:os"
import fse from "fs-extra"
import fg from "fast-glob"
import ts from "typescript"
import { assertSafeRuntimeTarget } from "../runtimePathGuard"
import {
  inventoryRuntimeTree,
  relocateRuntime,
  validateRuntimeDestination,
  selectRuntimePath,
  removableMountFor,
  leftPartialCopy,
  unsupportedRuntimeEntries,
  UnsupportedRuntimeEntriesError,
  removedPartialCopy,
  runtimeTargetRefusal,
  leftInOldFolder,
  forgetUnavailableRuntimePath,
  IncompleteMigrationError,
  LEFT_BEHIND_SHOWN,
  MIGRATION_MARKER
} from "../runtimeRelocation"
import { resetDatabaseAndRestart } from "../databaseReset"
import { restoreDatabaseSnapshot } from "../databaseRestore"
import { withRuntimeOperation } from "../runtimeOperation"

// Execute the actual IPC callbacks without importing Electron or starting an app.
function callback(name: string, globals: Record<string, unknown>) {
  const source = fss.readFileSync(
    path.resolve("packages/main/index.ts"),
    "utf8"
  )
  const ast = ts.createSourceFile(
    "index.ts",
    source,
    ts.ScriptTarget.Latest,
    true
  )
  let expression = ""
  function visit(node: ts.Node) {
    if (
      ts.isCallExpression(node) &&
      node.expression.getText(ast) === "ipcMain.handle" &&
      ts.isStringLiteral(node.arguments[0]) &&
      node.arguments[0].text === name
    ) {
      expression = node.arguments[1].getText(ast)
    }
    ts.forEachChild(node, visit)
  }
  visit(ast)
  const js = ts.transpileModule(`const callback = ${expression}`, {
    compilerOptions: { target: ts.ScriptTarget.ES2022 }
  }).outputText
  return new Function(...Object.keys(globals), `${js}; return callback`)(
    ...Object.values(globals)
  )
}
/**
 * The new folder is on a filesystem that is flushed file by file, as every
 * one is on Windows and macOS, and FAT or exFAT is on Linux.
 */
function flushFileByFile() {
  const statfs = fs.statfs.bind(fs)
  vi.spyOn(fs, "statfs").mockImplementation((async (...args: any[]) => ({
    ...(await (statfs as any)(...args)),
    type: 0x4d44
  })) as any)
}
const roots: string[] = []
afterEach(async () => {
  vi.restoreAllMocks()
  for (const root of roots.splice(0))
    await fs.rm(root, { recursive: true, force: true })
})
async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "runtime-relocation-"))
  roots.push(root)
  const source = path.join(root, "source"),
    target = path.join(root, "target"),
    pointer = path.join(root, "runtime_path_override")
  await fs.mkdir(source)
  await fs.mkdir(target)
  await fs.writeFile(path.join(source, "save"), "old save")
  await fs.writeFile(pointer, source)
  const stop = vi.fn(async () => {}),
    check = vi.fn(async (_core: unknown) => {}),
    relaunch = vi.fn()
  const dialog = { showMessageBox: vi.fn(async () => ({ response: 0 })) }
  const globals = {
    relocateRuntime,
    validateRuntimeDestination,
    leftPartialCopy,
    unsupportedRuntimeEntries,
    removedPartialCopy,
    runtimeTargetRefusal,
    leftInOldFolder,
    recordRuntimeMoveNotice,
    takeRuntimeMoveNotice,
    dismissRuntimeMoveNotice,
    IncompleteMigrationError,
    withRuntimeOperation,
    interruptibleUntilCommitted,
    relocationLeaseHeld,
    runtimeDestinationContext: () => ({}),
    overrideCLIDataPath: undefined,
    overrideEnvDataPath: undefined,
    coreModule: Promise.resolve({ type: "success", result: {} }),
    coreProcessHandle: {},
    checkRelocation: check,
    stopCoreForRelocation: stop,
    stopCoreForShutdown: stop,
    terminateAndObserveCoreExit: stop,
    resetDatabaseAndRestart,
    restoreDatabaseSnapshot,
    fs,
    fss,
    fse,
    fg,
    path,
    assertSafeRuntimeTarget,
    CURRENT_RUNTIME_PATH: source,
    RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
    RUNTIME_PATH_DEFAULT_NAME: "data",
    console,
    dialog,
    app: { getPath: () => root, relaunch, exit: vi.fn() },
    win: null,
    showRecoveryNotice: dialog.showMessageBox,
    ownedShutdown: async () => {
      await stop()
      return "stopped"
    }
  }
  return {
    root,
    source,
    target,
    pointer,
    stop,
    check,
    relaunch,
    dialog,
    globals,
    move: callback("changeRuntimePath", globals),
    validate: callback("validateRuntimePath", globals)
  }
}
describe("runtime relocation production IPC", () => {
  it("rejects linked world before stopping core", async () => {
    const f = await fixture()
    const external = path.join(f.root, "external")
    await fs.mkdir(external)
    await fs.writeFile(path.join(external, "world"), "valuable")
    await fs.symlink(external, path.join(f.source, "Linked"))
    await expect(f.move(null, f.target)).rejects.toThrow(/link|unsupported/i)
    expect(f.stop).not.toHaveBeenCalled()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
  })
  it("names every link inside the folder at once, and only in a move", async () => {
    const f = await fixture()
    const external = path.join(f.root, "external")
    await fs.mkdir(external)
    await fs.mkdir(path.join(f.source, "instances"))
    await fs.symlink(external, path.join(f.source, "Linked"), "junction")
    await fs.symlink(
      external,
      path.join(f.source, "instances", "saves"),
      "junction"
    )
    const refused = await f.rawMove(null, f.target)
    expect(refused).toMatchObject({
      type: "failed",
      recovery: "none",
      unsupported: { folder: f.source, count: 2 }
    })
    expect(refused.unsupported.entries.sort()).toEqual(
      ["Linked", path.join("instances", "saves")].sort()
    )
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("shortens a long list of links for the dialog and keeps the count", () => {
    const entries = Array.from(
      { length: LEFT_BEHIND_SHOWN + 5 },
      (_, index) => `link-${index}`
    )
    const found = unsupportedRuntimeEntries(
      new UnsupportedRuntimeEntriesError("/data", entries)
    )
    expect(found?.entries).toHaveLength(LEFT_BEHIND_SHOWN)
    expect(found?.count).toBe(entries.length)
    expect(unsupportedRuntimeEntries(new Error("other"))).toBeUndefined()
  })
  it("moves a data folder that is itself a link or junction, and keeps the link", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "instances"))
    await fs.writeFile(path.join(f.source, "instances", "world"), "world")
    const link = path.join(f.root, "linked-data")
    await fs.symlink(f.source, link, "junction")
    const outcome = await relocateRuntime({
      source: link,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop
    })
    expect(outcome).toMatchObject({ changed: [], notDeleted: [] })
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(
      await fs.readFile(path.join(f.target, "instances", "world"), "utf8")
    ).toBe("world")
    await expect(fs.lstat(path.join(f.source, "save"))).rejects.toMatchObject({
      code: "ENOENT"
    })
    expect((await fs.lstat(link)).isSymbolicLink()).toBe(true)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
  })
  it("still refuses links inside a data folder reached through a link", async () => {
    const f = await fixture()
    await fs.symlink(
      path.join(f.root),
      path.join(f.source, "Linked"),
      "junction"
    )
    const link = path.join(f.root, "linked-data")
    await fs.symlink(f.source, link, "junction")
    await expect(
      relocateRuntime({
        source: link,
        target: f.target,
        pointer: f.pointer,
        stopCore: f.stop
      })
    ).rejects.toThrow(UnsupportedRuntimeEntriesError)
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("switches whatever links either folder holds: a switch copies nothing", async () => {
    const f = await fixture()
    const external = path.join(f.root, "external")
    await fs.mkdir(external)
    await fs.writeFile(path.join(f.target, "gdl_conf.db"), "theirs")
    await fs.symlink(external, path.join(f.source, "Linked"), "junction")
    await fs.symlink(external, path.join(f.target, "Linked"), "junction")
    await f.move(null, f.target, true)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
    expect(
      (await fs.lstat(path.join(f.source, "Linked"))).isSymbolicLink()
    ).toBe(true)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })
  it("switches away from a data folder that is itself a link", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "gdl_conf.db"), "theirs")
    const link = path.join(f.root, "linked-data")
    await fs.symlink(f.source, link, "junction")
    await relocateRuntime({
      source: link,
      target: f.target,
      pointer: f.pointer,
      switchOnly: true,
      stopCore: f.stop
    })
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
  })
  it("accepts a link to a folder as the new data folder, for a switch and a move", async () => {
    const f = await fixture()
    const linked = path.join(f.root, "linked-target")
    await fs.symlink(f.target, linked, "junction")
    expect(await f.validate(null, linked)).toBe("valid")
    await f.move(null, linked)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readFile(f.pointer, "utf8")).toBe(linked)

    const g = await fixture()
    await fs.writeFile(path.join(g.target, "gdl_conf.db"), "theirs")
    const linkedFull = path.join(g.root, "linked-target")
    await fs.symlink(g.target, linkedFull, "junction")
    expect(await g.validate(null, linkedFull)).toBe("potentially_valid")
    await g.move(null, linkedFull, true)
    expect(await fs.readFile(g.pointer, "utf8")).toBe(linkedFull)
  })
  it("refuses a link that leads nowhere, or back into the data folder", async () => {
    const f = await fixture()
    const dangling = path.join(f.root, "dangling")
    await fs.symlink(path.join(f.root, "absent"), dangling)
    await expect(f.move(null, dangling)).rejects.toThrow(/link to a folder/)
    const back = path.join(f.root, "back")
    await fs.symlink(f.source, back, "junction")
    await expect(f.move(null, back)).rejects.toThrow(/nest/)
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("rejects an interrupted destination through advisory and direct calls", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, ".gdl_migration_in_progress"), "")
    await fs.writeFile(path.join(f.target, "partial"), "partial")
    expect(await f.validate(null, f.target)).toBe("other_incomplete_migration")
    await expect(f.move(null, f.target, true)).rejects.toThrow(
      /incomplete|migration/i
    )
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("preserves a source changed after copy", async () => {
    const f = await fixture()
    const outcome = await relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop,
      progress: (p) => {
        if (p.action === "remove")
          fss.writeFileSync(path.join(f.source, "save"), "new save")
      }
    })
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "new save"
    )
    expect(outcome.changed).toEqual(["save"])
    expect(outcome.notDeleted).toEqual([])
    expect(outcome.warnings).toHaveLength(1)
    expect(outcome.warnings[0]).toContain("changed after it was copied")
  })
  it("preserves pointer authority after a partial temporary write failure", async () => {
    const f = await fixture()
    const open = fs.open.bind(fs)
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      const handle = await (open as any)(file, ...args)
      if (String(file).includes(".runtime_path_override.")) {
        handle.writeFile = async () => {
          await handle.write("partial")
          throw new Error("ENOSPC")
        }
      }
      return handle
    }) as any)
    await expect(f.move(null, f.target)).rejects.toThrow("ENOSPC")
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
  })
  it("moves ordinary saves to an empty sibling", async () => {
    const f = await fixture()
    await f.move(null, f.target)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
  })
})

// Real filesystem operations; only the indicated fault boundary is injected.
// Windows skips directory syncs in the relocation code, so these faults can't fire there.
const syncsDirectories = process.platform !== "win32"

describe("runtime relocation transaction controls", () => {
  it.each(["absolute", "relative", "dangling", "directory"])(
    "rejects %s links before shutdown",
    async (kind) => {
      const f = await fixture()
      await fs.mkdir(path.join(f.root, "external"))
      const link =
        kind === "directory"
          ? path.join(f.source, "nested", "world")
          : path.join(f.source, "world")
      await fs.mkdir(path.dirname(link), { recursive: true })
      await fs.symlink(
        kind === "relative"
          ? "../external"
          : kind === "dangling"
            ? path.join(f.root, "absent")
            : path.join(f.root, "external"),
        link
      )
      await expect(f.move(null, f.target)).rejects.toThrow(/Unsupported/)
      expect(f.stop).not.toHaveBeenCalled()
      expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
        "old save"
      )
    }
  )
  it("preserves empty directories and nested control-name user files", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "empty", "inner"), { recursive: true })
    await fs.mkdir(path.join(f.source, "world"))
    await fs.writeFile(
      path.join(f.source, "world", "runtime_path_override"),
      "user content"
    )
    await f.move(null, f.target)
    expect(
      (await fs.stat(path.join(f.target, "empty", "inner"))).isDirectory()
    ).toBe(true)
    expect(
      await fs.readFile(
        path.join(f.target, "world", "runtime_path_override"),
        "utf8"
      )
    ).toBe("user content")
    // Moved, not copied: the old folder is emptied, its own subfolders included.
    expect(await fs.readdir(f.source)).toEqual([])
  })
  it("direct IPC cannot select marked partial data", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, MIGRATION_MARKER), "")
    await expect(f.move(null, f.target, true)).rejects.toThrow(/Incomplete/)
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("switches to complete populated data without deleting either tree", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    expect(await f.validate(null, f.target)).toBe("potentially_valid")
    await f.move(null, f.target, true)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readFile(path.join(f.target, "existing"), "utf8")).toBe(
      "complete"
    )
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
  })
  it("rejects a stale move flag for populated targets", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    await expect(f.move(null, f.target, false)).rejects.toThrow(/populated/)
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("never copies when admission or observed exit fails", async () => {
    const f = await fixture()
    f.stop.mockRejectedValue(new Error("busy or unknown exit"))
    await expect(f.move(null, f.target)).rejects.toThrow(/busy/)
    expect(await fs.readdir(f.target)).toEqual([])
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
  })
  it("rechecks destination after shutdown", async () => {
    const f = await fixture()
    f.stop.mockImplementation(async () => {
      await fs.writeFile(path.join(f.target, "new"), "foreign")
    })
    await expect(f.move(null, f.target)).rejects.toThrow(/populated/)
    expect(await fs.readFile(path.join(f.target, "new"), "utf8")).toBe(
      "foreign"
    )
  })
  it("rejects canonical nesting through an ancestor alias", async () => {
    const f = await fixture()
    const alias = path.join(f.root, "alias")
    await fs.symlink(f.source, alias)
    await expect(f.move(null, path.join(alias, "child"))).rejects.toThrow(
      /nest/
    )
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("inventory propagates enumeration errors", async () => {
    const f = await fixture()
    const read = fs.readdir.bind(fs)
    vi.spyOn(fs, "readdir").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (String(file) === f.source) throw new Error("EACCES fixture")
      return (read as any)(file, ...args)
    }) as any)
    await expect(f.move(null, f.target)).rejects.toThrow(/EACCES/)
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("marker creation failure prevents first copy", async () => {
    const f = await fixture()
    const open = fs.open.bind(fs)
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (String(file).endsWith(MIGRATION_MARKER))
        throw new Error("marker EACCES")
      return (open as any)(file, ...args)
    }) as any)
    await expect(f.move(null, f.target)).rejects.toThrow(/marker EACCES/)
    expect(await fs.readdir(f.target)).toEqual([])
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
  })
  it("copy failure removes the partial copy, so the folder can be used again", async () => {
    const f = await fixture()
    const copy = fs.copyFile.bind(fs)
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      await (copy as any)(...args)
      throw new Error("copy interrupted")
    }) as any)
    expect(await f.rawMove(null, f.target)).toMatchObject({
      type: "failed",
      recovery: "restart",
      // Shown exactly as the underlying failure.
      message: "Error: copy interrupted",
      incompleteTarget: false,
      copyRemoved: true
    })
    // The chosen folder is as it was: empty, with no marker.
    expect(await fs.readdir(f.target)).toEqual([])
    expect(await f.validate(null, f.target)).toBe("valid")
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
    vi.mocked(fs.copyFile).mockRestore()
    await f.move(null, f.target)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
  })
  it("a partial copy that cannot be removed keeps its marker and refuses a retry", async () => {
    const f = await fixture()
    const copy = fs.copyFile.bind(fs),
      unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      await (copy as any)(...args)
      throw new Error("copy interrupted")
    }) as any)
    // Windows refuses to delete a file another program holds open.
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (String(file) === path.join(f.target, "save"))
        throw Object.assign(new Error("EBUSY: resource busy or locked"), {
          code: "EBUSY"
        })
      return unlink(file)
    })
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({
      type: "failed",
      message: "Error: copy interrupted",
      incompleteTarget: true
    })
    expect(result.copyRemoved).toBeUndefined()
    // The marker names the folder it copies, so a later check knows it is only a copy.
    expect(
      JSON.parse(
        await fs.readFile(path.join(f.target, MIGRATION_MARKER), "utf8")
      )
    ).toEqual({ version: 1, source: f.source })
    expect(await f.validate(null, f.target)).toBe("incomplete_migration")
    await expect(f.move(null, f.target, true)).rejects.toThrow(/Incomplete/)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })
  it("marker removal failure preserves prior authority", async () => {
    const f = await fixture()
    const unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (String(file).endsWith(MIGRATION_MARKER))
        throw new Error("marker removal EACCES")
      return unlink(file)
    })
    await expect(f.move(null, f.target)).rejects.toThrow(/marker removal/)
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect(await f.validate(null, f.target)).toBe("incomplete_migration")
  })
  it.each(["open", "write", "sync", "close", "rename"])(
    "preserves old pointer at %s failure",
    async (stage) => {
      for (const switchOnly of [false, true])
        for (const absent of [false, true]) {
          const f = await fixture()
          if (absent) await fs.unlink(f.pointer)
          if (switchOnly)
            await fs.writeFile(path.join(f.target, "existing"), "complete")
          const open = fs.open.bind(fs)
          vi.spyOn(fs, "open").mockImplementation((async (
            file: any,
            ...args: any[]
          ) => {
            const temporary = String(file).includes(".runtime_path_override.")
            if (temporary && stage === "open") throw new Error("fault open")
            const handle = await (open as any)(file, ...args)
            if (temporary && stage === "write")
              handle.writeFile = async () => {
                await handle.write("partial")
                throw new Error("fault write")
              }
            if (temporary && stage === "sync")
              handle.sync = async () => {
                throw new Error("fault sync")
              }
            if (temporary && stage === "close") {
              const close = handle.close.bind(handle)
              handle.close = async () => {
                await close()
                throw new Error("fault close")
              }
            }
            return handle
          }) as any)
          if (stage === "rename")
            vi.spyOn(fs, "rename").mockRejectedValue(new Error("fault rename"))
          await expect(f.move(null, f.target, switchOnly)).rejects.toThrow(
            /fault/
          )
          expect(selectRuntimePath(null, f.root)).toBe(
            absent ? path.join(f.root, "data") : f.source
          )
          expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
            "old save"
          )
          if (!absent)
            expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
          vi.restoreAllMocks()
        }
    }
  )
  it.runIf(syncsDirectories)(
    "postcommit sync failure retains selected destination and source",
    async () => {
      const f = await fixture()
      const open = fs.open.bind(fs)
      vi.spyOn(fs, "open").mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        const handle = await (open as any)(file, ...args)
        if (file === f.root)
          handle.sync = async () => {
            throw new Error("directory sync")
          }
        return handle
      }) as any)
      const outcome = await relocateRuntime({
        source: f.source,
        target: f.target,
        pointer: f.pointer,
        stopCore: f.stop
      })
      expect(outcome.warnings).toHaveLength(1)
      expect(outcome.keptOldFolder).toBe(true)
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
      expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
        "old save"
      )
      expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
        "old save"
      )
    }
  )
  it("source replacement with directory is preserved without recursive deletion", async () => {
    const f = await fixture()
    await relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop,
      progress: (p) => {
        if (p.action === "remove") {
          fss.unlinkSync(path.join(f.source, "save"))
          fss.mkdirSync(path.join(f.source, "save"))
          fss.writeFileSync(path.join(f.source, "save", "new"), "valuable")
        }
      }
    })
    expect(await fs.readFile(path.join(f.source, "save", "new"), "utf8")).toBe(
      "valuable"
    )
  })
  it("parses mounted descendants with escapes and component boundaries", () => {
    const table =
      "1 0 0:1 / /run/media/user/disk rw - ext4 /dev/test rw\n2 0 0:2 / /run/media/user/My\\040Disk rw - ext4 /dev/test2 rw"
    expect(removableMountFor("/run/media/user/disk/gdl", table)).toBe(
      "/run/media/user/disk"
    )
    expect(removableMountFor("/run/media/user/My Disk/gdl", table)).toBe(
      "/run/media/user/My Disk"
    )
    expect(
      removableMountFor("/run/media/user/disk-other/gdl", table)
    ).toBeUndefined()
    expect(removableMountFor("/run/media/user/disk", table)).toBeUndefined()
  })
  it("rejects missing removable mounts before creating host directories", async () => {
    await expect(
      validateRuntimeDestination("/run/media/task04-missing/disk/gdl", null, {
        platform: "linux",
        mountInfo: ""
      })
    ).rejects.toThrow(/not mounted/)
  })
  it("rejects installations inside home before home allowance", async () => {
    const f = await fixture()
    await expect(
      validateRuntimeDestination(f.target, f.source, {
        homePath: f.root,
        installationRoots: [f.target]
      })
    ).rejects.toThrow(/installation/)
  })
  it("excludes concurrent operations and allows later reentry", async () => {
    let release!: () => void
    const pending = withRuntimeOperation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        })
    )
    await expect(withRuntimeOperation(async () => {})).rejects.toThrow(
      /still finishing another task/
    )
    release()
    await pending
    await expect(withRuntimeOperation(async () => 42)).resolves.toBe(42)
    // A brief step such as removing old recovery copies holds it as well, and
    // the refusal does not claim a move is running.
    const brief = withRuntimeOperation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        }),
      { brief: true }
    )
    await expect(withRuntimeOperation(async () => {})).rejects.toThrow(
      /^GDLauncher is still finishing another task\. Try again when it is done\.$/
    )
    release()
    await brief
  })
})

// Removable mounts under /run/media are only looked up on Linux.
it.skipIf(process.platform !== "linux")(
  "validated mounted storage agrees with advisory path validation",
  async () => {
    const f = await fixture()
    const mount = "/run/media/task04-fixture/disk"
    const virtualTarget = mount + "/target"
    const translate = (file: any) =>
      typeof file === "string" && file.startsWith(mount)
        ? f.root + file.slice(mount.length)
        : file
    const lstat = fs.lstat.bind(fs),
      stat = fs.stat.bind(fs),
      access = fs.access.bind(fs),
      readdir = fs.readdir.bind(fs),
      realpath = fs.realpath.bind(fs)
    vi.spyOn(fs, "lstat").mockImplementation(((file: any, ...args: any[]) =>
      (lstat as any)(translate(file), ...args)) as any)
    vi.spyOn(fs, "stat").mockImplementation(((file: any, ...args: any[]) =>
      (stat as any)(translate(file), ...args)) as any)
    vi.spyOn(fs, "access").mockImplementation((file: any, mode: any) =>
      access(translate(file), mode)
    )
    vi.spyOn(fs, "readdir").mockImplementation(((file: any, ...args: any[]) =>
      (readdir as any)(translate(file), ...args)) as any)
    vi.spyOn(fs, "realpath").mockImplementation((async (file: any) => {
      await realpath(translate(file))
      return file
    }) as any)
    const context = { mountInfo: `1 0 0:1 / ${mount} rw - ext4 fixture rw` }
    const advisory = callback("validateRuntimePath", {
      ...f.globals,
      runtimeDestinationContext: () => context
    })
    expect(await advisory(null, virtualTarget)).toBe("valid")
    expect(
      await validateRuntimeDestination(virtualTarget, f.source, context)
    ).toBe("valid")
    await expect(
      validateRuntimeDestination(mount, f.source, context)
    ).rejects.toThrow(/not mounted|root/)
  }
)
// Removable mounts under /run/media are only looked up on Linux.
describe.skipIf(process.platform !== "linux")(
  "a link to a folder on a removable drive",
  () => {
    const mount = "/run/media/task04-fixture/disk"
    /** A link in the data folder's parent, and `at` resolving to `destination`. */
    async function linked(destination: string, at?: string) {
      const f = await fixture()
      // Stands for what the link resolves to: nothing is mounted here in a test.
      const link = path.join(f.root, "link")
      await fs.symlink(f.target, link, "junction")
      const translate = (file: any) =>
        typeof file === "string" && file.startsWith(mount)
          ? f.root + file.slice(mount.length)
          : file
      const stat = fs.stat.bind(fs),
        access = fs.access.bind(fs),
        realpath = fs.realpath.bind(fs)
      vi.spyOn(fs, "stat").mockImplementation(((file: any, ...args: any[]) =>
        (stat as any)(translate(file), ...args)) as any)
      vi.spyOn(fs, "access").mockImplementation((file: any, mode: any) =>
        access(translate(file), mode)
      )
      vi.spyOn(fs, "realpath").mockImplementation((async (file: any) =>
        file === (at ?? link) ? destination : realpath(file)) as any)
      return { f, link }
    }
    const mounted = { mountInfo: `1 0 0:1 / ${mount} rw - ext4 fixture rw` }
    it("is accepted and checked by where it really is", async () => {
      const { f, link } = await linked(`${mount}/gdl`)
      expect(await validateRuntimeDestination(link, f.source, mounted)).toBe(
        "valid"
      )
    })
    it("says the drive is not mounted when it is not", async () => {
      const { f, link } = await linked(`${mount}/gdl`)
      await expect(
        validateRuntimeDestination(link, f.source, { mountInfo: "" })
      ).rejects.toThrow(/not mounted/)
    })
    it("is refused when it leads to the drive's root", async () => {
      const { f, link } = await linked(mount)
      await expect(
        validateRuntimeDestination(link, f.source, mounted)
      ).rejects.toThrow(/not mounted|root/)
    })
    it("elsewhere under /run stays a system directory", async () => {
      const { f, link } = await linked("/run/user/1000/gdl")
      await expect(
        validateRuntimeDestination(link, f.source, mounted)
      ).rejects.toThrow(/system directory/)
    })
    it("from the drive to a system directory is refused by where it really goes", async () => {
      const onDrive = `${mount}/link`
      const { f } = await linked("/etc/gdl", onDrive)
      await expect(
        validateRuntimeDestination(onDrive, f.source, mounted)
      ).rejects.toThrow(/system directory/)
    })
  }
)
it("changes during copy leave the source and old authority intact and remove the copy", async () => {
  const f = await fixture()
  const copy = fs.copyFile.bind(fs)
  vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
    await (copy as any)(...args)
    await fs.writeFile(path.join(f.source, "save"), "new save")
  }) as any)
  await expect(f.move(null, f.target)).rejects.toThrow(/changed during copy/)
  expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
    "new save"
  )
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  expect(await fs.readdir(f.target)).toEqual([])
  expect(await f.validate(null, f.target)).toBe("valid")
})
it("complete unselected crash-stage destination remains switchable", async () => {
  const f = await fixture()
  await fs.writeFile(path.join(f.target, "save"), "complete snapshot")
  // Persistent state after marker removal but before pointer rename.
  expect(selectRuntimePath(null, f.root)).toBe(f.source)
  await f.move(null, f.target, true)
  expect(selectRuntimePath(null, f.root)).toBe(f.target)
  expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
    "old save"
  )
})
it("explicit runtime overrides reject before core shutdown", async () => {
  const f = await fixture()
  const move = callback("changeRuntimePath", {
    ...f.globals,
    overrideCLIDataPath: { value: f.source }
  })
  await expect(move(null, f.target)).rejects.toThrow(/explicit runtime/)
  expect(f.stop).not.toHaveBeenCalled()
})

describe("a move refused by the core", () => {
  const blockers = [
    { kind: "game", name: "game 3" },
    { kind: "server", name: "server 7 run" }
  ]
  it("is dismissible, lists the blockers and leaves the pointer and both folders untouched", async () => {
    const f = await fixture()
    f.stop.mockRejectedValue(
      new ShutdownRefusedError("Core relocation refused", blockers as any)
    )
    const move = callback("changeRuntimePath", f.globals)
    await expect(move(null, f.target)).resolves.toEqual({
      type: "failed",
      recovery: "none",
      message: expect.stringMatching(/refused/),
      incompleteTarget: false,
      refused: { blockers }
    })
    expect(f.stop).toHaveBeenCalledOnce()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    expect(await fs.readdir(f.target)).toEqual([])
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
    expect(f.relaunch).not.toHaveBeenCalled()
  })
  it("a lease released before commit is dismissible without a blocker list", async () => {
    const f = await fixture()
    f.stop.mockRejectedValue(
      new ShutdownRefusedError("Core runtime differs from the selected source")
    )
    const move = callback("changeRuntimePath", f.globals)
    await expect(move(null, f.target)).resolves.toMatchObject({
      type: "failed",
      recovery: "none",
      refused: { blockers: [] }
    })
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
  })
  it("an unverified stop still keeps the move protected", async () => {
    const f = await fixture()
    f.stop.mockRejectedValue(new Error("core unreachable"))
    const move = callback("changeRuntimePath", f.globals)
    const result = await move(null, f.target)
    expect(result).toMatchObject({
      type: "failed",
      recovery: "shutdown_unresolved"
    })
    expect(result.refused).toBeUndefined()
  })
  it("is refused before the data folder is scanned and before the core is stopped", async () => {
    const f = await fixture()
    f.check.mockRejectedValue(
      new ShutdownRefusedError("Core relocation refused", blockers as any)
    )
    const lstat = vi.spyOn(fs, "lstat")
    const move = callback("changeRuntimePath", f.globals)
    await expect(move(null, f.target)).resolves.toEqual({
      type: "failed",
      recovery: "none",
      message: expect.stringMatching(/refused/),
      incompleteTarget: false,
      refused: { blockers }
    })
    // The whole-folder scan is an lstat of every entry, starting at the root.
    expect(lstat.mock.calls.map(([entry]) => String(entry))).not.toContain(
      path.join(f.source, "save")
    )
    expect(f.stop).not.toHaveBeenCalled()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
  })
  it("checks the core first, scans while nothing is stopped, then commits and stops it", async () => {
    const f = await fixture()
    const steps: string[] = []
    f.check.mockImplementation(async () => {
      steps.push("check core")
    })
    f.stop.mockImplementation(async () => {
      steps.push("stop core")
    })
    await relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      checkCore: () => f.check(undefined),
      stopCore: f.stop,
      progress: (p) => {
        if (steps.at(-1) !== p.action) steps.push(p.action)
      }
    })
    // From "scan" on, closing the app no longer ends the move, so the UI
    // shows its do-not-close warning before the core is asked to stop.
    expect(steps.slice(0, 4)).toEqual([
      "check core",
      "check",
      "scan",
      "stop core"
    ])
  })
  it("a check that fails otherwise ends the move while the core runs on, so it stays dismissible", async () => {
    const f = await fixture()
    f.check.mockRejectedValue(new Error("Relocation requires a live core"))
    const move = callback("changeRuntimePath", f.globals)
    const result = await move(null, f.target)
    expect(result).toMatchObject({ type: "failed", recovery: "none" })
    expect(result.refused).toBeUndefined()
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("a core found gone by the check before the scan asks for a restart", async () => {
    const f = await fixture()
    f.check.mockRejectedValue(new Error("Relocation requires a live core"))
    const move = callback("changeRuntimePath", {
      ...f.globals,
      coreProcessHandle: { exitCode: 1, signalCode: null }
    })
    await expect(move(null, f.target)).resolves.toMatchObject({
      type: "failed",
      recovery: "restart"
    })
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("a core that already exited asks for a restart even when it refused", async () => {
    const f = await fixture()
    f.stop.mockRejectedValue(new ShutdownRefusedError("refused", []))
    const move = callback("changeRuntimePath", {
      ...f.globals,
      coreProcessHandle: { exitCode: 1, signalCode: null }
    })
    await expect(move(null, f.target)).resolves.toMatchObject({
      type: "failed",
      recovery: "restart"
    })
  })
})

describe("closing the app while a move only checks the data folder", () => {
  const closed = () =>
    new Error("GDLauncher was closed before the move started")
  it("ends the move mid-scan before the core is stopped, leaving everything as it was", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "instances"))
    await fs.writeFile(path.join(f.source, "instances", "world"), "world")
    const lstat = fs.lstat.bind(fs)
    const visited: string[] = []
    vi.spyOn(fs, "lstat").mockImplementation((async (
      entry: any,
      ...rest: any[]
    ) => {
      if (String(entry).startsWith(f.source)) {
        visited.push(String(entry))
        // The user closes the window once the scan has started.
        if (visited.length === 2)
          expect(interruptRuntimeOperation(closed())).toBe(true)
      }
      return (lstat as any)(entry, ...rest)
    }) as any)
    const move = callback("changeRuntimePath", f.globals)
    const result = await move(null, f.target)
    expect(result).toEqual({
      type: "failed",
      recovery: "none",
      message: expect.stringMatching(/closed before the move started/),
      incompleteTarget: false,
      refused: { blockers: [] }
    })
    // The scan stopped at the next entry instead of walking the whole folder.
    expect(visited.length).toBeLessThan(4)
    expect(f.stop).not.toHaveBeenCalled()
    expect(f.relaunch).not.toHaveBeenCalled()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    expect(await fs.readdir(f.target)).toEqual([])
    expect(runtimeOperationActive()).toBe(false)
  })
  it("ends the move when the close comes while the core is being asked", async () => {
    const f = await fixture()
    f.check.mockImplementation(async () => {
      interruptRuntimeOperation(closed())
    })
    const move = callback("changeRuntimePath", f.globals)
    await expect(move(null, f.target)).resolves.toMatchObject({
      type: "failed",
      recovery: "none",
      refused: { blockers: [] }
    })
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("does not claim nothing is held when an earlier attempt may still hold a lease", async () => {
    const f = await fixture()
    f.check.mockImplementation(async () => {
      interruptRuntimeOperation(closed())
    })
    const move = callback("changeRuntimePath", {
      ...f.globals,
      relocationLeaseHeld: () => true
    })
    const result = await move(null, f.target)
    expect(result).toMatchObject({ type: "failed", recovery: "none" })
    // Without `refused` the renderer keeps an earlier attempt's protection.
    expect(result.refused).toBeUndefined()
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("cannot end a move once the core is asked to stop; it runs to its end", async () => {
    const f = await fixture()
    f.stop.mockImplementation(async () => {
      expect(interruptRuntimeOperation(closed())).toBe(false)
    })
    await f.move(null, f.target)
    expect(f.relaunch).toHaveBeenCalledOnce()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
  })
  it("runs a deferred quit once the operation ends, and at once when none runs", async () => {
    const order: string[] = []
    whenRuntimeOperationIdle(() => order.push("idle now"))
    expect(order).toEqual(["idle now"])
    expect(interruptRuntimeOperation(closed())).toBe(false)
    let release!: () => void
    const pending = withRuntimeOperation(async () => {
      const quit = () => order.push("quit")
      whenRuntimeOperationIdle(quit)
      whenRuntimeOperationIdle(quit)
      await new Promise<void>((resolve) => (release = resolve))
      order.push("operation end")
    })
    release()
    await pending
    await new Promise((resolve) => setTimeout(resolve, 0))
    // Once, and only after the operation.
    expect(order).toEqual(["idle now", "operation end", "quit"])
  })
})

it.each(["relaunch", "deleteDbAndRestart", "restoreDbSnapshotAndRestart"])(
  "excludes %s during relocation and vice versa",
  async (name) => {
    const f = await fixture()
    let release!: () => void
    const occupied = withRuntimeOperation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        })
    )
    const recovery = callback(name, f.globals)
    await expect(
      recovery(null, path.join(f.source, "gdl_conf.pre-downgrade.db"))
    ).rejects.toThrow(/still finishing another task/)
    expect(f.stop).not.toHaveBeenCalled()
    release()
    await occupied
    f.stop.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        })
    )
    const recovering = recovery(null, path.join(f.source, "invalid"))
    await expect(f.move(null, f.target)).rejects.toThrow(
      /still finishing another task/
    )
    release()
    await recovering
  }
)

describe("review transaction boundaries", () => {
  beforeEach(flushFileByFile)
  it.runIf(syncsDirectories)(
    "syncs all new target ancestors before marker completion and pointer commit",
    async () => {
      const f = await fixture(),
        target = path.join(f.root, "new-a", "new-b", "runtime")
      const open = fs.open.bind(fs),
        unlink = fs.unlink.bind(fs),
        rename = fs.rename.bind(fs),
        synced: string[] = []
      let atMarker: string[] = [],
        atCommit: string[] = []
      vi.spyOn(fs, "open").mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        const handle = await (open as any)(file, ...args),
          sync = handle.sync.bind(handle)
        handle.sync = async () => {
          await sync()
          synced.push(String(file))
        }
        return handle
      }) as any)
      vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
        if (String(file) === path.join(target, MIGRATION_MARKER))
          atMarker = [...synced]
        return unlink(file)
      })
      vi.spyOn(fs, "rename").mockImplementation(async (from, to) => {
        if (String(to) === f.pointer) atCommit = [...synced]
        return rename(from, to)
      })
      await f.move(null, target)
      const required = [
        target,
        path.dirname(target),
        path.join(f.root, "new-a"),
        f.root
      ]
      expect(atMarker).toEqual(expect.arrayContaining(required))
      expect(atCommit).toEqual(expect.arrayContaining(required))
      expect(atMarker.indexOf(path.dirname(target))).toBeLessThan(
        atMarker.indexOf(path.join(f.root, "new-a"))
      )
      expect(await fs.readFile(path.join(target, "save"), "utf8")).toBe(
        "old save"
      )
    }
  )
  it.runIf(syncsDirectories).each(["new-a/new-b", "new-a", "."])(
    "target ancestor %s sync fault prevents publication",
    async (ancestor) => {
      const f = await fixture(),
        target = path.join(f.root, "new-a", "new-b", "runtime"),
        open = fs.open.bind(fs)
      vi.spyOn(fs, "open").mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        const handle = await (open as any)(file, ...args)
        if (String(file) === path.resolve(f.root, ancestor))
          handle.sync = async () => {
            throw new Error("ancestor sync fault")
          }
        return handle
      }) as any)
      await expect(f.move(null, target)).rejects.toThrow(/ancestor sync fault/)
      expect(selectRuntimePath(null, f.root)).toBe(f.source)
      expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
        "old save"
      )
      // The copy and the folders made to hold it are removed again.
      expect(fss.existsSync(path.join(f.root, "new-a"))).toBe(false)
      expect(f.relaunch).not.toHaveBeenCalled()
    }
  )
  it.each([false, true])(
    "rejects empty switch-only with shutdown transition=%s",
    async (emptiedDuringShutdown) => {
      const f = await fixture()
      if (emptiedDuringShutdown) {
        await fs.writeFile(path.join(f.target, "existing"), "data")
        f.stop.mockImplementation(async () => {
          await fs.unlink(path.join(f.target, "existing"))
        })
      }
      await expect(f.move(null, f.target, true)).rejects.toThrow(
        /populated|empty/
      )
      expect(f.stop).toHaveBeenCalledTimes(emptiedDuringShutdown ? 1 : 0)
      expect(selectRuntimePath(null, f.root)).toBe(f.source)
      expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
        "old save"
      )
    }
  )
})

// GDL's managed Java (Azul Zulu zips) ships 0444 files such as
// lib/server/classes.jsa; on Windows any file can carry the Read-only attribute.
describe("read-only files in the data folder", () => {
  beforeEach(flushFileByFile)
  const relativeJsa = path.join(
    "managed_javas",
    "zulu",
    "lib",
    "server",
    "classes.jsa"
  )
  async function readOnlyFixture() {
    const f = await fixture()
    const jsa = path.join(f.source, relativeJsa)
    await fs.mkdir(path.dirname(jsa), { recursive: true })
    await fs.writeFile(jsa, "shared archive")
    // On Windows this sets the real Read-only attribute.
    await fs.chmod(jsa, 0o444)
    return { ...f, jsa, copy: path.join(f.target, relativeJsa) }
  }
  // Root opens 0444 files for writing anyway, which would hide the bug.
  it.skipIf(process.getuid?.() === 0)(
    "moves a read-only file and keeps it read-only at the new location",
    async () => {
      const f = await readOnlyFixture()
      await f.move(null, f.target)
      expect(await fs.readFile(f.copy, "utf8")).toBe("shared archive")
      const mode = (await fs.stat(f.copy)).mode
      expect(mode & 0o200).toBe(0)
      if (process.platform !== "win32") expect(mode & 0o777).toBe(0o444)
      expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
      expect(fss.existsSync(path.join(f.target, MIGRATION_MARKER))).toBe(false)
      // The read-only original is deleted like any other, so the move frees its space.
      expect(fss.existsSync(f.jsa)).toBe(false)
      expect(await fs.readdir(f.source)).toEqual([])
    }
  )

  // Windows refuses a write handle on a Read-only file with EPERM, whoever runs it.
  function refuseWriteOpenOfReadOnlyFiles(
    copy: string,
    events: string[],
    syncFault?: Error
  ) {
    const open = fs.open.bind(fs),
      stat = fs.stat.bind(fs),
      chmod = fs.chmod.bind(fs)
    vi.spyOn(fs, "chmod").mockImplementation(async (file, mode) => {
      if (String(file) === copy)
        events.push(Number(mode) & 0o200 ? "chmod writable" : "chmod restore")
      return chmod(file, mode)
    })
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      flags: any,
      ...rest: any[]
    ) => {
      if (flags === "r+" && ((await stat(file)).mode & 0o200) === 0)
        throw Object.assign(
          new Error(`EPERM: operation not permitted, open '${file}'`),
          { code: "EPERM" }
        )
      const handle = await (open as any)(file, flags, ...rest)
      if (String(file) === copy && flags === "r+") {
        events.push("open r+")
        const sync = handle.sync.bind(handle)
        handle.sync = async () => {
          events.push("sync")
          if (syncFault) throw syncFault
          return sync()
        }
      }
      return handle
    }) as any)
  }

  it("grants write access only around the flush when read-only files cannot be opened for writing", async () => {
    const f = await readOnlyFixture()
    const events: string[] = []
    refuseWriteOpenOfReadOnlyFiles(f.copy, events)
    await f.move(null, f.target)
    expect(events).toEqual([
      "chmod writable",
      "open r+",
      "sync",
      "chmod restore"
    ])
    expect(await fs.readFile(f.copy, "utf8")).toBe("shared archive")
    expect((await fs.stat(f.copy)).mode & 0o200).toBe(0)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
  })

  it("restores the read-only mode when the flush fails and removes the unfinished copy", async () => {
    const f = await readOnlyFixture()
    const events: string[] = []
    refuseWriteOpenOfReadOnlyFiles(f.copy, events, new Error("flush fault"))
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({
      type: "failed",
      recovery: "restart",
      incompleteTarget: false,
      copyRemoved: true,
      message: expect.stringMatching(/flush fault/)
    })
    expect(events).toEqual([
      "chmod writable",
      "open r+",
      "sync",
      "chmod restore"
    ])
    // The read-only copy is removed like any other.
    expect(await fs.readdir(f.target)).toEqual([])
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect(await fs.readFile(f.jsa, "utf8")).toBe("shared archive")
    expect((await fs.stat(f.jsa)).mode & 0o200).toBe(0)
    expect(await f.validate(null, f.target)).toBe("valid")
    expect(f.relaunch).not.toHaveBeenCalled()
  })

  it("aborts before commit when the copy's read-only mode cannot be put back", async () => {
    const f = await readOnlyFixture()
    const chmod = fs.chmod.bind(fs)
    vi.spyOn(fs, "chmod").mockImplementation(async (file, mode) => {
      if (String(file) === f.copy && !(Number(mode) & 0o200))
        throw new Error("restore fault")
      return chmod(file, mode)
    })
    await expect(f.move(null, f.target)).rejects.toThrow(/restore fault/)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect(await fs.readFile(f.jsa, "utf8")).toBe("shared archive")
    expect((await fs.stat(f.jsa)).mode & 0o200).toBe(0)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
    expect((await fs.readdir(f.source)).sort()).toEqual([
      "managed_javas",
      "save"
    ])
    expect(await fs.readdir(f.target)).toEqual([])
    expect(f.relaunch).not.toHaveBeenCalled()
  })
})

describe("unfinished moves are named, not just refused", () => {
  const markFrom = (target: string, source: string) =>
    fs.writeFile(
      path.join(target, MIGRATION_MARKER),
      JSON.stringify({ version: 1, source })
    )
  it("advisory validation tells an unfinished move apart from other refusals", async () => {
    const f = await fixture()
    await markFrom(f.target, f.source)
    await fs.writeFile(path.join(f.target, "partial"), "partial")
    expect(await f.validate(null, f.target)).toBe("incomplete_migration")
    const populated = path.join(f.root, "populated")
    await fs.mkdir(populated)
    await fs.writeFile(path.join(populated, "existing"), "complete")
    expect(await f.validate(null, populated)).toBe("potentially_valid")
    const file = path.join(f.root, "not-a-folder")
    await fs.writeFile(file, "")
    expect(await f.validate(null, file)).toBe("not_a_folder")
    expect(await f.validate(null, null)).toBe("invalid")
    await expect(
      validateRuntimeDestination(f.target, f.source)
    ).rejects.toBeInstanceOf(IncompleteMigrationError)
  })
  it("only a partial copy of the folder in use is called deletable", async () => {
    const f = await fixture()
    const marker = path.join(f.target, MIGRATION_MARKER)
    const other = path.join(f.root, "other")
    for (const contents of [
      "", // 2.0.40
      JSON.stringify({ version: 1, source: other }), // copied from another folder
      JSON.stringify({ version: 1, source: "relative" }),
      JSON.stringify({ version: 1 }),
      "not json"
    ]) {
      await fs.writeFile(marker, contents)
      expect(await f.validate(null, f.target), contents).toBe(
        "other_incomplete_migration"
      )
    }
    await fs.rm(marker)
    await fs.mkdir(marker)
    expect(await f.validate(null, f.target)).toBe("other_incomplete_migration")
    await fs.rm(marker, { recursive: true })
    if (process.platform !== "win32") {
      // A link is not followed, even to a marker that names the folder in use.
      const elsewhere = path.join(f.root, "elsewhere")
      await fs.writeFile(
        elsewhere,
        JSON.stringify({ version: 1, source: f.source })
      )
      await fs.symlink(elsewhere, marker)
      expect(await f.validate(null, f.target)).toBe(
        "other_incomplete_migration"
      )
    }
  })
  it("a folder a 2.0.40 user switched into keeps a way out instead of 'Delete it'", async () => {
    // 2.0.40 left empty markers behind after a crash and let users switch into them.
    const f = await fixture()
    const marker = path.join(f.target, MIGRATION_MARKER)
    await fs.writeFile(marker, "")
    await fs.writeFile(path.join(f.target, "gdl_conf.db"), "used as data")
    expect(await f.validate(null, f.target)).toBe("other_incomplete_migration")
    const refused = await f.rawMove(null, f.target, true)
    expect(refused).toMatchObject({
      type: "failed",
      recovery: "none",
      incompleteTarget: false
    })
    expect(refused.message).not.toMatch(/Delete it/)
    expect(f.stop).not.toHaveBeenCalled()
    // The tooltip's way out: remove the marker file, then pick the folder again.
    await fs.rm(marker)
    expect(await f.validate(null, f.target)).toBe("potentially_valid")
    await f.move(null, f.target, true)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(await fs.readFile(path.join(f.target, "gdl_conf.db"), "utf8")).toBe(
      "used as data"
    )
  })
  it("a move refused for an unfinished copy of the folder in use says so without stopping the core", async () => {
    const f = await fixture()
    await markFrom(f.target, f.source)
    expect(await f.rawMove(null, f.target)).toMatchObject({
      type: "failed",
      recovery: "none",
      incompleteTarget: true,
      message: expect.stringMatching(/unfinished copy.*Delete it/)
    })
    expect(f.stop).not.toHaveBeenCalled()
  })
  it("failures that leave no partial copy are not reported as unfinished moves", async () => {
    const populated = await fixture()
    await fs.writeFile(path.join(populated.target, "existing"), "complete")
    expect(await populated.rawMove(null, populated.target)).toMatchObject({
      type: "failed",
      incompleteTarget: false
    })
    const stopped = await fixture()
    stopped.stop.mockRejectedValue(new Error("busy"))
    expect(await stopped.rawMove(null, stopped.target)).toMatchObject({
      type: "failed",
      incompleteTarget: false
    })
    expect(leftPartialCopy(new Error("other"))).toBe(false)
    expect(leftPartialCopy(undefined)).toBe(false)
  })
  it("a path the guard refuses is never probed for a marker", async () => {
    const f = await fixture()
    const probed: string[] = []
    for (const method of ["lstat", "stat", "open", "access"] as const) {
      const original = (fs as any)[method].bind(fs)
      vi.spyOn(fs, method).mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        if (String(file).includes(MIGRATION_MARKER)) probed.push(String(file))
        return original(file, ...args)
      }) as any)
    }
    const windows = callback("changeRuntimePath", {
      ...f.globals,
      runtimeDestinationContext: () => ({ platform: "win32" })
    })
    for (const [move, target] of [
      [f.rawMove, "/"],
      [f.rawMove, path.join(f.source, "nested")],
      [windows, "\\\\host\\share\\GDL"]
    ] as const) {
      expect(await move(null, target)).toMatchObject({
        type: "failed",
        recovery: "none",
        incompleteTarget: false
      })
    }
    expect(probed).toEqual([])
    expect(f.stop).not.toHaveBeenCalled()
  })
})

// 2.0.40 let users switch into a folder an interrupted move had marked, and keep using it.
describe("a data folder in use that still holds an old move's marker", () => {
  for (const [kind, contents] of [
    ["an empty 2.0.40 marker", ""],
    ["a marker naming another folder", "other"]
  ] as const) {
    it(`moves away from a folder with ${kind}, leaving the stale marker behind`, async () => {
      const f = await fixture()
      const stale =
        contents &&
        JSON.stringify({ version: 1, source: path.join(f.root, contents) })
      await fs.writeFile(path.join(f.source, MIGRATION_MARKER), stale)
      expect(await f.rawMove(null, f.target)).toEqual({ type: "relaunching" })
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
      expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
        "old save"
      )
      // The published folder holds no marker: neither the stale one nor this move's.
      expect(await fs.readdir(f.target)).toEqual(["save"])
      // The stale marker is neither copied nor deleted, and is not named as left behind.
      expect(await fs.readdir(f.source)).toEqual([MIGRATION_MARKER])
      expect(
        await fs.readFile(path.join(f.source, MIGRATION_MARKER), "utf8")
      ).toBe(stale)
      expect(
        await callback("takeRuntimeMoveNotice", {
          ...f.globals,
          CURRENT_RUNTIME_PATH: f.target
        })()
      ).toBeNull()
    })
  }
  it("still refuses to switch into a marked folder", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, MIGRATION_MARKER), "")
    await fs.writeFile(path.join(f.target, MIGRATION_MARKER), "")
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    expect(await f.rawMove(null, f.target, true)).toMatchObject({
      type: "failed",
      recovery: "none",
      message: expect.stringMatching(/Incomplete migration/)
    })
    expect(f.stop).not.toHaveBeenCalled()
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    // Only the folder in use may hold a stale marker: an inventory of any other refuses it.
    await expect(inventoryRuntimeTree(f.target)).rejects.toThrow(
      /Incomplete migration in source/
    )
    expect(
      await inventoryRuntimeTree(f.target, undefined, { liveSource: true })
    ).toEqual([{ relative: "existing", directory: false }])
  })
  it("a marker that is not a plain file still refuses the move", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, MIGRATION_MARKER))
    await fs.writeFile(path.join(f.source, MIGRATION_MARKER, "inside"), "data")
    await expect(f.move(null, f.target)).rejects.toThrow(
      /Incomplete migration in source/
    )
    expect(f.stop).not.toHaveBeenCalled()
    expect(await fs.readdir(f.target)).toEqual([])
  })
  it("a nested file with the marker's name is ordinary user data and moves", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, MIGRATION_MARKER), "")
    await fs.mkdir(path.join(f.source, "world"))
    await fs.writeFile(path.join(f.source, "world", MIGRATION_MARKER), "mine")
    await f.move(null, f.target)
    expect(
      await fs.readFile(path.join(f.target, "world", MIGRATION_MARKER), "utf8")
    ).toBe("mine")
    expect(fss.existsSync(path.join(f.target, MIGRATION_MARKER))).toBe(false)
  })
})

// 2.0.40 deleted each original after copying it; a move must free the old drive again.
describe("a move frees the old folder", () => {
  async function populated() {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "instances", "pack", "saves", "world"), {
      recursive: true
    })
    await fs.mkdir(path.join(f.source, "empty"))
    await fs.writeFile(
      path.join(f.source, "instances", "pack", "saves", "world", "level.dat"),
      "level"
    )
    await fs.writeFile(
      path.join(f.source, "instances", "pack", "options.txt"),
      "options"
    )
    return f
  }
  const level = path.join("instances", "pack", "saves", "world", "level.dat")
  const options = path.join("instances", "pack", "options.txt")
  const move = (
    f: Awaited<ReturnType<typeof fixture>>,
    progress?: Parameters<typeof relocateRuntime>[0]["progress"]
  ) =>
    relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop,
      progress
    })

  it("a quiescent move copies everything, selects the new folder and empties the old one", async () => {
    const f = await populated()
    expect(await move(f)).toEqual({
      warnings: [],
      changed: [],
      notDeleted: [],
      keptOldFolder: false
    })
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readFile(path.join(f.target, level), "utf8")).toBe("level")
    expect(await fs.readFile(path.join(f.target, options), "utf8")).toBe(
      "options"
    )
    expect((await fs.stat(path.join(f.target, "empty"))).isDirectory()).toBe(
      true
    )
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
    // Empty, with no recovery folder holding a second copy.
    expect(await fs.readdir(f.source)).toEqual([])
  })

  it("an original rewritten after its copy stays with its new bytes and is named; the rest are deleted", async () => {
    const f = await populated()
    let injected = false
    const outcome = await move(f, (p) => {
      // Before the first original is checked, another program saves the world.
      if (p.action === "remove" && !injected) {
        injected = true
        fss.writeFileSync(path.join(f.source, level), "newer level")
      }
    })
    expect(injected).toBe(true)
    expect(outcome.changed).toEqual([level])
    expect(outcome.notDeleted).toEqual([])
    expect(outcome.keptOldFolder).toBe(false)
    expect(outcome.warnings.join("\n")).toContain(path.join(f.source, level))
    expect(await fs.readFile(path.join(f.source, level), "utf8")).toBe(
      "newer level"
    )
    // The new folder keeps the copy made during the move.
    expect(await fs.readFile(path.join(f.target, level), "utf8")).toBe("level")
    expect(fss.existsSync(path.join(f.source, "save"))).toBe(false)
    expect(fss.existsSync(path.join(f.source, options))).toBe(false)
    expect(fss.existsSync(path.join(f.source, "empty"))).toBe(false)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
  })

  // EBUSY/EPERM: Windows, open in another program or Read-only. EACCES: a folder the
  // user may not write, e.g. one created by running the launcher as root.
  it.each(["EBUSY", "EPERM", "EACCES"])(
    "an original that cannot be deleted (%s) stays and is named apart from changed files; the rest are deleted",
    async (code) => {
      const f = await populated()
      const unlink = fs.unlink.bind(fs)
      vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
        if (String(file) === path.join(f.source, options))
          throw Object.assign(
            new Error(
              `${code}: resource busy or locked, unlink '${String(file)}'`
            ),
            { code }
          )
        return unlink(file)
      })
      const outcome = await move(f)
      expect(outcome.notDeleted).toEqual([options])
      expect(outcome.changed).toEqual([])
      expect(outcome.warnings.join("\n")).toContain(code)
      expect(await fs.readFile(path.join(f.source, options), "utf8")).toBe(
        "options"
      )
      expect(fss.existsSync(path.join(f.source, "save"))).toBe(false)
      expect(fss.existsSync(path.join(f.source, level))).toBe(false)
      expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
      expect(await fs.readFile(path.join(f.target, options), "utf8")).toBe(
        "options"
      )
      expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
        "old save"
      )
    }
  )

  it("an original deleted by someone else after its copy is not reported", async () => {
    const f = await populated()
    let injected = false
    const outcome = await move(f, (p) => {
      if (p.action === "remove" && !injected) {
        injected = true
        fss.unlinkSync(path.join(f.source, options))
      }
    })
    expect(outcome).toMatchObject({ changed: [], notDeleted: [] })
    expect(await fs.readdir(f.source)).toEqual([])
  })

  it.runIf(syncsDirectories)(
    "an unconfirmed pointer keeps the whole old folder and creates no recovery folder",
    async () => {
      const f = await populated()
      const before = (await fg("**/*", { cwd: f.source, dot: true })).sort()
      const open = fs.open.bind(fs)
      vi.spyOn(fs, "open").mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        const handle = await (open as any)(file, ...args)
        if (file === f.root)
          handle.sync = async () => {
            throw new Error("directory sync")
          }
        return handle
      }) as any)
      const outcome = await move(f)
      expect(outcome).toMatchObject({
        changed: [],
        notDeleted: [],
        keptOldFolder: true
      })
      expect(outcome.warnings).toEqual([
        expect.stringContaining("directory sync failed")
      ])
      expect((await fg("**/*", { cwd: f.source, dot: true })).sort()).toEqual(
        before
      )
      expect(await fs.readFile(path.join(f.source, level), "utf8")).toBe(
        "level"
      )
      expect(await fs.readFile(f.pointer, "utf8")).toBe(f.target)
    }
  )

  it("the emptied old folder is a valid destination again, so moving back is a real move", async () => {
    const f = await populated()
    await f.move(null, f.target)
    expect(await validateRuntimeDestination(f.source, f.target)).toBe("valid")
    // Settings asks the same question before Reset to default or a move back.
    const back = callback("validateRuntimePath", {
      ...f.globals,
      CURRENT_RUNTIME_PATH: f.target
    })
    expect(await back(null, f.source)).toBe("valid")
  })

  it("the progress counts files and advances through the removal", async () => {
    const f = await populated()
    const seen: { action: string; current: number; total: number }[] = []
    await move(f, ({ action, current, total }) =>
      seen.push({ action, current, total })
    )
    const removals = seen.filter((p) => p.action === "remove")
    // Three files, each copied then removed.
    expect(removals.map((p) => p.current)).toEqual([3, 4, 5])
    // Past the scans (which count entries, or nothing before the core stops).
    const counted = seen.filter(
      (p) => p.action === "copy" || p.action === "remove"
    )
    expect(new Set(counted.map((p) => p.total))).toEqual(new Set([6]))
  })
})

// The remove phase deletes an original only while it holds its copy's bytes. A new inode
// number or change time alone is not a change, so these were kept and listed by mistake.
describe("unchanged originals are deleted even when their inode data moves", () => {
  const move = (
    f: Awaited<ReturnType<typeof fixture>>,
    progress?: Parameters<typeof relocateRuntime>[0]["progress"]
  ) =>
    relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop,
      progress
    })

  it("both names of a hard-linked mod are deleted, and both copies are kept", async () => {
    const f = await fixture()
    const a = path.join("instances", "a", "mods", "sodium.jar"),
      b = path.join("instances", "b", "mods", "sodium.jar")
    await fs.mkdir(path.dirname(path.join(f.source, a)), { recursive: true })
    await fs.mkdir(path.dirname(path.join(f.source, b)), { recursive: true })
    await fs.writeFile(path.join(f.source, a), "jar bytes")
    // Deduplicated: deleting either name changes the other's change time.
    await fs.link(path.join(f.source, a), path.join(f.source, b))
    expect(await move(f)).toMatchObject({ changed: [], notDeleted: [] })
    expect(await fs.readdir(f.source)).toEqual([])
    for (const name of [a, b])
      expect(await fs.readFile(path.join(f.target, name), "utf8")).toBe(
        "jar bytes"
      )
  })

  it("a database restored from its snapshot (two names for one file) is moved whole", async () => {
    const f = await fixture()
    // databaseRestore publishes the staged snapshot as gdl_conf.db with a hard link and
    // keeps its bundle beside it.
    const bundle = "gdl_conf.restore-originals-1b2c"
    await fs.mkdir(path.join(f.source, bundle))
    await fs.writeFile(
      path.join(f.source, bundle, "snapshot-staged.db"),
      "database"
    )
    await fs.link(
      path.join(f.source, bundle, "snapshot-staged.db"),
      path.join(f.source, "gdl_conf.db")
    )
    expect(await move(f)).toMatchObject({ changed: [], notDeleted: [] })
    expect(await fs.readdir(f.source)).toEqual([])
    expect(await fs.readFile(path.join(f.target, "gdl_conf.db"), "utf8")).toBe(
      "database"
    )
    expect(
      await fs.readFile(
        path.join(f.target, bundle, "snapshot-staged.db"),
        "utf8"
      )
    ).toBe("database")
  })

  it("a file given a new inode number after its copy (Linux FAT/exFAT) is deleted", async () => {
    const f = await fixture()
    let removing = false
    const lstat = fs.lstat.bind(fs)
    vi.spyOn(fs, "lstat").mockImplementation((async (file: any) => {
      const st = await lstat(file)
      if (!removing || String(file) !== path.join(f.source, "save")) return st
      // The driver numbered the file again when it was looked up anew.
      return Object.assign(Object.create(Object.getPrototypeOf(st)), st, {
        ino: st.ino + 1
      })
    }) as any)
    const outcome = await move(f, (p) => {
      if (p.action === "remove") removing = true
    })
    expect(outcome).toMatchObject({ changed: [], notDeleted: [] })
    expect(fss.existsSync(path.join(f.source, "save"))).toBe(false)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
  })

  it("a folder swapped for a link into the new folder never leads to deleting the copy", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "world"))
    await fs.writeFile(path.join(f.source, "world", "level.dat"), "level")
    // Copies keep the modification time on Windows; give both the same one here.
    await fs.utimes(path.join(f.source, "world", "level.dat"), 1e6, 1e6)
    let swapped = false
    const outcome = await move(f, (p) => {
      if (p.action !== "remove" || swapped) return
      swapped = true
      fss.utimesSync(path.join(f.target, "world", "level.dat"), 1e6, 1e6)
      fss.renameSync(
        path.join(f.source, "world"),
        path.join(f.root, "world-elsewhere")
      )
      fss.symlinkSync(
        path.join(f.target, "world"),
        path.join(f.source, "world")
      )
    })
    expect(swapped).toBe(true)
    // Same bytes, size and time through the link, but not the old folder's file.
    expect(outcome.changed).toEqual([path.join("world", "level.dat")])
    expect(
      await fs.readFile(path.join(f.target, "world", "level.dat"), "utf8")
    ).toBe("level")
    expect(
      await fs.readFile(
        path.join(f.root, "world-elsewhere", "level.dat"),
        "utf8"
      )
    ).toBe("level")
  })

  it("a file whose bytes changed is still kept, whatever its inode data", async () => {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "mods"))
    await fs.writeFile(path.join(f.source, "mods", "a.jar"), "jar bytes")
    await fs.link(
      path.join(f.source, "mods", "a.jar"),
      path.join(f.source, "mods", "b.jar")
    )
    let rewritten = false
    const outcome = await move(f, (p) => {
      if (p.action !== "remove" || rewritten) return
      rewritten = true
      // Same size and time, other bytes: only the content tells.
      const st = fss.statSync(path.join(f.source, "mods", "a.jar"))
      fss.writeFileSync(path.join(f.source, "mods", "a.jar"), "JAR BYTES")
      fss.utimesSync(path.join(f.source, "mods", "a.jar"), st.atime, st.mtime)
    })
    expect([...outcome.changed].sort()).toEqual([
      path.join("mods", "a.jar"),
      path.join("mods", "b.jar")
    ])
    expect(
      await fs.readFile(path.join(f.source, "mods", "a.jar"), "utf8")
    ).toBe("JAR BYTES")
  })
})

describe("the move's result on the extracted IPC callback", () => {
  const noticeFile = (f: Awaited<ReturnType<typeof fixture>>) =>
    path.join(f.root, RUNTIME_MOVE_NOTICE_NAME)
  // Each start from the new folder asks for the notice.
  const takeNotice = (f: Awaited<ReturnType<typeof fixture>>) =>
    callback("takeRuntimeMoveNotice", {
      ...f.globals,
      CURRENT_RUNTIME_PATH: selectRuntimePath(null, f.root)
    })()
  // The user pressed OK (or the close button) on the notice.
  const dismissNotice = (f: Awaited<ReturnType<typeof fixture>>) =>
    callback("dismissRuntimeMoveNotice", f.globals)()
  // A window closed (or crashed) during the move: its frame is gone.
  const closedWindow = {
    sender: { isDestroyed: () => true, send: vi.fn() },
    senderFrame: null
  }
  function keepOpen(file: string) {
    const unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "unlink").mockImplementation(async (name) => {
      if (String(name) === file)
        throw Object.assign(new Error("EBUSY: resource busy or locked"), {
          code: "EBUSY"
        })
      return unlink(name)
    })
  }

  it("restarts at once, with no notice, when the old folder was emptied", async () => {
    const f = await fixture()
    expect(await f.rawMove(null, f.target)).toEqual({ type: "relaunching" })
    expect(f.dialog.showMessageBox).not.toHaveBeenCalled()
    expect(f.relaunch).toHaveBeenCalledTimes(1)
    expect(await fs.readdir(f.source)).toEqual([])
    expect(await takeNotice(f)).toBeNull()
  })

  it("restarts at once even with files left behind, and names them on the first start from the new folder", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, "servers.dat"), "servers")
    keepOpen(path.join(f.source, "servers.dat"))
    // The window that asked was closed while the files were copied.
    expect(await f.rawMove(closedWindow, f.target)).toEqual({
      type: "relaunching"
    })
    // Main restarts by itself, as 2.0.40 did: nothing waits on a window.
    expect(f.relaunch).toHaveBeenCalledTimes(1)
    expect(f.dialog.showMessageBox).not.toHaveBeenCalled()
    expect(closedWindow.sender.send).not.toHaveBeenCalled()
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(fss.existsSync(path.join(f.source, "save"))).toBe(false)
    expect(await takeNotice(f)).toEqual({
      oldFolder: f.source,
      newFolder: f.target,
      changed: [],
      changedCount: 0,
      notDeleted: ["servers.dat"],
      notDeletedCount: 1,
      keptOldFolder: false
    })
    // A reload, or a modal that never opened, shows it again.
    expect(await takeNotice(f)).toMatchObject({ notDeleted: ["servers.dat"] })
    // Gone once the user acknowledges it.
    await dismissNotice(f)
    expect(await takeNotice(f)).toBeNull()
    expect(fss.existsSync(noticeFile(f))).toBe(false)
  })

  it("keeps the notice for every start until the user dismisses it", async () => {
    const f = await fixture()
    const notice = {
      oldFolder: f.source,
      newFolder: f.target,
      changed: ["saves/world/level.dat"],
      changedCount: 1,
      notDeleted: [],
      notDeletedCount: 0,
      keptOldFolder: false
    }
    await recordRuntimeMoveNotice(f.root, notice)
    for (let start = 0; start < 3; start++)
      expect(await takeRuntimeMoveNotice(f.root, f.target)).toEqual(notice)
    await dismissRuntimeMoveNotice(f.root)
    expect(await takeRuntimeMoveNotice(f.root, f.target)).toBeNull()
    // Dismissing again, with nothing left, is not an error.
    await dismissRuntimeMoveNotice(f.root)
    // One that cannot be removed is only logged, and shown again next start.
    await recordRuntimeMoveNotice(f.root, notice)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    vi.spyOn(fs, "unlink").mockRejectedValue(
      Object.assign(new Error("EPERM: operation not permitted"), {
        code: "EPERM"
      })
    )
    await dismissRuntimeMoveNotice(f.root)
    expect(warn.mock.calls.flat().join("\n")).toContain("EPERM")
    vi.restoreAllMocks()
    expect(await takeRuntimeMoveNotice(f.root, f.target)).toEqual(notice)
  })

  it("names a file that changed after its copy apart from one that could not be deleted", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, "servers.dat"), "servers")
    keepOpen(path.join(f.source, "servers.dat"))
    const relocate = vi.fn((options: Parameters<typeof relocateRuntime>[0]) =>
      relocateRuntime({
        ...options,
        progress: (p) => {
          if (p.action === "remove" && p.currentName === "save")
            fss.writeFileSync(path.join(f.source, "save"), "newer save")
        }
      })
    )
    const move = callback("changeRuntimePath", {
      ...f.globals,
      relocateRuntime: relocate
    })
    expect(await move(null, f.target)).toEqual({ type: "relaunching" })
    expect(await takeNotice(f)).toMatchObject({
      changed: ["save"],
      changedCount: 1,
      notDeleted: ["servers.dat"],
      notDeletedCount: 1
    })
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "newer save"
    )
  })

  it("names at most LEFT_BEHIND_SHOWN files and counts the rest", async () => {
    const f = await fixture()
    for (let i = 0; i < LEFT_BEHIND_SHOWN + 5; i++)
      await fs.writeFile(path.join(f.source, `file-${i}`), String(i))
    const unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (!String(file).startsWith(f.source + path.sep)) return unlink(file)
      throw Object.assign(new Error(`EPERM: unlink '${String(file)}'`), {
        code: "EPERM"
      })
    })
    expect(await f.rawMove(null, f.target)).toEqual({ type: "relaunching" })
    expect(f.relaunch).toHaveBeenCalledTimes(1)
    const notice = await takeNotice(f)
    expect(notice).toMatchObject({
      notDeletedCount: LEFT_BEHIND_SHOWN + 6,
      keptOldFolder: false
    })
    expect(notice.notDeleted).toHaveLength(LEFT_BEHIND_SHOWN)
  })

  it.runIf(syncsDirectories)(
    "says after the restart that the old folder was kept when the new location is unconfirmed",
    async () => {
      const f = await fixture()
      const open = fs.open.bind(fs)
      vi.spyOn(fs, "open").mockImplementation((async (
        file: any,
        ...args: any[]
      ) => {
        const handle = await (open as any)(file, ...args)
        if (file === f.root)
          handle.sync = async () => {
            throw new Error("directory sync")
          }
        return handle
      }) as any)
      expect(await f.rawMove(null, f.target)).toEqual({ type: "relaunching" })
      expect(f.relaunch).toHaveBeenCalledTimes(1)
      expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
        "old save"
      )
      vi.restoreAllMocks()
      expect(await takeNotice(f)).toEqual({
        oldFolder: f.source,
        newFolder: f.target,
        changed: [],
        changedCount: 0,
        notDeleted: [],
        notDeletedCount: 0,
        keptOldFolder: true
      })
    }
  )

  it("restarts even when the notice cannot be kept", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, "servers.dat"), "servers")
    keepOpen(path.join(f.source, "servers.dat"))
    const writeFile = fs.writeFile.bind(fs)
    vi.spyOn(fs, "writeFile").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (String(file).includes(RUNTIME_MOVE_NOTICE_NAME))
        throw Object.assign(new Error("ENOSPC: no space left on device"), {
          code: "ENOSPC"
        })
      return (writeFile as any)(file, ...args)
    }) as any)
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(await f.rawMove(null, f.target)).toEqual({ type: "relaunching" })
    expect(f.relaunch).toHaveBeenCalledTimes(1)
    expect(warn.mock.calls.flat().join("\n")).toContain("ENOSPC")
    expect(
      (await fs.readdir(f.root)).filter((n) =>
        n.includes(RUNTIME_MOVE_NOTICE_NAME)
      )
    ).toEqual([])
  })

  it("a later move that empties its old folder forgets an earlier move's notice", async () => {
    const f = await fixture()
    await recordRuntimeMoveNotice(f.root, {
      oldFolder: path.join(f.root, "older"),
      newFolder: f.target,
      changed: ["x"],
      changedCount: 1,
      notDeleted: [],
      notDeletedCount: 0,
      keptOldFolder: false
    })
    await f.rawMove(null, f.target)
    expect(fss.existsSync(noticeFile(f))).toBe(false)
  })

  it("a notice for another data folder waits, and an unreadable one is dropped", async () => {
    const f = await fixture()
    const notice = {
      oldFolder: f.source,
      newFolder: f.target,
      changed: [],
      changedCount: 0,
      notDeleted: ["servers.dat"],
      notDeletedCount: 1,
      keptOldFolder: false
    }
    await recordRuntimeMoveNotice(f.root, notice)
    // An explicit data path override is in effect.
    expect(await takeRuntimeMoveNotice(f.root, f.source)).toBeNull()
    expect(await takeRuntimeMoveNotice(f.root, f.target + path.sep)).toEqual(
      notice
    )
    await fs.writeFile(noticeFile(f), "{not json")
    expect(await takeRuntimeMoveNotice(f.root, f.target)).toBeNull()
    expect(fss.existsSync(noticeFile(f))).toBe(false)
    await fs.writeFile(
      noticeFile(f),
      JSON.stringify({ version: 1, ...notice, notDeleted: [1] })
    )
    expect(await takeRuntimeMoveNotice(f.root, f.target)).toBeNull()
  })

  it("a switch to a folder already in use never touches the old one", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    expect(await f.rawMove(null, f.target, true)).toEqual({
      type: "relaunching"
    })
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await takeNotice(f)).toBeNull()
  })

  it("leftInOldFolder is null only when nothing stayed behind", () => {
    expect(
      leftInOldFolder("/old", "/new", {
        warnings: ["logged only"],
        changed: [],
        notDeleted: [],
        keptOldFolder: false
      })
    ).toBeNull()
  })
})

const enoent = (file: string) =>
  Object.assign(new Error(`ENOENT: no such file or directory, '${file}'`), {
    code: "ENOENT"
  })

/**
 * Serves paths under `virtual` (/media, /run/media: folders this host cannot
 * create) from the real folder `real`, for the calls validation makes, and
 * resolves them to `resolve(file)` as a link or mapped drive would.
 */
function serveVirtual(
  virtual: string,
  real: string,
  options: { unwritable?: string[] } = {}
) {
  const translate = (file: any) =>
    typeof file === "string" &&
    (file === virtual || file.startsWith(virtual + "/"))
      ? real + file.slice(virtual.length)
      : file
  for (const method of ["lstat", "stat", "readdir"] as const) {
    const original = (fs as any)[method].bind(fs)
    vi.spyOn(fs, method).mockImplementation(((file: any, ...args: any[]) =>
      original(translate(file), ...args)) as any)
  }
  const access = fs.access.bind(fs)
  vi.spyOn(fs, "access").mockImplementation(async (file: any, mode: any) => {
    if (options.unwritable?.includes(file))
      throw Object.assign(new Error(`EACCES: permission denied, '${file}'`), {
        code: "EACCES"
      })
    return access(translate(file), mode)
  })
  const realpath = fs.realpath.bind(fs)
  vi.spyOn(fs, "realpath").mockImplementation((async (file: any) => {
    if (translate(file) === file) return realpath(file)
    await realpath(translate(file))
    return file
  }) as any)
}

// Folders 2.0.40 accepted as a data folder stay accepted, and a refused one says why.
describe("data folder destinations", () => {
  const mapped = {
    "Z:\\GDL": "\\\\nas\\share\\GDL",
    "Z:\\GDL\\sub": "\\\\nas\\share\\GDL\\sub",
    "Y:\\data": "\\\\nas\\share\\GDL"
  } as Record<string, string>
  // A Windows host with Z: and Y: mapped to \\nas\share; Windows resolves a
  // folder on a mapped drive to its share path.
  function mappedDrives(root: string) {
    const drive = (file: any) => typeof file === "string" && /^[YZ]:/.test(file)
    const realpath = fs.realpath.bind(fs),
      lstat = fs.lstat.bind(fs),
      access = fs.access.bind(fs)
    vi.spyOn(fs, "realpath").mockImplementation((async (file: any) => {
      if (Object.hasOwn(mapped, file)) return mapped[file]
      if (drive(file)) throw enoent(file)
      return String(file).startsWith("C:") ? file : realpath(file)
    }) as any)
    vi.spyOn(fs, "lstat").mockImplementation((async (file: any) => {
      if (/^[YZ]:[\\/]?$/.test(file)) return lstat(root)
      if (drive(file)) throw enoent(file)
      return lstat(file)
    }) as any)
    vi.spyOn(fs, "access").mockImplementation(async (file: any, mode: any) =>
      drive(file) ? undefined : access(file, mode)
    )
  }
  const windows = { platform: "win32" as const, homePath: "C:\\Users\\me" }

  it("accepts a folder on a mapped network drive and still checks the share", async () => {
    const f = await fixture()
    mappedDrives(f.root)
    expect(
      await validateRuntimeDestination(
        "Z:\\GDL",
        "C:\\Users\\me\\AppData\\Roaming\\gdlauncher_carbon\\data",
        windows
      )
    ).toBe("valid")
    // The same share through two drive letters still nests.
    await expect(
      validateRuntimeDestination("Z:\\GDL\\sub", "Y:\\data", windows)
    ).rejects.toMatchObject({ reason: "nested" })
    // A path typed as a share is refused, with a reason Settings can name.
    const typed = await validateRuntimeDestination(
      "\\\\nas\\share\\GDL",
      null,
      windows
    ).catch((e) => e)
    expect(runtimeTargetRefusal(typed)).toBe("network_path")
    const advisory = callback("validateRuntimePath", {
      ...f.globals,
      CURRENT_RUNTIME_PATH: "C:\\Users\\me\\data",
      runtimeDestinationContext: () => windows
    })
    vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(await advisory(null, "Z:\\GDL")).toBe("valid")
    expect(await advisory(null, "\\\\nas\\share\\GDL")).toBe("network_path")
  })

  it.skipIf(process.platform === "win32")(
    "accepts plain folders under /media, drive roots there and drives whose root the user cannot write",
    async () => {
      const f = await fixture()
      const media = path.join(f.root, "media")
      for (const folder of ["games", "user/USB", "user/Data/games"])
        await fs.mkdir(path.join(media, folder), { recursive: true })
      serveVirtual("/media", media, { unwritable: ["/media/user/Data"] })
      const context = {
        platform: "linux" as const,
        mountInfo: [
          "1 0 0:1 / /media/user/USB rw - vfat /dev/sdb1 rw",
          "2 0 0:2 / /media/user/Data rw - ext4 /dev/sdc1 rw"
        ].join("\n")
      }
      for (const target of [
        // A plain folder on the system drive.
        "/media/games/gdl",
        // The root of a mounted drive.
        "/media/user/USB",
        // A folder the user owns on a drive whose root belongs to root.
        "/media/user/Data/games/gdl"
      ])
        expect(
          await validateRuntimeDestination(target, f.source, context),
          target
        ).toBe("valid")
    }
  )

  it.skipIf(process.platform === "win32")(
    "says a drive is not connected when its folder under /media is picked while it is unplugged",
    async () => {
      const f = await fixture()
      const media = path.join(f.root, "media")
      // udisks' folder for the user and an fstab mount point, both root's.
      for (const folder of ["user", "storage", "user/Data"])
        await fs.mkdir(path.join(media, folder), { recursive: true })
      serveVirtual("/media", media, {
        unwritable: [
          "/media",
          "/media/user",
          "/media/storage",
          "/media/user/Data"
        ]
      })
      const linux = (mountInfo: string) => ({
        platform: "linux" as const,
        mountInfo
      })
      for (const target of [
        "/media/user/USB/gdl",
        "/media/storage/gdl",
        "/media/other/gdl"
      ]) {
        const refused = await validateRuntimeDestination(
          target,
          f.source,
          linux("")
        ).catch((e) => e)
        expect(runtimeTargetRefusal(refused), target).toBe(
          "removable_not_mounted"
        )
      }
      // Mounted, its root still belongs to root: that is the reason given.
      const mounted = await validateRuntimeDestination(
        "/media/user/Data/gdl",
        f.source,
        linux("2 0 0:2 / /media/user/Data rw - ext4 /dev/sdc1 rw")
      ).catch((e) => e)
      expect(runtimeTargetRefusal(mounted)).toBe("not_writable")
      // Elsewhere a folder that cannot be written is named as such.
      const darwin = await validateRuntimeDestination(
        "/media/user/USB/gdl",
        f.source,
        { platform: "darwin" }
      ).catch((e) => e)
      expect(runtimeTargetRefusal(darwin)).toBe("not_writable")
    }
  )

  it.skipIf(process.platform === "win32")(
    "says a drive is not connected on macOS when its folder under /Volumes is picked while it is unplugged",
    async () => {
      const f = await fixture()
      const volumes = path.join(f.root, "Volumes")
      // An NTFS drive, which macOS mounts read-only.
      await fs.mkdir(path.join(volumes, "NTFS"), { recursive: true })
      serveVirtual("/Volumes", volumes, {
        unwritable: ["/Volumes", "/Volumes/NTFS"]
      })
      const refusal = async (target: string, platform: NodeJS.Platform) =>
        runtimeTargetRefusal(
          await validateRuntimeDestination(target, f.source, {
            platform
          }).catch((e) => e)
        )
      for (const target of ["/Volumes/USB/GDLauncher", "/Volumes/USB"])
        expect(await refusal(target, "darwin"), target).toBe(
          "removable_not_mounted"
        )
      // A connected drive that cannot be written, and /Volumes itself, are
      // named as folders that cannot be written.
      for (const target of ["/Volumes/NTFS/GDLauncher", "/Volumes"])
        expect(await refusal(target, "darwin"), target).toBe("not_writable")
      // Only macOS mounts drives there.
      expect(await refusal("/Volumes/USB/GDLauncher", "linux")).toBe(
        "not_writable"
      )
    }
  )

  it("says a drive is not connected when not even the root of the folder is there", async () => {
    const f = await fixture()
    const target = path.join(f.root, "drive", "gdl")
    // Windows reports a drive letter no drive has as missing, root included.
    vi.spyOn(fs, "realpath").mockImplementation((async (file: any) => {
      throw enoent(String(file))
    }) as any)
    vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(await f.validate(null, target)).toBe("removable_not_mounted")
    // Gone after the path was resolved: the search for a folder to write in
    // stops at the root.
    vi.mocked(fs.realpath).mockRestore()
    const lstat = fs.lstat.bind(fs)
    let calls = 0
    vi.spyOn(fs, "lstat").mockImplementation((async (file: any) => {
      if (
        String(file) === path.parse(target).root ||
        isInside(target, String(file))
      ) {
        if (++calls > 100) throw new Error("searched forever")
        throw enoent(String(file))
      }
      return lstat(file)
    }) as any)
    expect(await f.validate(null, target)).toBe("removable_not_mounted")
  })

  // Linux paths, which a Windows host cannot resolve.
  it.skipIf(process.platform === "win32")(
    "follows a link from home into a drive mounted under /run/media",
    async () => {
      const f = await fixture()
      const usb = path.join(f.root, "usb")
      await fs.mkdir(usb)
      serveVirtual("/run/media/me/USB", usb)
      const target = path.join(f.root, "link", "gdl")
      const realpath = vi.mocked(fs.realpath).getMockImplementation()!
      vi.mocked(fs.realpath).mockImplementation((async (file: any) =>
        file === target ? "/run/media/me/USB/gdl" : realpath(file)) as any)
      const mounted = {
        platform: "linux" as const,
        homePath: f.root,
        mountInfo: "1 0 0:1 / /run/media/me/USB rw - ext4 /dev/sdb1 rw"
      }
      expect(await validateRuntimeDestination(target, f.source, mounted)).toBe(
        "valid"
      )
      // Unplugged: /run is a system folder, and the reason says to connect it.
      const unplugged = await validateRuntimeDestination(target, f.source, {
        ...mounted,
        mountInfo: ""
      }).catch((e) => e)
      expect(runtimeTargetRefusal(unplugged)).toBe("removable_not_mounted")
      // A drive's own mount point is refused like a drive root.
      const root = await validateRuntimeDestination(
        "/run/media/me/USB",
        f.source,
        mounted
      ).catch((e) => e)
      expect(runtimeTargetRefusal(root)).toBe("filesystem_root")
    }
  )

  it("uses a link to a folder as that folder, and refuses a link to nothing", async () => {
    const f = await fixture()
    const link = path.join(f.root, "link")
    await fs.symlink(f.target, link)
    expect(await f.validate(null, link)).toBe("valid")
    await f.move(null, link)
    expect(selectRuntimePath(null, f.root)).toBe(link)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    const dangling = path.join(f.root, "dangling")
    await fs.symlink(path.join(f.root, "absent"), dangling)
    vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(await f.validate(null, dangling)).toBe("not_a_folder")
    await expect(f.move(null, dangling)).rejects.toThrow(/link to a folder/)
  })

  it("names why a folder is refused and logs the details", async () => {
    const f = await fixture()
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const install = path.join(f.root, "install")
    await fs.mkdir(install)
    const advisory = callback("validateRuntimePath", {
      ...f.globals,
      runtimeDestinationContext: () => ({ installationRoots: [install] })
    })
    const posix = process.platform !== "win32"
    const cases: [string, string][] = [
      ["relative/gdl", "not_absolute"],
      [path.parse(f.root).root, "filesystem_root"],
      [posix ? "/usr/gdl" : "C:\\Windows\\gdl", "system_dir"],
      [path.join(f.source, "nested"), "nested"],
      [path.join(install, "data"), "install_overlap"],
      [f.pointer, "not_a_folder"]
    ]
    // Windows reports a path through a file as missing.
    if (posix) cases.push([path.join(f.pointer, "gdl"), "not_a_folder"])
    // Root writes and reads anywhere, so only others are refused these.
    const locked = path.join(f.root, "locked"),
      hidden = path.join(f.root, "hidden")
    if (posix && process.getuid?.() !== 0) {
      await fs.mkdir(locked, { mode: 0o555 })
      await fs.mkdir(path.join(hidden, "inside"), { recursive: true })
      await fs.chmod(hidden, 0o000)
      cases.push([path.join(locked, "gdl"), "not_writable"])
      // A folder the user may not even look into.
      cases.push([path.join(hidden, "inside", "gdl"), "not_writable"])
    }
    try {
      for (const [target, reason] of cases)
        expect(await advisory(null, target), target).toBe(reason)
    } finally {
      await fs.chmod(hidden, 0o755).catch(() => {})
    }
    expect(warn).toHaveBeenCalledTimes(cases.length)
    expect(String(warn.mock.calls[2])).toMatch(/system directory/)
    // Removable drives are looked up on Linux only.
    const linux = callback("validateRuntimePath", {
      ...f.globals,
      runtimeDestinationContext: () => ({ platform: "linux", mountInfo: "" })
    })
    expect(await linux(null, "/run/media/me/absent/gdl")).toBe(
      "removable_not_mounted"
    )
    // Settings checks the folder in use on every visit; that is not logged.
    warn.mockClear()
    expect(await advisory(null, f.source)).toBe("nested")
    expect(warn).not.toHaveBeenCalled()
  })

  it("moves a data folder reached through a link again, and back to where it was", async () => {
    const f = await fixture()
    const link = path.join(f.root, "link")
    await fs.symlink(f.target, link)
    await f.move(null, link)
    expect(selectRuntimePath(null, f.root)).toBe(link)
    // The launcher restarts with the link as its data folder.
    const fromLink = { ...f.globals, CURRENT_RUNTIME_PATH: link }
    const next = path.join(f.root, "next")
    expect(await callback("validateRuntimePath", fromLink)(null, next)).toBe(
      "valid"
    )
    expect(
      await callback("changeRuntimePath", fromLink)(null, next)
    ).toMatchObject({ type: "relaunching" })
    expect(selectRuntimePath(null, f.root)).toBe(next)
    expect(await fs.readFile(path.join(next, "save"), "utf8")).toBe("old save")
    expect(await fs.readdir(f.target)).toEqual([])
    // Links inside a data folder are still refused: a move cannot carry them.
    await fs.symlink(f.root, path.join(next, "inner"))
    await expect(inventoryRuntimeTree(next)).rejects.toThrow(/Unsupported/)
    await fs.unlink(path.join(next, "inner"))
    // The emptied old folder takes the data back.
    const fromNext = { ...f.globals, CURRENT_RUNTIME_PATH: next }
    expect(
      await callback("changeRuntimePath", fromNext)(null, f.source)
    ).toMatchObject({ type: "relaunching" })
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })

  it("resolves a folder where Windows cannot say where it really is, as on a RAM disk", async () => {
    const f = await fixture()
    const alias = path.join(f.root, "alias")
    await fs.symlink(f.source, alias)
    const realpath = fs.realpath.bind(fs)
    // What libuv reports when GetFinalPathNameByHandle is not supported.
    vi.spyOn(fs, "realpath").mockImplementation((async (file: any) => {
      if (isInside(String(file), f.root))
        throw Object.assign(
          new Error("EISDIR: illegal operation on a directory, realpath"),
          { code: "EISDIR" }
        )
      return realpath(file)
    }) as any)
    expect(await f.validate(null, path.join(f.root, "new"))).toBe("valid")
    // Still resolved: a link into the folder in use nests with it.
    vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(await f.validate(null, path.join(alias, "child"))).toBe("nested")
    // The move itself resolves the old folder the same way.
    await f.move(null, f.target)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readdir(f.source)).toEqual([])
  })
})

describe("a move that stops before it finishes", () => {
  async function many(count: number) {
    const f = await fixture()
    for (let i = 0; i < count; i++)
      await fs.writeFile(path.join(f.source, `file-${i}`), `bytes ${i}`)
    return f
  }

  it("lets the copies already running end, then removes every one", async () => {
    const f = await many(20)
    const copy = fs.copyFile.bind(fs)
    let calls = 0
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      const call = ++calls
      await new Promise((resolve) => setTimeout(resolve, 5))
      await (copy as any)(...args)
      if (call === 5) throw new Error("ENOSPC: no space left on device")
    }) as any)
    expect(await f.rawMove(null, f.target)).toMatchObject({
      type: "failed",
      incompleteTarget: false,
      copyRemoved: true
    })
    // No new copy starts after the failure.
    expect(calls).toBeLessThan(5 + 4)
    expect(await fs.readdir(f.target)).toEqual([])
    expect((await fs.readdir(f.source)).length).toBe(21)
  })

  it("removes the folders it made, but never what someone else put in them", async () => {
    const f = await fixture()
    const target = path.join(f.root, "new-a", "new-b", "runtime")
    const copy = fs.copyFile.bind(fs)
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      await (copy as any)(...args)
      await fs.writeFile(path.join(f.root, "new-a", "foreign"), "theirs")
      throw new Error("copy interrupted")
    }) as any)
    expect(await f.rawMove(null, target)).toMatchObject({
      type: "failed",
      copyRemoved: true
    })
    expect(fss.existsSync(path.join(f.root, "new-a", "new-b"))).toBe(false)
    expect(await fs.readdir(path.join(f.root, "new-a"))).toEqual(["foreign"])
  })

  it("removes a complete copy when the pointer cannot be written", async () => {
    const f = await fixture()
    vi.spyOn(fs, "rename").mockRejectedValue(
      Object.assign(new Error("EPERM: operation not permitted"), {
        code: "EPERM"
      })
    )
    expect(await f.rawMove(null, f.target)).toMatchObject({
      type: "failed",
      recovery: "restart",
      incompleteTarget: false,
      copyRemoved: true
    })
    expect(await fs.readdir(f.target)).toEqual([])
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })

  it("marks a complete copy again before removing any of it", async () => {
    const f = await many(3)
    vi.spyOn(fs, "rename").mockRejectedValue(
      Object.assign(new Error("EPERM: operation not permitted"), {
        code: "EPERM"
      })
    )
    const marker = path.join(f.target, MIGRATION_MARKER)
    const unlink = fs.unlink.bind(fs),
      markedWhileRemoving: (string | null)[] = []
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (path.dirname(String(file)) === f.target && String(file) !== marker)
        markedWhileRemoving.push(
          fss.existsSync(marker)
            ? JSON.parse(fss.readFileSync(marker, "utf8")).source
            : null
        )
      return unlink(file)
    })
    expect(await f.rawMove(null, f.target)).toMatchObject({
      type: "failed",
      copyRemoved: true
    })
    // Four copies, each removed while the folder says it is unfinished.
    expect(markedWhileRemoving).toEqual(Array(4).fill(f.source))
    expect(await fs.readdir(f.target)).toEqual([])
  })

  // A full disk: the pointer's temporary file and the marker both need space.
  function diskFullAfterCopy(markersRefused: number[]) {
    const full = () =>
      Object.assign(new Error("ENOSPC: no space left on device"), {
        code: "ENOSPC"
      })
    vi.spyOn(fs, "rename").mockRejectedValue(full())
    const open = fs.open.bind(fs)
    let markers = 0
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (
        String(file).endsWith(MIGRATION_MARKER) &&
        markersRefused.includes(++markers)
      )
        throw full()
      return (open as any)(file, ...args)
    }) as any)
  }

  it("removes a complete copy that cannot be marked again, since a full disk needs the space", async () => {
    const f = await many(3)
    diskFullAfterCopy([2])
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({
      type: "failed",
      incompleteTarget: false,
      copyRemoved: true
    })
    expect(await fs.readdir(f.target)).toEqual([])
    expect(await f.validate(null, f.target)).toBe("valid")
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect((await fs.readdir(f.source)).length).toBe(4)
  })

  it("marks what stays of a copy it could not mark before, once removing the rest made room", async () => {
    const f = await many(3)
    diskFullAfterCopy([2])
    const busy = path.join(f.target, "file-1")
    const unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (String(file) === busy)
        throw Object.assign(new Error("EBUSY: resource busy or locked"), {
          code: "EBUSY"
        })
      return unlink(file)
    })
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({ type: "failed", incompleteTarget: true })
    expect(result.copyRemoved).toBeUndefined()
    expect((await fs.readdir(f.target)).sort()).toEqual(
      [MIGRATION_MARKER, "file-1"].sort()
    )
    // Settings names it as an unfinished copy instead of offering a switch.
    expect(await f.validate(null, f.target)).toBe("incomplete_migration")
  })

  it("an unremovable copy that can never be marked is still not called removed", async () => {
    const f = await many(3)
    diskFullAfterCopy([2, 3])
    const busy = path.join(f.target, "file-1")
    const unlink = fs.unlink.bind(fs)
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (String(file) === busy)
        throw Object.assign(new Error("EBUSY: resource busy or locked"), {
          code: "EBUSY"
        })
      return unlink(file)
    })
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({ type: "failed", incompleteTarget: true })
    expect(result.copyRemoved).toBeUndefined()
    expect(await fs.readdir(f.target)).toEqual(["file-1"])
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
  })

  it("says what it removes while it removes the copy", async () => {
    const f = await many(6)
    const copy = fs.copyFile.bind(fs)
    let calls = 0
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      if (++calls === 5) throw new Error("ENOSPC: no space left on device")
      return (copy as any)(...args)
    }) as any)
    const seen: { action: string; currentName: string; total: number }[] = []
    await expect(
      relocateRuntime({
        source: f.source,
        target: f.target,
        pointer: f.pointer,
        stopCore: f.stop,
        progress: (p) => seen.push(p)
      })
    ).rejects.toThrow(/ENOSPC/)
    const rollback = seen.filter((p) => p.action === "rollback")
    // Every copy started, the failed one included, is named once.
    expect(rollback.length).toBeGreaterThanOrEqual(5)
    expect(rollback.map((p) => p.total)).toEqual(
      Array(rollback.length).fill(rollback.length)
    )
    expect(rollback.map((p) => p.currentName)).toEqual(
      expect.arrayContaining(
        seen.filter((p) => p.action === "copy").map((p) => p.currentName)
      )
    )
    expect(await fs.readdir(f.target)).toEqual([])
  })

  // Windows cannot rename the "drive" while the other copies hold files open.
  it.each(
    process.platform === "win32"
      ? ["while removing the copy"]
      : ["while copying", "while removing the copy"]
  )(
    "does not say the copy is gone when its drive was disconnected %s",
    async (when) => {
      const f = await many(6)
      const drive = path.join(f.root, "drive"),
        away = path.join(f.root, "unplugged")
      await fs.mkdir(drive)
      const target = path.join(drive, "gdl")
      const copy = fs.copyFile.bind(fs),
        unlink = fs.unlink.bind(fs)
      let calls = 0,
        removed = 0
      vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
        if (++calls === 4) {
          if (when === "while copying") {
            await fs.rename(drive, away)
            throw enoent(String(args[1]))
          }
          throw new Error("EIO: i/o error, copyfile")
        }
        return (copy as any)(...args)
      }) as any)
      vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
        if (isInside(String(file), target) && ++removed === 2)
          await fs.rename(drive, away)
        return unlink(file)
      })
      const result = await f.rawMove(null, target)
      expect(result).toMatchObject({ type: "failed", incompleteTarget: true })
      expect(result.copyRemoved).toBeUndefined()
      // Plugged in again, the folder still says it holds an unfinished move.
      await fs.rename(away, drive)
      expect(await fs.readdir(target)).toContain(MIGRATION_MARKER)
      expect(await f.validate(null, target)).toBe("incomplete_migration")
      expect(selectRuntimePath(null, f.root)).toBe(f.source)
    }
  )

  it("ends with nothing copied when the new folder's drive disappears after the last check", async () => {
    const f = await many(3)
    const drive = path.join(f.root, "drive")
    await fs.mkdir(drive)
    const target = path.join(drive, "gdl")
    let stopped = false,
      unplugged = false,
      calls = 0
    f.stop.mockImplementation(async () => {
      stopped = true
    })
    // Pulled out while the old folder is scanned once the core stopped. On
    // Windows the drive letter's root goes too.
    const readdir = fs.readdir.bind(fs),
      lstat = fs.lstat.bind(fs)
    vi.spyOn(fs, "readdir").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (stopped && String(file) === f.source) unplugged = true
      return (readdir as any)(file, ...args)
    }) as any)
    vi.spyOn(fs, "lstat").mockImplementation((async (file: any) => {
      const name = String(file)
      if (
        unplugged &&
        (name === path.parse(target).root || isInside(target, name))
      ) {
        if (++calls > 100) throw new Error("searched forever")
        throw enoent(name)
      }
      return lstat(file)
    }) as any)
    const result = await f.rawMove(null, target)
    expect(result).toMatchObject({ type: "failed", recovery: "restart" })
    expect(result.message).toMatch(/not connected/)
    expect(calls).toBeLessThan(30)
    // Nothing was made there, and the folder that held it is left alone.
    expect(await fs.readdir(drive)).toEqual([])
    expect(selectRuntimePath(null, f.root)).toBe(f.source)
    expect((await fs.readdir(f.source)).length).toBe(4)
  })

  it("keeps a copy the pointer names even when writing it reported a failure", async () => {
    const f = await fixture()
    const rename = fs.rename.bind(fs)
    vi.spyOn(fs, "rename").mockImplementation(async (from, to) => {
      await rename(from, to)
      throw new Error("reported after the rename")
    })
    const result = await f.rawMove(null, f.target)
    expect(result).toMatchObject({ type: "failed", incompleteTarget: false })
    expect(result.copyRemoved).toBeUndefined()
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })
})

describe("a move's copy is checked and flushed without repeating work", () => {
  it("copies several files at a time", async () => {
    const f = await fixture()
    for (let i = 0; i < 12; i++)
      await fs.writeFile(path.join(f.source, `file-${i}`), `bytes ${i}`)
    const copy = fs.copyFile.bind(fs)
    let running = 0,
      most = 0
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      most = Math.max(most, ++running)
      try {
        await new Promise((resolve) => setTimeout(resolve, 5))
        return await (copy as any)(...args)
      } finally {
        running--
      }
    }) as any)
    await f.move(null, f.target)
    expect(most).toBeGreaterThan(1)
    expect(most).toBeLessThanOrEqual(4)
    expect((await fs.readdir(f.target)).length).toBe(13)
  })

  it("flushes every copied file before the pointer names the new folder", async () => {
    flushFileByFile()
    const execFile = vi.spyOn(childProcess, "execFile")
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "mods"))
    for (let i = 0; i < 6; i++)
      await fs.writeFile(path.join(f.source, "mods", `mod-${i}.jar`), `${i}`)
    const open = fs.open.bind(fs),
      rename = fs.rename.bind(fs),
      synced = new Set<string>()
    let atCommit: string[] = []
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      const handle = await (open as any)(file, ...args),
        sync = handle.sync.bind(handle)
      handle.sync = async () => {
        await sync()
        synced.add(String(file))
      }
      return handle
    }) as any)
    vi.spyOn(fs, "rename").mockImplementation(async (from, to) => {
      if (String(to) === f.pointer) atCommit = [...synced]
      return rename(from, to)
    })
    await f.move(null, f.target)
    expect(atCommit).toEqual(
      expect.arrayContaining([
        path.join(f.target, "save"),
        ...Array.from({ length: 6 }, (_, i) =>
          path.join(f.target, "mods", `mod-${i}.jar`)
        )
      ])
    )
    expect(execFile).not.toHaveBeenCalled()
  })

  /**
   * Records what the move flushes: each file, each folder, or the whole
   * filesystem (`sync -f`), and what was flushed when the marker went and
   * when the pointer was written.
   */
  async function flushes(syncfsFails = false) {
    const f = await fixture()
    await fs.mkdir(path.join(f.source, "mods"))
    for (let i = 0; i < 6; i++)
      await fs.writeFile(path.join(f.source, "mods", `mod-${i}.jar`), `${i}`)
    const statfs = fs.statfs.bind(fs)
    // ext4, as most Linux home folders are.
    vi.spyOn(fs, "statfs").mockImplementation((async (...args: any[]) => ({
      ...(await (statfs as any)(...args)),
      type: 0xef53
    })) as any)
    const open = fs.open.bind(fs),
      unlink = fs.unlink.bind(fs),
      rename = fs.rename.bind(fs),
      flushed: string[] = []
    const seen = { atMarker: [] as string[], atCommit: [] as string[] }
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      const handle = await (open as any)(file, ...args),
        sync = handle.sync.bind(handle)
      handle.sync = async () => {
        await sync()
        flushed.push(String(file))
      }
      return handle
    }) as any)
    const execFile = vi.spyOn(childProcess, "execFile").mockImplementation(((
      file: string,
      args: string[],
      done: (error: Error | null) => void
    ) => {
      flushed.push(`${file} ${args.join(" ")}`)
      setImmediate(() => done(syncfsFails ? new Error("syncfs: EIO") : null))
    }) as any)
    vi.spyOn(fs, "unlink").mockImplementation(async (file) => {
      if (String(file) === path.join(f.target, MIGRATION_MARKER))
        seen.atMarker = [...flushed]
      return unlink(file)
    })
    vi.spyOn(fs, "rename").mockImplementation(async (from, to) => {
      if (String(to) === f.pointer) seen.atCommit = [...flushed]
      return rename(from, to)
    })
    const copies = [
      path.join(f.target, "save"),
      ...Array.from({ length: 6 }, (_, i) =>
        path.join(f.target, "mods", `mod-${i}.jar`)
      )
    ]
    return { f, flushed, seen, execFile, copies }
  }

  it.runIf(process.platform === "linux")(
    "on Linux flushes the whole filesystem once instead of each file and folder",
    async () => {
      vi.spyOn(os, "release").mockReturnValue("6.8.0-generic")
      const { f, flushed, seen, execFile, copies } = await flushes()
      const progress: string[] = []
      const outcome = await relocateRuntime({
        source: f.source,
        target: f.target,
        pointer: f.pointer,
        stopCore: f.stop,
        progress: (p) => progress.push(p.action)
      })
      expect(outcome.changed).toEqual([])
      expect(execFile).toHaveBeenCalledTimes(1)
      // Before the marker goes, and with no file or folder flushed on its own.
      expect(seen.atMarker).toContain(`sync -f ${f.target}`)
      expect(seen.atMarker.filter((file) => copies.includes(file))).toEqual([])
      expect(seen.atMarker).not.toContain(path.join(f.target, "mods"))
      expect(seen.atCommit).toContain(`sync -f ${f.target}`)
      // The marker's removal is flushed before the pointer names the folder.
      expect(seen.atCommit.slice(seen.atMarker.length)).toContain(f.target)
      expect(flushed.filter((file) => copies.includes(file))).toEqual([])
      // The overlay says the copy is being saved while the drive catches up.
      expect(progress.indexOf("flush")).toBeGreaterThan(
        progress.lastIndexOf("copy")
      )
      expect(progress.indexOf("flush")).toBeLessThan(progress.indexOf("remove"))
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
    }
  )

  it.runIf(process.platform === "linux")(
    "flushes each file and folder when the whole filesystem cannot be flushed",
    async () => {
      vi.spyOn(os, "release").mockReturnValue("6.8.0-generic")
      const { f, seen, execFile, copies } = await flushes(true)
      await f.move(null, f.target)
      expect(execFile).toHaveBeenCalledTimes(1)
      expect(seen.atMarker).toEqual(expect.arrayContaining(copies))
      expect(seen.atMarker).toEqual(
        expect.arrayContaining([path.join(f.target, "mods"), f.target])
      )
      expect(selectRuntimePath(null, f.root)).toBe(f.target)
    }
  )

  it.each([
    [
      "Linux before 5.8, whose syncfs does not report a failed write",
      "5.4.0-150-generic"
    ],
    ["Linux 4.19", "4.19.0-27-amd64"]
  ])("flushes file by file on %s", async (_, release) => {
    vi.spyOn(os, "release").mockReturnValue(release)
    const { f, seen, execFile, copies } = await flushes()
    await f.move(null, f.target)
    expect(execFile).not.toHaveBeenCalled()
    expect(seen.atMarker).toEqual(expect.arrayContaining(copies))
  })

  it("deletes originals untouched since their copy without reading them again", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.source, "other"), "other bytes")
    const open = fs.open.bind(fs),
      opened: string[] = []
    let removing = false
    vi.spyOn(fs, "open").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      if (removing) opened.push(String(file))
      return (open as any)(file, ...args)
    }) as any)
    const outcome = await relocateRuntime({
      source: f.source,
      target: f.target,
      pointer: f.pointer,
      stopCore: f.stop,
      progress: (p) => {
        if (p.action === "remove") removing = true
      }
    })
    expect(outcome).toMatchObject({ changed: [], notDeleted: [] })
    expect(opened).toEqual([])
    expect(await fs.readdir(f.source)).toEqual([])
  })

  it("moves a file whose change time alone moved while it was copied, as reading an online-only OneDrive file does", async () => {
    const f = await fixture()
    const save = path.join(f.source, "save")
    const copy = fs.copyFile.bind(fs)
    let changed = false
    vi.spyOn(fs, "copyFile").mockImplementation((async (...args: any[]) => {
      const before = await fs.lstat(save)
      await (copy as any)(...args)
      // Same bytes, size and modification time; only the change time moves.
      for (let i = 0; i < 50 && !changed; i++) {
        await new Promise((resolve) => setTimeout(resolve, 2))
        await fs.chmod(save, before.mode & 0o7777)
        changed = (await fs.lstat(save)).ctimeMs !== before.ctimeMs
      }
    }) as any)
    await f.move(null, f.target)
    if (process.platform !== "win32") expect(changed).toBe(true)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(await fs.readFile(path.join(f.target, "save"), "utf8")).toBe(
      "old save"
    )
    // Still the bytes that were copied, so the original is deleted.
    expect(await fs.readdir(f.source)).toEqual([])
  })
})

describe("a switch to a folder already in use", () => {
  it("never reads the old folder", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    // The old folder holds something a move could not take; a switch leaves it be.
    await fs.symlink(
      path.join(f.root, "elsewhere"),
      path.join(f.source, "link")
    )
    const readdir = fs.readdir.bind(fs),
      read: string[] = []
    vi.spyOn(fs, "readdir").mockImplementation((async (
      file: any,
      ...args: any[]
    ) => {
      read.push(String(file))
      return (readdir as any)(file, ...args)
    }) as any)
    await f.move(null, f.target, true)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(read.filter((file) => isInside(file, f.source))).toEqual([])
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })

  it("switches to a folder that holds links, as 2.0.40 did", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    await fs.symlink(f.root, path.join(f.target, "shared"))
    await f.move(null, f.target, true)
    expect(selectRuntimePath(null, f.root)).toBe(f.target)
    expect(await fs.readFile(path.join(f.source, "save"), "utf8")).toBe(
      "old save"
    )
  })

  it("switches to a link to a folder in use", async () => {
    const f = await fixture()
    await fs.writeFile(path.join(f.target, "existing"), "complete")
    const link = path.join(f.root, "link")
    await fs.symlink(f.target, link)
    expect(await f.validate(null, link)).toBe("potentially_valid")
    await f.move(null, link, true)
    expect(selectRuntimePath(null, f.root)).toBe(link)
  })
})

const isInside = (file: string, folder: string) =>
  file === folder || file.startsWith(folder + path.sep)

describe("a start that cannot open the data folder", () => {
  it("uses the default folder and names the one it could not open", async () => {
    const f = await fixture()
    const unavailable = vi.fn()
    // A drive letter or mount that is gone: the folder cannot be made.
    const file = path.join(f.root, "not-a-drive")
    await fs.writeFile(file, "")
    const folder = path.join(file, "gdl")
    await fs.writeFile(f.pointer, folder)
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(
      path.join(f.root, "data")
    )
    expect(unavailable).toHaveBeenCalledWith(folder)
    const shown = callback("getUnavailableRuntimePath", {
      UNAVAILABLE_RUNTIME_PATH: folder,
      CURRENT_RUNTIME_PATH: path.join(f.root, "data")
    })
    expect(await shown()).toEqual({
      folder,
      current: path.join(f.root, "data")
    })
    expect(
      await callback("getUnavailableRuntimePath", {
        UNAVAILABLE_RUNTIME_PATH: null,
        CURRENT_RUNTIME_PATH: f.source
      })()
    ).toBeNull()
  })

  it("says nothing when there is no data folder to miss", async () => {
    const f = await fixture()
    const unavailable = vi.fn()
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(f.source)
    await fs.writeFile(f.pointer, "relative/gdl")
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(
      path.join(f.root, "data")
    )
    await fs.unlink(f.pointer)
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(
      path.join(f.root, "data")
    )
    expect(selectRuntimePath("/explicit", f.root, unavailable)).toBe(
      "/explicit"
    )
    expect(unavailable).not.toHaveBeenCalled()
  })

  /** A data folder on a drive that is gone: the pointer names a folder that cannot be made. */
  async function goneForGood() {
    const f = await fixture()
    const drive = path.join(f.root, "usb")
    await fs.writeFile(drive, "")
    const folder = path.join(drive, "gdl")
    await fs.writeFile(f.pointer, folder)
    const keep = (unavailable: string | null = folder) =>
      callback("keepDefaultRuntimePath", {
        withRuntimeOperation,
        forgetUnavailableRuntimePath,
        path,
        app: { getPath: () => f.root },
        RUNTIME_PATH_OVERRIDE_NAME: "runtime_path_override",
        UNAVAILABLE_RUNTIME_PATH: unavailable
      })()
    return { ...f, drive, folder, keep }
  }

  it("keeps the default folder on request, so later starts use it without a word", async () => {
    const f = await goneForGood()
    const unavailable = vi.fn()
    const fallback = path.join(f.root, "data")
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(fallback)
    expect(unavailable).toHaveBeenCalledWith(f.folder)
    await f.keep()
    expect(fss.existsSync(f.pointer)).toBe(false)
    unavailable.mockClear()
    expect(selectRuntimePath(null, f.root, unavailable)).toBe(fallback)
    expect(unavailable).not.toHaveBeenCalled()
  })

  it("the forgotten folder can be switched back to once its drive is there again", async () => {
    const f = await goneForGood()
    await f.keep()
    // The drive is back, with the data folder on it untouched.
    await fs.unlink(f.drive)
    await fs.mkdir(f.folder, { recursive: true })
    await fs.writeFile(path.join(f.folder, "save"), "kept on the drive")
    const fromDefault = callback("changeRuntimePath", {
      ...f.globals,
      CURRENT_RUNTIME_PATH: path.join(f.root, "data")
    })
    await fs.mkdir(path.join(f.root, "data"))
    expect(
      await callback("validateRuntimePath", {
        ...f.globals,
        CURRENT_RUNTIME_PATH: path.join(f.root, "data")
      })(null, f.folder)
    ).toBe("potentially_valid")
    expect(await fromDefault(null, f.folder, true)).toMatchObject({
      type: "relaunching"
    })
    expect(selectRuntimePath(null, f.root)).toBe(f.folder)
    expect(await fs.readFile(path.join(f.folder, "save"), "utf8")).toBe(
      "kept on the drive"
    )
  })

  it("forgets only the folder this start could not open", async () => {
    const f = await goneForGood()
    // This start opened its data folder: there is nothing to forget.
    await f.keep(null)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.folder)
    // The pointer names another folder by now: it stays as it is.
    await fs.writeFile(f.pointer, f.source)
    await f.keep()
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    await forgetUnavailableRuntimePath(f.pointer, f.folder)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.source)
    // Already gone is fine.
    await fs.unlink(f.pointer)
    await f.keep()
    expect(fss.existsSync(f.pointer)).toBe(false)
  })

  it("waits for no move: it is refused while one runs, and says why when the pointer cannot be removed", async () => {
    const f = await goneForGood()
    await withRuntimeOperation(async () => {
      await expect(f.keep()).rejects.toThrow(/still finishing another task/)
    })
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.folder)
    vi.spyOn(fs, "unlink").mockRejectedValueOnce(
      Object.assign(new Error("EACCES: permission denied"), { code: "EACCES" })
    )
    await expect(f.keep()).rejects.toThrow(/EACCES/)
    expect(await fs.readFile(f.pointer, "utf8")).toBe(f.folder)
  })
})
