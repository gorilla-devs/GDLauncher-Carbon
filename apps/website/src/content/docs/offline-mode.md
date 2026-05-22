---
title: "Offline Mode"
description: "What you can and can't do in GDLauncher without an internet connection. What's cached, what needs to phone home, and how the token expiry behavior actually plays."
faq:
  - question: "Can I play Minecraft offline through GDLauncher?"
    answer: "Yes. Singleplayer works fully offline. If your cached token is still valid, you click Play and Minecraft launches normally. If the cached token has expired, the launcher shows an Account Expired prompt with a 'Launch anyway' button, choose that and you can still play singleplayer. Online auth is only required for multiplayer servers that verify identity through Mojang."
  - question: "How long can I stay offline before tokens expire?"
    answer: "It depends on what you want to do. For singleplayer, there's no real time limit, the launcher will offer 'Launch anyway' once the token is expired. For multiplayer servers that verify identity through Mojang, you need a fresh token, which means going back online to refresh. The launcher refreshes the Minecraft auth token proactively about 12 hours before its 24-hour expiry, so as long as you've been online recently, multiplayer keeps working."
  - question: "Can I install new mods or modpacks offline?"
    answer: "No. Mod downloads come from CurseForge and Modrinth's CDNs, both require internet. Same for Java downloads, Minecraft asset downloads, and modpack manifests. Anything install-related needs a connection."
  - question: "Can I update an existing instance offline?"
    answer: "No. Same reason: updates pull new files from CDNs. The launcher will queue the update and try again when it sees a connection."
  - question: "What about the GDL account, does it work offline?"
    answer: "Partially. The launcher remembers you're signed in to GDL, but anything that requires talking to the GDL service (Cloud Instance Sharing, profile edits, viewing your shares) needs internet. The Microsoft account is the one that gates launching; GDL is for features above and beyond launch."
---

# Offline Mode

## What "offline" actually means here

GDLauncher's offline behavior depends on three different network needs:

1. **Microsoft auth** (proving you own Minecraft to Mojang's servers).
2. **Mod and asset downloads** (CurseForge, Modrinth, Mojang's libraries CDN).
3. **GDL account features** (Cloud Instance Sharing, profile, display name history, etc.).

Each one fails differently when the internet is down, and the launcher's behavior is correspondingly different in each case.

## Launching an installed instance offline

This is the most common scenario: you're on a plane, in a cabin, or your home internet is out, and you want to play something you already have installed.

**Usually works**, because GDLauncher caches the data needed to launch:

- Mojang's auth tokens are stored locally with their expiry timestamps.
- Minecraft's libraries and assets are already on disk (in the runtime path).
- Modded instances have their mods installed locally.

When you click Play offline, the launcher:

1. Checks whether the active Microsoft account's Minecraft auth token is still valid (not expired).
2. If yes, it launches Minecraft with that token directly. Minecraft itself doesn't need internet to launch a singleplayer world.
3. If the access token is expired but the refresh token is still valid, the launcher tries to call Microsoft's refresh endpoint, which needs internet. Offline, this call fails, and the account's status flips to "expired" in Settings → Accounts.
4. If the account is expired and you click Play anyway, the launcher pops an Account Expired modal with two buttons: **Launch anyway** (uses the cached token, fine for singleplayer) and **Back to login** (sends you through the Microsoft sign-in flow, needs the internet).

So for singleplayer, "Launch anyway" works regardless of how long ago you were online, the token isn't verified by anything once Minecraft has started. For multiplayer servers that verify identity, you need a non-expired token, which means having been online recently enough to refresh.

### Why tokens expire

This is set by Microsoft's and Mojang's auth servers, not by GDLauncher. The auth chain produces two tokens that matter for the launcher:

- A **Microsoft OAuth access token** (~1 hour). This is what the launcher uses to talk to Microsoft / Xbox / Mojang's auth APIs. It's short-lived but the launcher renews it with a refresh token whenever it's online; you rarely notice.
- A **Minecraft auth token** (~24 hours). This is what gets handed to Minecraft at launch, so this is the one that controls offline play. GDLauncher refreshes it proactively about 12 hours before expiry while you're online.

Microsoft's refresh token lasts months but can be invalidated server-side, for example when you change your Microsoft password, enable a new security feature, or sign out from Microsoft's website. If your refresh token gets invalidated while you're offline, there's nothing the launcher can do until you're online again to re-authenticate.

## Joining multiplayer servers offline

**Doesn't work**, because multiplayer servers verify your identity against Mojang's session server, which needs internet on both sides. LAN multiplayer can work between machines on the same offline LAN as long as both already authed online recently.

## Installing new instances, mods, or modpacks offline

**Doesn't work.** Every install flow downloads from a CDN:

- Modpacks pull their manifest and then individual mod files.
- Adding a mod from the Addons tab downloads its JAR.
- Creating a custom instance for a Minecraft version you don't have downloads the JSON manifest for that version, plus the version JAR, plus assets, plus the mod loader installer.

All of those will fail offline with timeout or DNS errors. The launcher doesn't retry indefinitely, you'll see a failure in the instance creation modal or the Tasks panel.

If you know you're going somewhere offline, pre-install the instances you'll want before you leave.

## GDL account features offline

**Mostly doesn't work**, because GDL account features are by definition "talking to the GDL backend". Specifically:

- Cloud Instance Sharing (generating a code): fails, GDL service can't be reached.
- Importing a shared instance: fails for the same reason.
- Editing your GDL profile: fails.
- Listing your shares: shows cached state, can't refresh.

The launcher remembers you're signed in to GDL while offline, but the UI shows stale data and refuses actions that would need a network call.

## TL;DR

- Already-installed instance, fresh token: launch works offline.
- Already-installed instance, expired token: launcher prompts; pick "Launch anyway" for singleplayer.
- Multiplayer with expired token: blocked until you can reach Microsoft to refresh.
- Anything that downloads: blocked.
- Anything that talks to the GDL backend: blocked.
- Singleplayer worlds: 100% offline-capable once the instance is on disk.
