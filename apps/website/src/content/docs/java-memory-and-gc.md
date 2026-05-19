---
title: "Java Memory and Garbage Collection"
description: "How Minecraft uses RAM, why allocating more isn't always faster, what the heap, GC pauses, and Aikar's flags really do, and when to leave defaults alone."
faq:
  - question: "How much RAM should I allocate to Minecraft?"
    answer: "For vanilla: 2-4 GB is plenty. For a small modded instance (20-40 mods): 4 GB. For a large modpack (100+ mods, ATM-style): 6-8 GB. Going above 8 GB rarely helps because the bottleneck shifts to the garbage collector, not raw memory size."
  - question: "Will more RAM make Minecraft faster?"
    answer: "Up to a point, then no. Once Minecraft has enough heap to hold the world and all loaded mods without constant garbage collection, adding more memory just gives the GC more space to scan when it does run, which means longer GC pauses and stuttery gameplay. The right setting is 'just enough', not 'as much as I have'."
  - question: "What are Aikar's flags and should I use them?"
    answer: "Aikar's flags are a set of JVM arguments that tune the G1 garbage collector to favor short pauses over throughput, originally designed for Minecraft servers. They help on servers and on large modded clients. GDLauncher doesn't apply them automatically; you can paste them into Java Arguments in instance Settings if you want to try them. They're not magic and they're not always faster."
  - question: "Why does Minecraft stutter every few seconds even with 16 GB allocated?"
    answer: "That's almost always GC pause stutter, not a memory shortage. The garbage collector runs less often when you give it more heap, but when it does run on a bigger heap it takes longer. Try lowering allocation (paradoxically), or switch to a smaller modpack."
  - question: "What's the difference between Xmx and Xms?"
    answer: "Xmx is the maximum heap size (the ceiling). Xms is the initial heap size (the starting point). GDLauncher's RAM slider sets Xmx; Xms is set to a sensible value automatically. For Minecraft, setting Xmx equal to Xms doesn't help meaningfully, the JVM grows the heap as needed within Xmx anyway."
---

# Java Memory and Garbage Collection

## How Minecraft uses memory

Minecraft is a Java program. Like all Java programs, it runs inside a Java Virtual Machine (JVM) that gets a fixed slice of system RAM. Everything Minecraft does, loaded chunks, entities, mod state, textures, runs in that slice.

When you adjust **Instance Java Memory** in GDLauncher's instance settings, you're setting `-Xmx`, the maximum heap size the JVM is allowed to use. Java code itself (object allocations, mod data structures, the world state) lives in this heap. Textures and OpenGL buffers live outside the heap, in native memory, and aren't affected by Xmx.

## The garbage collector is the real bottleneck

Java doesn't free memory by hand; it has a **garbage collector** that periodically scans the heap, finds objects nothing references anymore, and reclaims them. Modern Minecraft uses the **G1** collector by default.

GC runs in two modes:

- **Young GC.** Short, frequent. Scans a small "young generation" of recently-created objects. Usually a millisecond or two.
- **Old GC / Mixed GC.** Longer, less frequent. Scans the rest of the heap. Can take dozens of milliseconds or more on a big heap.

When the GC runs, **Minecraft is paused**. The longer the heap, the longer those big collections take. That's why adding more RAM past what the game actually needs makes pause stutters *worse*, not better.

This is the single most counterintuitive thing about Java memory tuning: **allocating less can be smoother than allocating more**.

## The right amount of RAM

Rough guidelines, based on what fits the active heap without thrashing:

| Workload | Recommended Xmx |
|---|---|
| Vanilla Minecraft | 2-4 GB |
| Light mods (20-40, Sodium-style) | 4 GB |
| Medium modpack (80-120 mods) | 4-6 GB |
| Large modpack (ATM, FTB Continents, 250+ mods) | 6-8 GB |
| "Kitchen sink" modpacks (500+ mods, deep chunk pregen) | 8-10 GB |

Going above 10 GB is rarely useful unless the pack's documentation explicitly says so. Some packs do request more (memory-hungry mods like Better End or NetherEx in combination); follow the pack's recommended setting.

## How GDLauncher's slider works

Open an instance, click the Settings tab, scroll to **Instance Java Memory**. Flip the toggle to enable a per-instance override, then drag the slider; it goes from 1 GB up to your system's total RAM (with a warning past 80%). The launcher converts the value into `-Xmx<n>M` and passes it to the JVM.

The same slider exists at the global level in **Settings → Java → Java Memory**, used as the default for any instance that doesn't override. Set the global one low for casual play, bump only the heavy modpacks.

## Aikar's flags

A long set of JVM arguments that tune G1 to favor short pauses over throughput. Originally written for Minecraft servers but useful on modded clients too. They look like:

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

GDLauncher doesn't apply them by default. To use them, paste the full string into the **Instance Java Arguments** field in the instance's Settings tab (or into **Settings → Java → Java Arguments** to apply globally). Effects vary; the most consistent benefit is fewer long pauses on heaps in the 6-10 GB range.

A few caveats:

- They're tuned for older Java versions. On Java 17+ the defaults are already pretty good and Aikar's gains are smaller.
- They assume server-style allocation patterns. On a desktop with a small heap, they can hurt.
- They don't add memory or change how much the game uses; they only change how the collector behaves.

If you don't have a specific reason to use them, leave the Java Arguments field as GDLauncher set it.

## Diagnosing stutter

If Minecraft pauses for hundreds of milliseconds every few seconds:

1. Open the F3 debug screen. Look at the "Mem:" line (top right). If it's bouncing rapidly between low and high values, you're seeing GC churn.
2. Lower Xmx by 1-2 GB and try again. Counter-intuitive, but smaller heaps GC faster.
3. If a specific mod is allocating like crazy (some pre-generation or rendering mods do), it'll show up in mod-side profilers (Spark, JmxMC). The mod might need an update.
4. If your CPU is at 100% during pauses, the GC is genuinely working hard. Drop Xmx further or remove memory-hungry mods.

## TL;DR

- Set Java Memory to *just enough*, not as much as you have.
- The bottleneck is GC pause time, not raw heap size.
- Aikar's flags help on large heaps but aren't a magic fix.
- 4-6 GB is right for almost everything modded.
