---
title: "Java-Speicher und Garbage Collection"
description: "Wie Minecraft RAM nutzt, warum mehr nicht immer schneller heißt, was Heap, GC-Pausen und Aikar's Flags tatsächlich tun, und wann man die Defaults besser lässt."
faq:
  - question: "Wie viel RAM soll ich Minecraft geben?"
    answer: "Vanilla: 2-4 GB reichen. Kleine Modded-Instanz (20-40 Mods): 4-6 GB. Großes Modpack (100+ Mods, ATM-Stil): 6-8 GB. Über 8 GB hilft kaum, der Engpass wandert von der Speichergröße zum Garbage Collector und größere Heaps bedeuten längere GC-Pausen und ruckeligeres Gameplay."
  - question: "Macht mehr RAM Minecraft schneller?"
    answer: "Bis zu einem Punkt, dann nicht mehr. Sobald Minecraft genug Heap hat, um Welt und Mods ohne ständiges Garbage Collecting zu halten, gibt zusätzlicher Speicher dem GC nur mehr zu scannen, wenn er läuft, längere Pausen, ruckeligeres Spiel. Richtig ist 'gerade genug', nicht 'so viel wie ich habe'."
  - question: "Was sind Aikar's Flags und sollte ich sie nutzen?"
    answer: "Aikar's Flags sind JVM-Argumente, die den G1-GC auf kurze Pausen statt Durchsatz trimmen, ursprünglich für Minecraft-Server. Sie helfen auf Servern und auf großen modded Clients. GDLauncher setzt sie nicht automatisch; du kannst sie in Instance Settings → Java Arguments einfügen. Sie sind kein Wundermittel und nicht immer schneller."
  - question: "Warum ruckelt Minecraft alle paar Sekunden, obwohl 16 GB allokiert sind?"
    answer: "Fast immer GC-Pause-Ruckler, kein Speichermangel. Der GC läuft seltener bei größerem Heap, aber jeder Lauf dauert dann länger. Paradox: weniger zuweisen oder kleineres Modpack nehmen."
  - question: "Was ist der Unterschied zwischen Xmx und Xms?"
    answer: "Xmx ist die maximale Heap-Größe (Obergrenze). Xms die initiale (Startwert). GDLaunchers RAM-Slider setzt Xmx; Xms wird automatisch sinnvoll gesetzt. Für Minecraft bringt es nichts, Xmx gleich Xms zu setzen, die JVM wächst innerhalb von Xmx ohnehin nach Bedarf."
---

# Java-Speicher und Garbage Collection

## Wie Minecraft Speicher nutzt

Minecraft ist ein Java-Programm. Wie jedes Java-Programm läuft es in einer Java Virtual Machine (JVM), die einen festen Anteil System-RAM bekommt. Alles, was Minecraft tut, geladene Chunks, Entities, Mod-State, Texturen, lebt in diesem Anteil.

Wenn du in den Instance Settings von GDLauncher **Instance Java Memory** einstellst, setzt du `-Xmx`, die maximale Heap-Größe, die die JVM nutzen darf. Java-Code (Objekt-Allokationen, Mod-Datenstrukturen, Welt-State) lebt in diesem Heap. Texturen und OpenGL-Buffer liegen außerhalb, im Native Memory, und sind von Xmx nicht betroffen.

## Der wahre Engpass ist der Garbage Collector

Java gibt Speicher nicht von Hand frei; es hat einen **Garbage Collector**, der periodisch den Heap scannt, Objekte ohne Referenzen findet und freigibt. Modernes Minecraft nutzt standardmäßig den **G1**-Collector.

GC läuft in zwei Modi:

- **Young GC.** Kurz, häufig. Scannt eine kleine "Young Generation" frisch erzeugter Objekte. Meist ein paar Millisekunden.
- **Old GC / Mixed GC.** Länger, seltener. Scannt den Rest des Heaps. Auf großem Heap zig Millisekunden bis mehr.

Wenn der GC läuft, ist **Minecraft pausiert**. Je größer der Heap, desto länger die großen Collections. Deshalb macht mehr RAM über das eigentliche Bedürfnis hinaus Pause-Ruckler *schlimmer*, nicht besser.

Das ist das kontraintuitivste an Java-Memory-Tuning: **weniger zuteilen kann smoother sein als mehr**.

## Die richtige RAM-Menge

Grobe Richtwerte für den passenden Heap ohne Thrashing:

| Workload | Empfohlene Xmx |
|---|---|
| Vanilla Minecraft | 2-4 GB |
| Leichte Mods (20-40, Sodium-Stil) | 4 GB |
| Mittleres Modpack (80-120 Mods) | 4-6 GB |
| Großes Modpack (ATM, FTB Continents, 250+ Mods) | 6-8 GB |
| "Kitchen Sink"-Modpacks (500+ Mods, tiefe Chunk-Pregen) | 8-10 GB |

Über 10 GB nur, wenn die Pack-Doku es explizit verlangt. Manche speicherhungrigen Mod-Kombos (Better End plus NetherEx z. B.) brauchen wirklich mehr, dann folgst du der Empfehlung des Packs.

## Wie GDLaunchers Slider funktioniert

Instanz öffnen, auf den Settings-Tab klicken, zu **Instance Java Memory** scrollen. Den Toggle umlegen, um den Per-Instance-Override zu aktivieren, dann den Slider ziehen; er geht von 1 GB bis zum Gesamt-RAM des Systems (mit Warnung jenseits 80 %). Der Launcher übersetzt den Wert in `-Xmx<n>M` und gibt's der JVM.

Denselben Slider gibt's global in **Settings → Java → Java Memory**, als Default für jede Instanz, die nichts überschreibt. Den globalen niedrig halten, nur schwere Modpacks anheben.

## Aikar's Flags

Lange Liste an JVM-Argumenten, die G1 auf kurze Pausen statt Durchsatz tunen. Original für Minecraft-Server, aber auch auf modded Clients nützlich. Sieht so aus:

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

GDLauncher setzt sie nicht standardmäßig. Einfügen kannst du sie in das **Instance Java Arguments**-Feld im Settings-Tab der Instanz (oder in **Settings → Java → Java Arguments** für global). Effekte variieren; konstantester Vorteil sind weniger lange Pausen auf Heaps zwischen 6 und 10 GB.

Wichtige Punkte:

- Tuned für ältere Java-Versionen. Auf Java 17+ ist der Default schon gut, Aikar's Gewinn geringer.
- Geht von Server-Allokationsmustern aus. Auf Desktop mit kleinem Heap kann's schaden.
- Bringt keinen extra Speicher und ändert nicht, wie viel das Spiel nutzt, nur das Verhalten des Collectors.

Ohne konkreten Grund: das Java-Arguments-Feld so lassen, wie GDLauncher es setzt.

## Ruckler diagnostizieren

Wenn Minecraft alle paar Sekunden hunderte Millisekunden steht:

1. F3-Debug-Screen öffnen, "Mem:"-Zeile (rechts oben) checken. Springt sie schnell zwischen niedrig und hoch, ist GC-Churn die Ursache.
2. Xmx um 1-2 GB runter und nochmal testen. Kontraintuitiv, aber kleinerer Heap GC'd schneller.
3. Bestimmter Mod allokiert wild (manche Pregen- oder Rendering-Mods tun das), zeigt sich in Mod-Profilern (Spark, JmxMC). Mod braucht vielleicht ein Update.
4. CPU bei 100% während Pausen heißt, der GC arbeitet wirklich hart. Xmx weiter senken oder speicherhungrige Mods raus.

## TL;DR

- Java Memory auf *gerade genug* stellen, nicht so viel wie du hast.
- Engpass ist GC-Pausenzeit, nicht reine Heap-Größe.
- Aikar's Flags helfen auf großen Heaps, sind kein Allheilmittel.
- 4-6 GB ist für fast alles Modded richtig.
