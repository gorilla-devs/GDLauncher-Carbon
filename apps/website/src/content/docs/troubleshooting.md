---
title: "Troubleshooting"
description: "Fix common GDLauncher and Minecraft launch problems. App data path, runtime path, log locations, and known-good solutions."
faq:
  - question: "Where does GDLauncher store its data?"
    answer: "On Windows: C:\\Users\\<you>\\AppData\\Roaming\\gdlauncher_carbon. On macOS: /Users/<you>/Library/Application Support/gdlauncher_carbon. On Linux: $XDG_DATA_HOME/gdlauncher_carbon (or ~/.local/share/gdlauncher_carbon if XDG isn't set)."
  - question: "Where are GDLauncher logs?"
    answer: "GDLauncher writes two app-level logs to different files. main.log (Electron) lives in the app data folder, and timestamped <timestamp>.log files live in the runtime path's __gdl_logs__ folder (Rust core; the newest 10 are kept). When reporting issues, send both, see the Share App Logs guide for exact locations."
  - question: "GDLauncher won't open. What do I do?"
    answer: "First, check the logs in the data folder for an error. Common causes: corrupted runtime, antivirus blocking the executable, or a partially-applied update. Reinstalling GDLauncher fresh and restoring your instances usually fixes both."
  - question: "Why does my modpack crash on launch?"
    answer: "Most launch crashes come from a Minecraft version / mod loader / mod mismatch. Check the latest.log file for the error. If a single mod is named, that's usually the culprit, disable it in the Addons tab and relaunch. If it's an OutOfMemoryError, increase RAM in instance settings."
  - question: "How do I move GDLauncher to a different drive or folder?"
    answer: "Open Settings → General → Runtime Path. Change it to the new location and GDLauncher will migrate your instances and downloads automatically. The migration runs once on next launch."
  - question: "Can I use GDLauncher offline?"
    answer: "You can play instances you've already installed offline. Authentication still requires going online once initially (Microsoft account), and downloading new mods or modpacks needs an internet connection."
---

## App Data Path

This is the path where GDLauncher stores electron's data, as well as the Core Module Runtime Path by default.

### Windows

`C:\Users\\{{Your Username}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Your Username}}/Library/Application Support/gdlauncher_carbon`

### Linux

- if `$XDG_DATA_HOME` env is available: `$XDG_DATA_HOME/gdlauncher_carbon`
- else: `{{homedir}}/.local/share/gdlauncher_carbon`

[More details on homedir](https://nodejs.org/api/os.html#oshomedir)

## Core Module Runtime Path

This is the path where the core module stores all its data, including all instances, assets, libraries.
It is usually located in the same path as the App Data Path, nested in the `data` folder, unless you explicitly set it to another location.

### App Database

The app database is located in the Core Module Runtime Path, and it is a SQLite database file named `gdl_conf.db`.

**DO NOT SEND THIS FILE TO ANYONE, IT CONTAINS SENSITIVE DATA.**

### App Logs

GDLauncher writes two app-level logs to different files. For support, **always send both**, the two halves of the launcher hand work off to each other and the cause of a failure on one side often shows up in the other side's log.

- **`main.log`** in the App Data Path: the Electron main-process log. Covers window creation, IPC, auto-update, native dialogs, and hard crashes of the desktop shell.
- **`__gdl_logs__/<timestamp>.log`** in the Core Module Runtime Path: the Rust core log. Covers account sign-in, asset downloads, mod loader installs, instance launches, settings changes. The launcher keeps the most recent 10 and removes older ones automatically; the newest is the one you want.

See [Share App Logs](/guides/share-app-logs) for screenshots and per-OS paths.

**LOGS MAY CONTAIN SENSITIVE DATA, BE CAREFUL WHEN SHARING THEM.**

### Change Runtime Path

If you change the runtime path, the app will automatically move all your instances and configuration files to the new location.

If the target folder is already in use, the app will simply switch the runtime path configuration and no files will be moved or copied.

#### Migration Error

If the migration fails, the app will display an error message.

Read the error message and note which stage failed. Keep the old runtime, the destination, and any recovery copies until you have verified both the destination's contents and the runtime the launcher actually uses after restart. Finishing the copy does not prove that the runtime pointer was published. Older launcher versions could roll back copied destination files after a later failure; deleting the source then could delete the only usable copy.

**If Settings → Runtime Path still shows the old location after the restart**, the move stopped before it finished and your data is safe in the old location. The launcher removes what it had copied to the new folder, so fix the cause shown in the error (for example, free up space on the target drive) and move again, to the same path or to another empty folder. If it could not remove some of those files (another program had them open, say), the new folder keeps them, marked by a `.gdl_migration_in_progress` file. The launcher won't use that folder or move into it again, and Settings says "This folder holds an unfinished move" if you pick it. You can delete that folder, then move again.

When a move fails before the runtime pointer is published, the current launcher removes the destination data it copied; anything it cannot remove stays, marked with `.gdl_migration_in_progress`, while the old runtime remains selected. After pointer publication, the destination is selected unless an explicit override takes precedence; subsequent warnings do not roll it back. After that, the launcher deletes each original from the old runtime once it has checked that the file still matches its copy, so the move frees the space on the old drive. A file that changed after it was copied stays in the old runtime, as does one that could not be deleted (another program had it open, or you lack permission). The launcher restarts either way, and its first start from the new location lists those files in two groups. Files that changed may be newer than their copy in the new location: if you need one, copy it to the same place in the new location before you play. Files that could not be deleted match their copy, so you can delete them. If the launcher cannot confirm that the new location was saved to disk, it deletes nothing and says so after the restart; once you have checked that your instances are in the new location, you can delete the old folder.

The `runtime_path_override` pointer is in Electron's `userData` directory (the launcher's application data), not the old runtime. Preserve it. An explicit command-line runtime path takes precedence over an environment runtime path, and both take precedence over this pointer. Use the launcher's runtime path setting to verify the effective location. If the recovery dialog requires Restart or Retry, follow that action; keep both folders if it fails again.

See the [runtime_path_override marker and app-data locations](/docs/runtime-path-vs-app-data#the-runtime_path_override-marker) when locating the authoritative pointer.

If you're in doubt, make sure to join our [discord server](https://discord.gdlauncher.com) and ask for help.
