---
title: "Anatomy of an Instance Folder"
description: "What's inside a GDLauncher instance folder, where mods, worlds, configs, screenshots, and logs live, and what's safe to delete by hand."
faq:
  - question: "Where do my Minecraft worlds save in GDLauncher?"
    answer: "Each world lives in <runtime_path>/instances/<instance>/instance/saves/<world-name>/. The saves/ folder is created the first time you generate or import a world. Open the instance folder via right-click → Open Folder, then go into instance/saves to find it. Copy the world folder to back it up."
  - question: "Where are my crash reports?"
    answer: "Inside the instance: instance/crash-reports/. The folder only exists if Minecraft has actually crashed; if you've never seen the game die hard, it won't be there yet. Each crash report is a timestamped text file (crash-<date>-server.txt or similar)."
  - question: "Where do mod JARs go?"
    answer: "instance/mods/ inside the instance folder. Adding a JAR there manually works but bypasses GDLauncher's tracking, prefer the Addons tab unless you have a reason. Locked modpack instances block the Addons tab's Add button, but a manual file copy goes through."
  - question: "What's the difference between the launcher logs and the Minecraft logs?"
    answer: "Two different sets. The launcher logs are GDLauncher's own session logs and live at <instance>/logs/. Minecraft's game logs (latest.log, crash-reports, anything the game writes) live one level deeper at <instance>/instance/logs/ and <instance>/instance/crash-reports/."
  - question: "What are instance.json and packinfo.json?"
    answer: "Metadata files GDLauncher writes at the top level of every instance. instance.json holds the instance name, icon, modloader, Minecraft version, last-played time, and play time. packinfo.json (only present for instances paired with a CurseForge or Modrinth modpack) tracks which files belong to the pack so the launcher can tell pack-managed mods apart from user-added ones. Don't delete either by hand."
---

# Anatomy of an Instance Folder

## Where the instance folder lives

Each GDLauncher instance sits in a subfolder of the runtime path:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← the instance folder
```

`<shortpath>` is a sanitized version of the instance's display name. Right-click the instance in GDLauncher and pick **Open Folder** to jump there.

## What's inside

The instance folder splits into a few things GDLauncher tracks at the top level, plus an `instance/` subfolder which is Minecraft's actual game directory. Some folders are always present; many are created on demand the first time something writes to them.

```
<shortpath>/
├── instance.json          ← GDLauncher's metadata for this instance (always present)
├── packinfo.json          ← Modpack pairing info (only for paired modpacks)
├── icon.png | icon.webp   ← Custom icon (only if you set one)
├── logs/                  ← GDLauncher's per-instance launcher logs
└── instance/              ← Minecraft's game directory
    ├── mods/              ← Mod JARs
    ├── config/            ← Mod configs
    ├── shaderpacks/       ← Shader packs (if you've installed any)
    ├── options.txt        ← Minecraft client settings (after first launch)
    ├── logs/              ← Minecraft's session logs (latest.log etc.)
    ├── saves/             ← Worlds (created when you generate one)
    ├── screenshots/       ← F2 screenshots (created on first F2)
    ├── crash-reports/     ← Crash dumps (only if the game has crashed)
    ├── resourcepacks/     ← Custom resource packs (when you add one)
    ├── datapacks/         ← Global data packs (per-world packs live under saves/<world>/datapacks)
    └── (pack-specific)    ← kubejs/, defaultconfigs/, packmenu/, etc. only if the modpack ships them
```

Don't be surprised if some of these are missing from a fresh instance. The launcher and Minecraft only create what they need, when they need it. An instance you've never played has no `saves/`, no `screenshots/`, no `options.txt`. A vanilla instance has no `mods/` or `config/`.

## What each thing holds

### Top-level files

- **`instance.json`**: GDLauncher's metadata, name, icon path, modloader and version, Minecraft version, when you created it, when you last played, total play time. Always present.
- **`packinfo.json`**: A hash manifest of which files came from the modpack. Lets the launcher tell apart pack-managed mods from anything you added yourself. Only present for instances paired with a CurseForge or Modrinth modpack.
- **`icon.png`** or **`icon.webp`**: The custom icon you uploaded. Missing if you're using the default.

### `logs/` (top-level)

GDLauncher's own per-instance logs. These are what the **View Logs** action in the right-click menu shows you. They cover the *launcher's* view of the launch (Java arguments, asset downloads, mod loader install, exit code) and are useful when the game never gets far enough to write its own log.

### `instance/mods/`

The mod JAR files. Minecraft loads everything in here at startup, subject to the mod loader's rules. The launcher tracks which mods belong to an instance in its database (keyed by filename and content hash), there are no sidecar files. JARs you drop in by hand are picked up too; the launcher just won't have CurseForge/Modrinth metadata for them.

### `instance/config/`

One subfolder or file per mod. This is where mod settings persist. Most mods write `config/<modid>.toml` or a `config/<modid>/` directory. Editing these by hand is usually safe; many mods read changes on game restart.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Asset packs. Resource packs change textures and sounds, shader packs change rendering (require Iris/OptiFine/etc. installed as a mod), data packs add recipes/loot/functions. Data packs intended for a specific world live under `saves/<world>/datapacks/` instead. These folders are only created when you actually have content for them.

### `instance/saves/`

Each world is one subfolder. Inside, `level.dat` is the world's master file, `region/` has the chunk data, `playerdata/` has per-player state, `datapacks/` has world-scoped data packs. To back up a world, copy the whole `<world>/` folder. The `saves/` folder itself appears the first time you generate a world.

### `instance/screenshots/`

Anything you've pressed F2 in. PNGs, named by timestamp. Created the first time you take a screenshot.

### `instance/logs/` and `instance/crash-reports/`

Minecraft's own diagnostic output. `logs/latest.log` is always the most recent launch (rolled into a `logs/<date>-1.log.gz` on the next launch). `crash-reports/` holds full crash dumps when the game dies hard, and only appears once there's actually been a crash.

### `instance/options.txt`

Minecraft's client settings (graphics, controls, sounds). Plain text, key=value. Editable if you really want to.

### Modpack-specific folders

Many large modpacks ship extra folders. The most common:

- **`kubejs/`**: KubeJS scripts (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Pack authors use this for runtime tweaks.
- **`defaultconfigs/`**: A snapshot of "what the configs should look like by default". The pack's launch script copies missing entries into `config/` on each start.
- **`packmenu/`**: Pack-themed main-menu assets (custom buttons, backgrounds, splash text).
- **`defaultsettings/`**: Similar to `defaultconfigs/`, but for `options.txt` and keybindings.

These only exist if the pack ships them. Vanilla and most custom instances don't have any.

## What's safe to delete

| Folder | Safe to delete? | Effect |
|---|---|---|
| `instance/mods/` (a specific JAR) | Yes | That mod is gone. May break worlds that used it. |
| `instance/config/<modid>/` | Yes | Mod resets to defaults next launch. |
| `instance/resourcepacks/`, `instance/shaderpacks/` contents | Yes | Pack is gone. |
| `instance/saves/<world>/` | Yes | World is gone permanently. Back up first. |
| `instance/logs/`, `crash-reports/` | Yes | Just frees disk space. |
| `instance/screenshots/` | Yes | Bye to old screenshots. |
| `logs/` (launcher logs) | Yes | Same. |
| `instance/options.txt` | Yes | Resets game settings to defaults. |
| `instance.json` | Don't | The launcher loses track of the instance. |
| `packinfo.json` | Possible (effectively unpairs) | The launcher stops treating the instance as a paired modpack. Same effect as the Unpair button in the instance's Settings tab, but messier. |
| The whole `instance/` folder | Don't | The instance becomes broken-shaped. Use Delete in the launcher UI instead. |

## A note on locked modpack instances

The instance folder is just a folder on disk; the lock that GDLauncher applies to modpack instances is enforced in the UI, not the filesystem. Dropping a JAR into `instance/mods/` of a locked instance works mechanically, the launcher just won't be aware of it through the Addons tab. Removing such a file requires going back to the filesystem. For safer workflows, open the instance, go to the **Settings** tab, and click **Unlock** in the Modpack section first.
