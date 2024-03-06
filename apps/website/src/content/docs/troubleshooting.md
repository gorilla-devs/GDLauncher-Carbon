---
title: "Troubleshooting"
---

## What to know about integrations?

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
