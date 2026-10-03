---
title: "Runtime Path vs App Data Path"
description: "GDLauncher uses two different paths for storing data. What each holds, why they're separate, and which one you usually want to move."
faq:
  - question: "What's the difference between the app data path and the runtime path?"
    answer: "The app data path is the standard per-user directory Electron uses for caches and the runtime_path_override marker. The runtime path is where GDLauncher's core stores everything heavy: instances, Minecraft assets and libraries, Java installations, the launcher database, and the app-wide logs. By default the runtime path sits inside the app data path, but it's the one designed to be moved."
  - question: "Where is the app data path on my OS?"
    answer: "Windows: C:\\Users\\<you>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<you>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, or ~/.local/share/gdlauncher_carbon if XDG isn't set."
  - question: "Should I move the runtime path or the app data path?"
    answer: "The runtime path. It's the one that grows with each instance you install. The app data path stays small and is tied to your OS user profile. GDLauncher exposes the runtime path move in Settings → Runtime Path; the app data path is managed by Electron and isn't movable from the UI."
  - question: "What is the runtime_path_override file for?"
    answer: "After you change the runtime path, GDLauncher writes a small text file called runtime_path_override inside the app data path. It contains the new runtime path. On the next launch, GDLauncher reads it to know where your data lives. If it's missing, the launcher falls back to the default runtime path."
  - question: "Can I share the runtime path between two computers?"
    answer: "No. The launcher database tracks per-machine state (paths, account tokens, Java installs) and isn't designed for simultaneous use by two installs. If you want the same instances on a second computer, copy instances individually or use the Cloud Instance Sharing feature."
---

# Runtime Path vs App Data Path

## Two paths, two purposes

GDLauncher splits its files into two locations: an **app data path** for the small Electron-side bits, and a **runtime path** for the big stuff (instances, assets, Java, the database). The runtime path is the one you'll occasionally want to move, the app data path you usually never touch.

### App data path

This is the standard per-user app folder, the location Electron's `userData` points at. GDLauncher uses it for:

- The `runtime_path_override` marker, a one-line text file that tells the launcher where the runtime path actually lives
- The default runtime path, in a `data/` subfolder, if you haven't moved it
- Electron's own Chromium caches (Network/, GPUCache/, Cookies, etc.)
- The Electron main-process logs

It lives in the OS-standard location:

- **Windows:** `C:\Users\<you>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<you>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, or `~/.local/share/gdlauncher_carbon` if `$XDG_DATA_HOME` isn't set

Without the `data/` subfolder it's usually small. GDLauncher doesn't expose a setting to move this directory, the OS conventions and Electron expect it where it is.

### Runtime path (Core Module)

This is where GDLauncher's Rust core puts everything else:

- Your instances (under `instances/`)
- Minecraft's shared assets (the textures and sounds Mojang ships)
- Minecraft's shared libraries (the JAR files Mojang and the mod loaders provide)
- Java installations downloaded by GDLauncher
- The launcher database, `gdl_conf.db`
- The app-wide launcher logs, in `__gdl_logs__/`

By default the runtime path is `<app data path>/data/`, so it sits inside the app data folder. You can point it anywhere with **Settings → Runtime Path**. This is the path that gets big, large modpacks and a handful of modded instances easily push it past 50 GB.

## When to move the runtime path

The two common reasons:

1. **Your SSD is filling up.** Move instances to a larger HDD or a secondary SSD.
2. **You want backups separate from your OS user profile.** Putting the runtime path on a drive you back up independently is fine; just don't actively sync it while playing, the launcher and the sync tool will fight for file handles.

You don't need to move the runtime path for normal use. The default location is correct for most setups.

## How the move works

Open **Settings → Runtime Path**. Type the new location or pick it with the folder icon. The apply button (the circle-arrow icon at the right of the row) lights up once the path differs from the current one and is valid. Clicking it opens a confirmation modal showing the old and new paths.

If the target folder is empty (or doesn't exist yet), confirming runs a full migration: an overlay shows scanning, then per-file copying, then per-file removal of the source. Don't close the app or shut the machine down while this is running. After it finishes the launcher restarts itself. If some files stay in the old location, because they changed after they were copied or could not be deleted, the launcher lists them when it starts again from the new location, with a button that opens the old folder.

If the target already contains a GDLauncher runtime path (you previously moved it and want a fresh install to pick it up), the modal warns you in yellow that the folder isn't empty. Confirming there does a "switch only": the marker is rewritten to point at the existing data, no files are copied, and the launcher restarts. The data left at the old location becomes orphaned and you can delete it manually.

If a migration fails part-way, the overlay turns red and shows the error. Nothing is lost: until every file has been copied and checked, the launcher keeps using the old location and leaves your files there untouched, and after the restart Settings → Runtime Path still shows it. The launcher also removes the files it had copied to the new folder, so their space comes back and you can retry once you've fixed the cause. If it can't remove some of them (another program has them open, say), the new folder keeps them, plus a `.gdl_migration_in_progress` file that marks it as an unfinished move. The launcher won't use that folder or move into it again, and Settings says "This folder holds an unfinished move" if you pick it. Delete that folder (or pick another empty one) before you retry. The two common causes are missing write permissions on the target drive and running out of free space.

### The runtime_path_override marker

When you change the runtime path, GDLauncher writes a small text file called `runtime_path_override` inside the **app data path** (not inside the runtime path). The file contains the new runtime path as plain text. On every launch the app reads it to know where your data is.

If you delete the marker, GDLauncher falls back to its default runtime path (`<app data>/data/`). Your data isn't gone, it's still wherever you moved it to, but the launcher won't see it until you go to **Settings → Runtime Path** and point it back at that folder. Since the folder already contains GDLauncher data, the launcher treats it as a "switch only" operation and just updates the marker without copying anything.

If the folder the marker names can't be opened when GDLauncher starts (for example, it's on a drive that isn't connected), the launcher starts with the default runtime path and says so on every start. Connect the drive and restart to get your data back. If the drive is gone for good, choose **Keep the default folder** in that message: GDLauncher removes the marker and stops showing the message, and deletes nothing in the old folder. You can switch back to that folder later the same way as above.

## What the database holds, and why you shouldn't share it

The `gdl_conf.db` file in the runtime path holds account tokens, Microsoft refresh tokens, GDL account state, and per-instance metadata. It's machine-local and contains sensitive credentials. **Don't share it with anyone**, and don't try to use the same database across two computers, the second machine will fight the first for token refresh and both end up logged out.

If you want the same instances on a second computer, copy instances out of `instances/` manually, or use the Cloud Instance Sharing feature, which is designed for that.
