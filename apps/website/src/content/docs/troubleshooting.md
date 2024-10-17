---
title: "Troubleshooting"
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

The app logs are located in the Core Module Runtime Path, and they are stored in the `__logs__` folder.
Each log file is named after the date/time it was created.
Every time you start the app, a new log file is created.

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
