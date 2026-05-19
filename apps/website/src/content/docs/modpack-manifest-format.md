---
title: "Modpack Manifest Format"
description: "What's inside the three modpack archives GDLauncher reads and writes: CurseForge .zip, Modrinth .mrpack, and GDLauncher's own hash-based .gdlpack. Field-by-field reference, side-by-side comparison, and notes on how each one resolves mods at install time."
faq:
  - question: "What is a modpack manifest?"
    answer: "A JSON file inside a modpack archive that tells the launcher what the pack contains: Minecraft version, mod loader, list of mods, and which bundled files (overrides) to extract into the instance. GDLauncher reads the manifest before doing anything else when it installs a pack."
  - question: "What's the difference between CurseForge, Modrinth, and GDLauncher pack formats?"
    answer: "All three are zips with a JSON manifest. CurseForge uses manifest.json with CurseForge project/file IDs. Modrinth uses modrinth.index.json with direct download URLs plus hashes. GDLauncher uses gdlpack.json with content hashes only (sha512 + sha1 + murmur2) and resolves each file against whichever platform actually has it. They're not interchangeable; the launcher dispatches by which JSON it finds at the archive root."
  - question: "What's a .gdlpack and why does it exist?"
    answer: "It's GDLauncher's own modpack format. Each mod is identified by content hashes, so a single pack works whether the mods live on CurseForge, Modrinth, or both, with no URLs to go stale. It also has first-class support for optional features (skippable bundles of mods + configs) and embedded icons. Recommended when both sides use GDLauncher; CurseForge .zip remains the safer choice for cross-launcher distribution."
  - question: "Where do I find the manifest in an installed instance?"
    answer: "GDLauncher doesn't keep the original archive after install. The pack contents end up unpacked into the instance folder. The manifest GDLauncher works from is tracked in the launcher's database against the instance; it's not browsable as a file. To inspect a manifest before installing, open the .zip / .mrpack / .gdlpack with any archive tool and read the JSON file inside directly."
  - question: "Can I edit a manifest?"
    answer: "You can edit one inside an archive if you're building your own pack, but you can't edit the manifest of an installed instance through the launcher UI. To change what mods an instance has, use the Addons tab (or unlock a locked instance first). To change a paired pack's version, open the instance, go to the Settings tab, and click Change Modpack Version."
  - question: "What's an 'override' in pack terminology?"
    answer: "Files the pack ships pre-bundled, not mods. Default configs, scripts, resource packs, sometimes a starter world. They get extracted on top of the mods into the instance folder when the pack installs. CurseForge lists them under the overrides field; Modrinth marks pack files with env.client=required; GDLPack uses an overrides folder (configurable via the manifest)."
---

# Modpack Manifest Format

## Why this matters

You don't normally need to know what's inside a modpack archive. But when something goes wrong during install you'll see messages like "manifest", "invalid manifest format", "manifest missing field", or "manifest version unsupported", and you'll want to know what those refer to. This page is also useful if you're building a pack to share, or trying to figure out why the same pack installs cleanly in one launcher and breaks in another.

GDLauncher reads and writes three formats:

- **CurseForge** (`.zip`, `manifest.json` inside).
- **Modrinth** (`.mrpack`, `modrinth.index.json` inside).
- **GDLauncher** (`.gdlpack`, `gdlpack.json` inside).

All three are plain ZIP archives under the hood; the file extension just hints which manifest schema to expect.

## What's in a modpack archive

The general shape is the same across all three formats:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← format-specific manifest
├── overrides/            ← bundled configs, scripts, optional starter world
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (CurseForge only, optional, human-readable mod list)
```

The manifest is the only file the launcher strictly needs to know how to install the pack. The overrides folder is content to unpack into the resulting instance.

## CurseForge: `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

Key fields:

- `minecraft.version`: the Minecraft version the pack targets.
- `minecraft.modLoaders`: which loader and which version (the format is `<loader>-<version>`).
- `files`: each mod is referenced by CurseForge `projectID` and `fileID`. GDLauncher resolves these against the CurseForge API to actually download.
- `overrides`: name of the folder inside the zip whose contents get copied into the instance after mod downloads complete.

If a file's `projectID` or `fileID` no longer exists on CurseForge (the author removed it), the install fails with a "file not found" error for that specific mod.

## Modrinth: `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

Key differences from CurseForge:

- `dependencies` declares the Minecraft and loader versions directly (no nested object).
- Each `files` entry includes the **exact download URL** and **hash**, so installs don't depend on a metadata API call.
- Each file declares whether it's required on client, server, or both via `env`.

This format is simpler and easier to install offline (URLs are baked in). It's also stricter, hash mismatches mean a hard refusal to install rather than a warning.

## GDLauncher: `gdlpack.json`

GDLauncher's own pack format is the format GDLauncher writes when you pick **GDLauncher .gdlpack** in **Export Instance**. The defining property: every mod is identified only by its content hashes, not by a platform-specific ID or a URL. GDLauncher resolves each hash against CurseForge and Modrinth at install time and downloads from whichever platform actually has the file.

### Why hashes instead of IDs or URLs

- **Cross-platform from one source.** A mod that lives on both CurseForge and Modrinth has different IDs on each side. A `.gdlpack` doesn't care; one hash list works on either.
- **No URL rot.** Modrinth CDN URLs are content-addressed and stable, but adding URLs hard-codes a single download origin. Hashes let the launcher fall back when one platform is down.
- **Verification by construction.** Every byte that gets written to your `mods/` folder is checked against the manifest hash. There's no way to silently substitute a different JAR.

### Archive layout

```
mypack.gdlpack/
├── gdlpack.json          ← the manifest
├── .gdl/
│   └── icon.png          ← embedded pack icon (optional)
└── overrides/            ← bundled files (configs, scripts, non-resolvable mods)
    ├── config/
    └── ...
```

The manifest sits at the root. The `.gdl/` folder holds metadata the launcher embeds (currently just the icon). Overrides work the same as in CurseForge: contents are extracted into the resulting instance after mod resolution.

### Minimal manifest

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

Only `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries`, and `overrides` are required to load.

### Full manifest, annotated

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### Top-level fields

| Field | Type | Required | Meaning |
|---|---|---|---|
| `formatVersion` | integer | yes | Manifest schema version. Currently always `1`. |
| `name` | string | yes | Pack display name. Used as the default instance name on import. |
| `version` | string | no | Pack version (semver recommended, e.g. `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | no | One-line tagline. |
| `author` | string | no | Pack author or team. |
| `createdAt` | RFC 3339 timestamp | yes | When the archive was exported. |
| `icon` | string | no | Path inside the archive to the icon file (e.g. `.gdl/icon.png`). |
| `dependencies` | object | yes | Minecraft and modloader requirements (see below). |
| `entries` | array | yes | Platform files and optional features (see below). Empty array is valid for a pure-overrides pack. |
| `overrides` | string | yes (defaults to `"overrides"`) | Name of the directory inside the archive to extract into the instance. |
| `serverOverrides` | string | no | Directory of server-only overrides. |
| `clientOverrides` | string | no | Directory of client-only overrides. |
| `source` | object | no | If this pack is derived from a CurseForge or Modrinth pack, this points back at the original (see below). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: required Minecraft version (e.g. `1.20.1`, `1.21.4`, `25w20a` for snapshots).
- `modloaders`: zero or more loader entries. Each has:
  - `type`: one of `forge`, `neoforge`, `fabric`, `quilt` (lowercase).
  - `version`: exact loader version the pack was built against.
  - `primary`: marks the recommended loader when multiple are listed (e.g. a pack compatible with both Fabric and Quilt).

A vanilla pack has an empty `modloaders` array.

#### `entries`

Two entry types, discriminated by the `type` field:

**`platform`**, a required mod resolved via hash:

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

All three hashes must be present. Each one is used differently at resolution time:

| Hash | Used for |
|---|---|
| `sha512` | Modrinth resolution (Modrinth's `version_file` endpoint), primary integrity check on downloaded file. |
| `sha1` | Modrinth resolution (fallback), used by Minecraft itself when verifying the file at launch. |
| `murmur2` | CurseForge resolution (CurseForge's `fingerprints` endpoint). |

At install time, the launcher tries Modrinth first (sha512 lookup), falls back to CurseForge (murmur2 fingerprint), and fails the entry if neither platform has the file. Resolved files always download from whichever platform answered.

**`optional`**, a skippable group of mods and/or override paths the user can include or exclude:

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description`: shown to the user in the import preview so they can decide whether to include this feature.
- `platforms`: zero or more hash entries that get downloaded only if the feature is included.
- `overridePaths`: zero or more paths (files or folders) inside the `overrides/` directory that get extracted only if the feature is included. Paths are relative to `overrides/`.

A feature can have only platform files, only overrides, or both. Use these to bundle related optional content: a shader mod plus its config, a hardcore difficulty preset that's just configs, an optional resource pack.

#### `source`

When a pack was exported from an existing CurseForge or Modrinth modpack instance, GDLauncher records where it came from:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

or

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

This is informational; the launcher doesn't use it to download anything (the `entries` array already covers that). It exists so derivatives of public packs stay attributable to the original.

### Bundled vs manifest-only exports

When exporting an instance to `.gdlpack`, the **Bundle Addons** toggle in the export wizard decides what ends up where:

- **Bundle off (manifest-only).** Each mod GDLauncher can resolve via CurseForge or Modrinth becomes a `platform` entry in `entries`. The actual JAR is *not* included in the archive. The recipient's launcher re-downloads from the platform on import. Mods GDLauncher can't resolve (e.g. JARs you dropped in by hand) are bundled in `overrides/mods/` as a fallback. Result: small archive, recipient needs internet at install time.
- **Bundle on (self-contained).** No `platform` entries are emitted; every mod is copied into `overrides/mods/`. Result: larger archive (often hundreds of MB to several GB), installable offline once the Minecraft assets are cached.

Resource packs and shader packs behave the same way: resolvable ones become `platform` entries with bundle off, everything is bundled directly with bundle on.

## Side-by-side: the three formats

| Property | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Manifest filename | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Mod identification | CurseForge project + file ID | Download URL + hash | Content hashes only |
| Resolves from | CurseForge API | URL embedded in manifest | CurseForge **or** Modrinth, whichever responds |
| Offline install | No (needs CDN) | Yes (URLs may still need CDN) | Yes when `Bundle Addons` is on |
| Optional features | Per-mod `required: false` flag | Per-file `env` (client/server) | Dedicated `optional` entry with multiple files |
| Overrides folder | `overrides/` | per-file path | `overrides/` (configurable) |
| Server-only files | Separate pack | `env.server` | `serverOverrides` directory |
| Icon | External | External | Embedded in `.gdl/icon.<ext>` |
| Source tracking | None | None | `source` field |
| Portability | Broadest (most launchers read it) | Modrinth-supporting launchers | GDLauncher |

## Overrides, expanded

When the manifest references an overrides folder or pack files marked client-required, the launcher extracts those into the instance after the mod resolution step. This is how packs bundle:

- Default mod configs (so the pack plays the same out of the box for everyone).
- KubeJS or CraftTweaker scripts.
- A starter world (occasionally).
- Resource packs the pack expects to be active.
- `options.txt` with pack-tuned game settings.

Overrides win over auto-generated defaults but lose to anything the player later changes manually. GDLPack additionally supports `serverOverrides` and `clientOverrides` directories for files that should only land on one side, useful for packs intended to ship a matching client and server bundle.

## When manifests get out of date

A pack manifest is a snapshot of what the pack author last published. If a mod referenced by the manifest is removed from the source platform after the manifest is built, that pack version's install will fail until the author publishes a new one. This is what's happening when an older pack version installs cleanly but a newer one breaks: the newer one references something that's no longer available.

The fix is on the author's side; the launcher can only do what the manifest tells it. GDLPack has a partial advantage here because it isn't tied to one platform: if a mod is removed from CurseForge but still on Modrinth (or the other way around), the same `entries` array still resolves.

## Reading a manifest yourself

You don't need anything special. Rename `mypack.mrpack` or `mypack.gdlpack` to `mypack.zip`, open it with any archive tool, and look at the JSON file inside (`modrinth.index.json` or `gdlpack.json`). Same with CurseForge `.zip` files and `manifest.json`. All three are plain JSON, readable in any text editor.

## Building a pack

If you're building a pack to share, the easiest path is to set up the instance in GDLauncher, then use **Export Instance** (right-click → Export Instance). The export wizard lets you pick the target format: CurseForge `.zip`, Modrinth `.mrpack`, or GDLauncher's own `.gdlpack`. Pick `.gdlpack` if the recipient also uses GDLauncher and you want optional features or embedded icons; pick `.zip` for the broadest cross-launcher compatibility.
