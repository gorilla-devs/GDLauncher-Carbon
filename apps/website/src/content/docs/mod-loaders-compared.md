---
title: "Mod Loaders Compared: Forge, NeoForge, Fabric, Quilt"
description: "GDLauncher supports four Minecraft mod loaders. What each is, how they differ, and which one to pick for a given mod or modpack."
faq:
  - question: "Which mod loader should I use for Minecraft?"
    answer: "Use whichever the mod or modpack you want requires, that's almost always how the decision gets made. If you're picking on your own from scratch: Fabric for performance/QoL mods on the newest versions, NeoForge for new big-content mods, Forge for older modpacks and the largest historical library."
  - question: "Are Forge mods compatible with Fabric?"
    answer: "No. Forge mods and Fabric mods are not interchangeable. A mod written for one won't load on the other. Many popular mods have separate Forge and Fabric builds; check the mod's page for which versions of which loader are supported."
  - question: "Is NeoForge a replacement for Forge?"
    answer: "Effectively yes for newer Minecraft versions. NeoForge started as a 2023 fork of Forge with the same API; the two have diverged since, so a current mod usually ships a separate NeoForge build rather than running across both. Many Forge mods from 1.20.4 onward are now built for NeoForge instead. For Minecraft 1.20.1 and earlier, Forge is still the standard."
  - question: "Are Fabric mods compatible with Quilt?"
    answer: "Mostly yes. Quilt is a fork of Fabric and runs Fabric mods directly. Some Quilt-only mods use Quilt APIs that don't run on Fabric. If you have a list of mods you want and they're all Fabric, you can use either loader and it'll work."
  - question: "Can I run two mod loaders side by side?"
    answer: "Not in the same instance. Each instance picks exactly one mod loader. To use both, create two separate instances. GDLauncher's instance system is designed for exactly this: one Forge instance, one Fabric instance, switch between them with a click."
---

# Mod Loaders Compared: Forge, NeoForge, Fabric, Quilt

## The four mod loaders GDLauncher supports

GDLauncher can install and run any of the four major mod loaders for Minecraft Java Edition, plus vanilla (no loader). You pick one when creating a custom instance; for modpack installs the loader is whatever the pack manifest says.

### Forge

The original mod loader, started in 2011. Forge has the largest historical mod library, especially for content-heavy mods that add tech trees, magic systems, or new worlds (Tinkers' Construct, Twilight Forest, Create on its earlier versions). It's also the loader most older modpacks target.

Forge updates more slowly than Fabric. New Minecraft versions often see a Forge release weeks or months after launch.

### NeoForge

A 2023 fork of Forge, created after a governance split. NeoForge keeps the Forge API style (mods are usually source-compatible) but ships faster and is where a lot of Forge mod development has migrated.

On Minecraft 1.20.4 and later, NeoForge is the more active of the two. Many large mods now ship NeoForge builds at parity with Forge or instead of it.

### Fabric

A different design philosophy: small, fast, modular. Fabric ships almost the day a new Minecraft version drops, sometimes within hours. Its mod ecosystem leans toward performance (Sodium, Lithium, FerriteCore), QoL (Mod Menu, Iris), and newer high-quality content mods.

Fabric is the loader you want if performance is the priority or if you're playing on a cutting-edge Minecraft version.

### Quilt

A 2022 fork of Fabric with a different governance model and a few extra APIs. Quilt runs Fabric mods directly, so the practical difference is small: pick Quilt if a specific mod requires it, otherwise Fabric works the same.

Quilt has a smaller dedicated ecosystem than Fabric but is fully compatible with most Fabric content.

## Compatibility matrix

| Mod built for | Runs on Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Yes | Sometimes (early NeoForge could run unmodified Forge mods because it started as a fresh fork; the APIs have diverged since, so most current Forge mods need a NeoForge build) | No | No |
| NeoForge | No | Yes | No | No |
| Fabric | No | No | Yes | Yes |
| Quilt | No | No | Quilt-API mods: no; rest: yes | Yes |

There's no cross-loader bridge in production. A mod loader is essentially a different runtime, the JARs you put in `mods/` need to match the loader the instance is running.

## Choosing for a new instance

You don't usually choose, the mods or modpack you want choose for you:

- **Installing a modpack from CurseForge or Modrinth?** GDLauncher reads the pack's manifest and installs the loader the pack specifies. You don't have a choice in the matter.
- **Building a custom instance around one specific mod?** Check the mod's page. If it says "Fabric 1.21.x", create a Fabric 1.21.x instance.
- **Building a custom instance around a list of mods?** Find the loader most of them support. Look up each mod, list which loaders it has builds for, pick the intersection. Most performance mods are Fabric-only; most big content mods are Forge/NeoForge.

If you genuinely have no constraint and want a vanilla recommendation: **Fabric** for performance-focused or visually polished setups, **NeoForge** for content-heavy modded survival.

## Switching loaders on an existing instance

GDLauncher lets you change an instance's mod loader after creation, see [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). Briefly: right-click the instance → Edit → pick a different loader. The mods folder isn't cleared, so JARs from the old loader stay around; remove the incompatible ones manually before launching.

## A note on loader versions

Each loader has its own version stream, independent from Minecraft. When you pick "Forge" you also pick a Forge version (something like `47.2.0` for Minecraft 1.20.1). For mods, the loader version usually doesn't matter past "the same major as the pack expects", but some mods do require a minimum loader build. The mod's CurseForge or Modrinth page will say so.
