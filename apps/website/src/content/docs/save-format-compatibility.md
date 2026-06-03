---
title: "Save Format Compatibility"
description: "Why upgrading a Minecraft world to a new version is usually a one-way trip, how the save format actually changes, and the right way to back up before letting it happen."
faq:
  - question: "Can I open a Minecraft 1.21 world in 1.20?"
    answer: "Not safely. Minecraft only ever migrates worlds forward, never backward. A world opened in 1.21 has its level.dat and region files rewritten in the newer format; older versions either refuse to load it or crash. If you need both, keep a backup copy of the world before launching the newer version."
  - question: "Does GDLauncher warn me before upgrading a world?"
    answer: "The launcher itself doesn't, the warning is on the Minecraft side. When you open a world saved in an older version, Minecraft shows a 'This world was saved in a different version' dialog before loading. That's the moment to back out and copy the world folder elsewhere."
  - question: "What gets rewritten when a world is upgraded?"
    answer: "level.dat (world metadata), the region files in region/ (chunk data), the playerdata/ folder (per-player state), and any per-world datapacks. The Data Version field in level.dat is bumped to match the new Minecraft version; that field is what newer/older versions read to decide whether they can open the world."
  - question: "Is downgrading impossible?"
    answer: "In the strict sense, yes. There's no official downgrade path. Some community tools claim to roll back the Data Version, but they don't actually rewrite chunk data, so the world ends up partially corrupted (newer biomes, blocks, or entities the older version doesn't understand). Treat upgrades as one-way."
  - question: "How do I back up a world before upgrading?"
    answer: "Right-click the instance in GDLauncher → Open Folder. Go into instance/saves and copy the world folder (named after the world) somewhere outside the instance directory. Keep that copy around until you're confident the upgraded version works."
---

# Save Format Compatibility

## Why save formats change

Minecraft's world file format isn't fixed. Every major Minecraft update revises how data on disk is structured. New blocks need new IDs. New entities need new NBT shapes. New biomes need new biome registries. Behind the scenes, every world has a number called the **Data Version** in `level.dat`, and Minecraft uses it to decide what to do when you open a world.

If your world's Data Version is older than the current Minecraft version's, Minecraft runs a one-time **DataFixer** pass that rewrites the world into the new format. Chunks, entities, block states, player data, all of it gets converted to the new schema. The Data Version in `level.dat` is updated to match.

This conversion is **destructive and one-way**. Once the chunks are rewritten, the older Minecraft version can't read them anymore.

## What "one-way" actually means

Imagine you have a 1.20.1 world. You open it in 1.21. Minecraft shows the "different version" warning, you click "Convert" (or load anyway), and the game starts. Behind the scenes:

- `level.dat` is rewritten so that its `DataVersion` field matches 1.21 instead of 1.20.1.
- Every region file in `region/` that gets loaded (everything within your view distance at minimum) gets rewritten chunk-by-chunk.
- New 1.21 blocks like Crafter or Trial Spawner can now exist in the world; they don't exist in 1.20.1's block registry.
- Existing 1.20.1 entities and tiles are migrated to the 1.21 schemas.

If you now try to open the same folder in 1.20.1:

- Minecraft compares `DataVersion` against its own and refuses to load (or crashes loading certain chunks).
- Even bypassing the version check would leave 1.21-only blocks as missing/error blocks in the older client.

Hence: **upgrading a world to a newer version is permanent**. The only safe rollback is restoring from a backup made *before* the upgrade.

## Modded worlds make it worse

Vanilla Minecraft's DataFixer is at least exhaustive and well-tested. Modded saves add another layer of risk:

- Removed mods leave **missing block** and **missing entity** errors. The world loads but cubes that were mod blocks become "?" placeholders.
- Replaced mods (an old version → a newer one) sometimes change block IDs or entity NBT keys. The migration is up to the mod author and isn't always smooth.
- Major Minecraft version jumps inside a modpack often coincide with most mods updating to entirely new APIs (Forge 1.20.1 → 1.21.x, for example). Worlds that worked under the old version may have undefined behavior under the new.

For modded instances, treat any version jump as a potential corruption event and back up first.

## Backing up a world properly

The simplest backup is a folder copy. In GDLauncher:

1. Right-click the instance → **Open Folder**.
2. Open `instance/saves/`.
3. Copy the folder named after your world (the same name you see in the world list) somewhere outside the instance. A separate drive, a `~/Documents/mc-backups/` folder, anywhere that won't get overwritten.

That copy is a snapshot of the world at the time you copied it. Keep it until you're certain the new version works.

For ongoing backups, third-party tools like FTBBackups (a mod) take in-game snapshots at intervals. They write to `backups/` inside the instance and are restorable per-snapshot.

## What "snapshot version" warnings mean

If you accidentally open a world saved in a Minecraft snapshot (a development build, like `24w11a`), the official game shows an additional warning because snapshot Data Versions are sometimes ahead of any released version's. A world from a snapshot can be unopenable in the next stable release if the snapshot's format made changes that were rolled back before release. The safe path is: don't play important worlds on snapshots, or accept that the world is snapshot-locked.

## TL;DR

- World upgrades are one-way; back up before opening in a newer version.
- Modded worlds are extra fragile; treat any version jump as a potential corruption event.
- For modpack updates that bump Minecraft versions, copy the entire saves folder first, then upgrade.
