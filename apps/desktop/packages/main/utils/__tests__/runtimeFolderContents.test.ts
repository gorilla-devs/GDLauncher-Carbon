import { describe, expect, it } from "vitest"
import { isLauncherEntry, isSystemLeftover } from "../runtimeFolderContents"

describe("what is at the top of a data folder", () => {
  it("knows the launcher's own folders, database files and records", () => {
    for (const name of [
      "instances",
      "servers",
      "managed_javas",
      "libraries",
      "assets",
      "natives",
      "logging_configs",
      "temp",
      "download",
      "gdl_conf.db",
      "gdl_conf.db-wal",
      "gdl_conf.db-shm",
      "gdl_conf.pre-downgrade.db",
      "gdl_conf.reset-recovery-older",
      "__gdl_logs__",
      "__gdl_runtime_writers",
      ".gdl_migration_in_progress"
    ])
      expect(isLauncherEntry(name), name).toBe(true)
  })
  it("takes anything else for someone else's", () => {
    for (const name of [
      "Steam",
      "notes.txt",
      "instances.bak",
      "gdl_conf",
      "my_gdl_conf.db",
      "saves",
      ".DS_Store"
    ])
      expect(isLauncherEntry(name), name).toBe(false)
  })
  it("knows what the system and file managers leave in a folder", () => {
    for (const name of [
      ".DS_Store",
      "._notes.txt",
      ".Spotlight-V100",
      ".fseventsd",
      ".Trashes",
      "desktop.ini",
      "Thumbs.db",
      "lost+found",
      ".Trash-1000",
      "$RECYCLE.BIN",
      "System Volume Information"
    ])
      expect(isSystemLeftover(name), name).toBe(true)
    for (const name of ["notes.txt", ".Trash-old", ".hidden", "temp", "found"])
      expect(isSystemLeftover(name), name).toBe(false)
  })
})
