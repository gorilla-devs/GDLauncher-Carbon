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

The first thing you should do is try to understand what the error message means.
If all the files were copied successfully, it probably errored while trying to delete the old files. You can close the app and manually delete the old files.

Make sure to NOT DELETE the file called `runtime_path_override` in the old runtime path, as it is used by the app to detect if the runtime path has been changed.

If you're in doubt, make sure to join our [discord server](https://discord.gdlauncher.com) and ask for help.
