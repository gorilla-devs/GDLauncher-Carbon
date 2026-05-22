---
title: "Microsoft Authentication Errors"
description: "Fix common Microsoft authentication errors in GDLauncher. Solutions for Invalid Grant, Account banned, Console access required, and Xbox Live errors."
faq:
  - question: "Why am I getting an 'Invalid Grant' error in GDLauncher?"
    answer: "An 'Invalid Grant' error usually means there's a problem with your Microsoft account security. The most common fixes are enabling two-factor authentication on your Microsoft account, setting a password if you didn't have one, or signing out and back in."
  - question: "Why does GDLauncher say my account is banned?"
    answer: "If GDLauncher reports your account as banned, the ban comes from Mojang or Microsoft, not from GDLauncher. Sign in to minecraft.net or your Microsoft account to see the ban reason. GDLauncher only relays the authentication response, there's no GDLauncher-side ban list."
  - question: "Why does GDLauncher say I need console access?"
    answer: "This typically appears for child accounts or accounts under family-group restrictions. The parent account needs to grant the child account permission to play Minecraft on the platform you're trying to use. Adjust the family settings on account.microsoft.com/family."
  - question: "I keep getting Xbox Live authentication errors. What do I do?"
    answer: "Xbox Live errors usually mean your country/region setting on the Microsoft account doesn't allow Xbox Live, or the account hasn't agreed to the Xbox Live terms of service. Sign in at xbox.com once with the same Microsoft account to accept the terms, then retry GDLauncher."
  - question: "Do I need to buy Minecraft again to use GDLauncher?"
    answer: "No. GDLauncher uses your existing Microsoft / Mojang Minecraft account. There's no separate purchase or subscription. If you already own Minecraft Java Edition, you can sign in to GDLauncher with the same account."
---

# Microsoft Authentication Errors

When you sign in to GDLauncher with a Microsoft account, the launcher talks to Microsoft's OAuth service and Mojang's authentication API on your behalf. Errors from those services are surfaced directly in the launcher; the wording comes from Microsoft, not GDLauncher.

Below are the most common ones and what they mean.

## Invalid Grant

Surfaces when Microsoft refuses the OAuth exchange. The most common causes:

- The account doesn't have a password set (it's a Microsoft account created via an email link or social login). Add a password at [account.microsoft.com](https://account.microsoft.com).
- The account uses an older sign-in flow without two-factor authentication. Enabling 2FA at [account.microsoft.com/security](https://account.microsoft.com/security) clears it up for most people.
- The cached tokens are stale. Sign out of the account in **Settings → Accounts** and sign in again.

## Account banned

GDLauncher relays Mojang's response unchanged. The ban is on the Mojang side; GDLauncher doesn't maintain its own ban list. Sign in at [minecraft.net](https://minecraft.net) with the same account to see the ban reason and appeal options.

## Console access required

This usually shows up for child accounts inside a Microsoft family group. The parent account needs to authorize Minecraft Java Edition for the child at [account.microsoft.com/family](https://account.microsoft.com/family). After granting permission, sign out and back in inside GDLauncher.

## Xbox Live errors

Most Xbox Live failures fall into one of two categories:

- The country/region on the Microsoft account doesn't allow Xbox Live. Adjust it at [account.microsoft.com/profile](https://account.microsoft.com/profile).
- The account hasn't accepted Xbox Live's terms of service. Sign in once at [xbox.com](https://xbox.com) with the same Microsoft account to accept them, then retry GDLauncher.

## Account expired

The Microsoft refresh token has expired or been revoked (most often because you changed the account password elsewhere). GDLauncher shows an "Account expired" prompt and offers to re-authenticate. Sign in again from **Settings → Accounts**.

## When in doubt

If the error message doesn't match any of the above, share both app-level logs on our [Discord](https://discord.gdlauncher.com): `main.log` (Electron) and the newest `__gdl_logs__/<timestamp>.log` (Rust core). See [Share App Logs](/guides/share-app-logs) for where to find them. We almost always need both, the authentication flow crosses between the two processes.
