---
title: "GDL Account vs Microsoft Account"
description: "GDLauncher uses two different account types. Microsoft for playing Minecraft, GDL for sharing and social features. What each is, what they unlock, and which ones you need."
faq:
  - question: "Do I need a GDL account to play Minecraft?"
    answer: "No. To play Minecraft you only need a Microsoft account (the one you bought Minecraft Java Edition with). A GDL account is optional and only unlocks GDLauncher's own features: Cloud Instance Sharing, friend codes, display name history, profile editing. You can use GDLauncher just fine without one."
  - question: "What does a GDL account unlock?"
    answer: "Today, mainly Cloud Instance Sharing: you generate a code from a right-click → Share, and another GDLauncher user pastes it to import the instance. You also get a stable display name with rename history and a profile card with a friend code that identifies you in share previews. Everything that involves talking to other GDLauncher users runs through the GDL account."
  - question: "Can I use GDLauncher without a Microsoft account?"
    answer: "No. The Microsoft account is what proves you own Minecraft and gets the launch token from Mojang. Without it, the launcher has nothing to authenticate to Minecraft's servers with."
  - question: "Can I have multiple Microsoft accounts in GDLauncher?"
    answer: "Yes. Settings → Accounts shows all signed-in Microsoft accounts in a table. You can add more, remove ones you don't use, and pick which one is Active (the one Play uses). The active account is highlighted in the column to the left of the username."
  - question: "What's the friend code on my GDL profile?"
    answer: "A short stable identifier for your GDL account. It doesn't change when you rename your display name, and it's shown in share previews so other users can tell who the sharer is. Copy it from Settings → Accounts → GDL Account profile card."
---

# GDL Account vs Microsoft Account

## Two account systems, one launcher

GDLauncher has two account systems. **Microsoft** is the one that proves you own Minecraft and is required to play. **GDL** is GDLauncher's own optional account, used for features that involve the GDL backend (Cloud Instance Sharing, profile, display name history).

### Microsoft account

The account you bought Minecraft Java Edition with, the one that owns the game license. Microsoft requires it to launch Minecraft. GDLauncher signs in to Microsoft, holds onto the resulting tokens, and hands them to Mojang at launch time so Minecraft's servers know you own the game.

You need at least one signed-in Microsoft account to play. Without it, Play has nothing to do.

Stored locally per account: access token, refresh token, ID token, the Minecraft username and UUID, a skin reference, and the access-token expiry. The launcher refreshes the access token in the background using the refresh token; you usually never notice.

What it unlocks: launching Minecraft, joining servers, owning the game.

### GDL account

GDLauncher's own account system. It's optional and exists to power features GDLauncher itself provides, things Microsoft doesn't and shouldn't care about.

You sign up with an email and a display name, and you get a stable friend code. From there you can use the features that involve other GDLauncher users.

Stored locally is just the link: which Microsoft account this GDL identity belongs to, and a JWT for talking to the GDL backend. The display name, friend code, email, profile picture, and so on live on the GDL backend and the UI pulls them on demand.

What it unlocks:

- **Cloud Instance Sharing.** Right-click an instance → Share generates a code other GDLauncher users can paste to import the instance.
- **Display name history.** Renaming your display name tracks the change history; you can view past names from your profile card and clear them if you want.
- **Profile editing.** Display name, profile picture, email recovery settings, all from the GDL profile card in Settings → Accounts.

## When you need each

| Scenario | Microsoft | GDL |
|---|---|---|
| Just launch Minecraft | Required | Not needed |
| Install mods and modpacks from CurseForge/Modrinth | Required | Not needed |
| Share an instance with a friend | Required | Required |
| Receive an instance share code | Required | Required |
| Use the friend system | Required | Required |
| Play offline (already-installed instance) | Cached auth works briefly | Not needed |

## How to manage them

Both live in **Settings → Accounts**.

The GDL Account section sits at the top of the page. When signed out, it shows a Sign in / Sign up button. When signed in, it shows a profile card with display name, friend code (copyable), recovery email, and verification status. A "Danger Zone" near the bottom lets you schedule account deletion (with a 7-day cooldown).

The Microsoft Accounts section is a table below. Columns: Active, Username, Type, Status, UUID, Actions. The Status column tells you whether each account's token is fresh:

- **ok** (green checkmark): token is valid, the account can launch.
- **expired** (yellow alert): token has expired. The Actions column shows a refresh icon, clicking it sends you back through the Microsoft sign-in flow.
- **refreshing** (yellow refresh): the launcher is currently refreshing the token in the background. No action needed.
- **invalid** (red X): the token couldn't be refreshed. Same refresh icon as expired, clicking it walks you through the Microsoft sign-in flow.

To switch which account is Active, click the Active cell in the row you want. The active row shows a double-check icon; other rows show it faintly on hover.

## Removing accounts

Removing the only Microsoft account signs you out of GDLauncher entirely and you're routed back to the home page.

Removing a Microsoft account that's the linked owner of your GDL account triggers a confirmation modal, you'll be asked whether you really want to break the link before deletion proceeds.

Deleting your GDL account is a 7-day delayed action. During the cooldown you can cancel it from the same page.
