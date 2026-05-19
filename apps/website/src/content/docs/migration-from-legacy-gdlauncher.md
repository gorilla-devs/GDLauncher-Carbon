---
title: "Migration from Legacy GDLauncher"
---

# Migration from legacy GDLauncher

If you're still using the GDLauncher Legacy, the migration to GDLauncher Carbon is simple. Here's how you can do it.

## Download GDLauncher Carbon

You can download GDLauncher Carbon from our [official website](https://gdlauncher.com).

## Install / Replace GDLauncher Carbon

Once you've downloaded the new version, you can install it by following the installation instructions for your operating system [here](/docs/installation).
Installing GDLauncher Carbon will replace the old version as they share the same exact app id. By installing GDLauncher Carbon you will not lose any of your old instances, as they are stored in a different folder.

## Auto Import your old instances

After installing GDLauncher Carbon and logging in, the onboarding flow asks if you want to import your existing instances from legacy GDLauncher. If you skipped onboarding, you can run the same flow at any time: open the Library page, click the **+** in the Library toolbar → **Import** tab → pick **GDLauncher (legacy)** as the source. GDLauncher Carbon reads the legacy launcher's instance list directly and imports each instance into its own folder.

## Manually Import your old instances

If the importer can't find the legacy launcher (e.g. installed in a non-default location), point it at the folder manually from the same Import tab.

If something still fails, please report it on our [Discord](https://discord.gdlauncher.com). As a last resort you can copy files by hand:

- Create a new instance in GDLauncher Carbon with the same configuration (same Minecraft version and mod loader; if it's a modpack, install the same exact pack version first).
- Open the legacy GDLauncher data path, find the source instance folder.
- Open the new Carbon instance: right-click → **More Options** → **Open Folder**. Copy the contents of the legacy instance into Carbon's `instance/` subfolder. See the [troubleshooting](/docs/troubleshooting) page for the exact data path locations.

## Deleting old instances (optional)

After you're done importing, you can optionally delete your old instances. This will not be done automatically.