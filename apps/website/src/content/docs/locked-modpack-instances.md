---
title: "Locked Modpack Instances"
description: "What it means when a modpack instance is locked, why GDLauncher locks them, and how to unlock or unpair when you need to."
---

## What is a locked instance?

When you install a modpack from CurseForge or Modrinth in GDLauncher, the instance is **locked** by default. A lock icon shows up next to the instance and the actions that would change the pack's contents, adding, removing, or updating individual mods, are disabled. You can still play the instance, change Java or RAM settings, take screenshots, and everything else; the lock only protects the *pack-managed mod set*.

The lock exists because a modpack is a tested, version-pinned collection of mods. Pack authors compose their mod list deliberately and pin specific versions for compatibility. If you swap out one mod for a newer version, you can break a sibling mod that depended on the older one. The lock catches that mistake before you make it.

## What you can and can't do while locked

You **can** do all of the following while an instance is locked:

- Launch and play the instance.
- Change RAM, Java arguments, and Java override.
- Take screenshots and browse logs.
- Change the instance's name and icon (via Edit Instance).
- Update the entire modpack to a newer release (Settings → Change Modpack Version).

You **can't** do these while locked:

- Add anything through the Addons tab, that covers **mods, shaders, resource packs, data packs, and worlds**. The Add button is disabled across every addon type while the lock is on.
- Remove or disable a pack-managed mod or addon.
- Update individual pack-managed mods to newer versions.

The Mods and Addons tabs show a *"This instance is locked, changes can't be applied"* hint next to disabled actions. Installing from the Addons browser into a locked instance is similarly blocked at the install button.

## Three states: locked, unlocked, unpaired

These three terms appear in GDLauncher and they're not synonymous.

- **Locked.** The instance is paired with a CurseForge or Modrinth modpack and the pack-managed mod set is read-only. This is the default for installed modpacks.
- **Unlocked.** Still paired with the modpack (the pack name and version are still tracked), but you can edit the mod set freely. GDLauncher remembers the pack so you can still update to a newer release later, but you take responsibility for keeping the mods consistent.
- **Unpaired.** No longer associated with the modpack at all. The instance becomes a custom instance, same files, but GDLauncher won't track pack updates or treat it as a modpack instance any more. Going from unlocked to unpaired is a one-way move.

## How to unlock an instance

1. Open the instance and click the cog icon (or right-click the instance → Settings).
2. Scroll to the **Modpack Info** section at the top of the settings page. You'll see the pack's icon, name, and current version, with a row of buttons below.
3. Click the **Unlock** button (the one with the lock icon next to "Unlock"). The instance flips to the unlocked state immediately.

Once unlocked, the section header changes to a "Unlocked" indicator with the open-padlock icon. You can lock the instance again by going through the same flow, but in practice, once you've started maintaining the mod set yourself, there's not much reason to relock.

## How to unpair an instance

1. In the same Modpack Info section, click the **Unpair** button (the git-branch icon).
2. Confirm in the modal that opens. GDLauncher warns you that the action is permanent.

After unpairing, the Modpack Info section disappears entirely. The instance is now a custom instance and the **Change Modpack Version** and **Reinstall** options no longer apply to it.

## Reinstall vs unlock

The Modpack Info section also has a **Reinstall** action. It's separate from unlocking and serves a different purpose: it reinstalls the modpack at its current version, overwriting the pack-managed mods and configs to whatever the manifest says they should be. Use it to recover a broken install (a mod jar got corrupted, configs got nuked, etc.) without losing your worlds.

| Action | Effect on pack-managed mods | Pack association |
|--------|----------------------------|------------------|
| Unlock | Stays, but now editable | Preserved |
| Unpair | Stays as files, no longer "pack mods" | Removed |
| Reinstall | Reset to manifest's version | Preserved |
| Change Modpack Version | Replaced with new version's manifest | Preserved (just at a new version) |

## When to unlock, and when not to

Unlock when:
- A specific pack-managed mod has a critical bug or security fix and the pack hasn't been updated.
- You want to add a personal mod, shader, resource pack, data pack, or world on top of what the pack ships with, the Addons tab's Add button is gated by the lock, so you have to unlock to install through the UI.
- You're maintaining an unmaintained pack yourself.

Stay locked when:
- The pack is actively maintained, let the author handle version pinning by waiting for the next pack release.
- You're playing through a curated experience and don't want to drift from the intended mod set.

A common pattern is: unlock briefly, install your extras, then leave the instance unlocked. Things you've added yourself stay even if you relock, since the lock only governs the *pack-managed* set, but in practice there's little reason to relock once you've started maintaining the instance.

## What the lock doesn't do

The lock is not a permission system or a security boundary. It's a guard rail to prevent accidental mod edits in the GDLauncher UI. The instance folder on disk is still a normal folder, anything that writes directly to the `mods` directory (a third-party tool, a manual file copy) bypasses the lock entirely.

If you do that and then look at the Mods tab, GDLauncher will show the manually-added file alongside the pack-managed mods. Removing such a file requires going through the file system, not the UI.

## Quick troubleshooting

- **"I can't update a single mod."** That's the lock working as intended. Either unlock (Settings → Unlock) or use Change Modpack Version to update the whole pack instead.
- **"The Update All button is greyed out on a locked instance."** Same reason. Either use Change Modpack Version, or unlock first.
- **"Why does my user-added mod still show on the Mods tab after relocking?"** The lock applies to pack-managed mods, anything you added on top stays visible regardless.
- **"The Reinstall option overwrote a config I edited."** That's expected. Reinstall resets to the pack manifest. Back up edited configs before reinstalling.
