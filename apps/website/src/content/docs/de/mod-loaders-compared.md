---
title: "Mod-Loader im Vergleich: Forge, NeoForge, Fabric, Quilt"
description: "GDLauncher unterstützt vier Minecraft-Mod-Loader. Was jeder ist, wie sie sich unterscheiden, und welcher für welchen Mod oder Modpack passt."
faq:
  - question: "Welchen Mod-Loader soll ich für Minecraft nehmen?"
    answer: "Den, den der Mod oder das Modpack verlangt, in der Praxis macht das die Wahl. Wenn du frei wählst: Fabric für Performance/QoL auf aktuellen Versionen, NeoForge für große Content-Mods, Forge für ältere Modpacks und die größte Bibliothek."
  - question: "Laufen Forge-Mods auf Fabric?"
    answer: "Nein. Forge- und Fabric-Mods sind nicht austauschbar. Ein Mod für den einen Loader lädt auf dem anderen nicht. Viele populäre Mods gibt's als separate Forge- und Fabric-Builds, die jeweilige Mod-Seite zeigt die unterstützten Loader-Versionen."
  - question: "Ist NeoForge ein Ersatz für Forge?"
    answer: "Für neuere Minecraft-Versionen praktisch ja. NeoForge startete 2023 als Fork von Forge mit derselben API; die beiden sind seitdem auseinandergedriftet, ein heutiger Mod liefert in der Regel einen separaten NeoForge-Build statt auf beiden zu laufen. Ab 1.20.4 bauen viele Forge-Mods für NeoForge statt für Forge. 1.20.1 und früher ist Forge weiterhin Standard."
  - question: "Laufen Fabric-Mods auf Quilt?"
    answer: "Meistens ja. Quilt ist ein Fabric-Fork und führt Fabric-Mods direkt aus. Manche reine Quilt-Mods nutzen Quilt-eigene APIs und laufen nicht auf Fabric. Wenn deine Mods komplett Fabric sind, kannst du beide Loader nehmen, gleiches Ergebnis."
  - question: "Kann ich zwei Mod-Loader gleichzeitig nutzen?"
    answer: "Nicht in derselben Instanz. Jede Instanz hat genau einen Loader. Wer beide will, legt zwei Instanzen an. Genau dafür ist GDLaunchers Instanz-System gemacht: Eine Forge-, eine Fabric-Instanz, mit einem Klick wechseln."
---

# Mod-Loader im Vergleich: Forge, NeoForge, Fabric, Quilt

## Die vier von GDLauncher unterstützten Mod-Loader

GDLauncher kann jeden der vier großen Minecraft-Java-Mod-Loader installieren und starten, plus Vanilla (kein Loader). Beim Anlegen einer Custom-Instanz wählst du einen. Bei Modpack-Installation kommt der Loader, den die Pack-Manifest vorgibt.

### Forge

Der ursprüngliche Mod-Loader, gestartet 2011. Forge hat die größte historische Mod-Bibliothek, besonders für Content-schwere Mods (Tech-Trees, Magie, neue Welten, wie Tinkers' Construct, Twilight Forest, Create in älteren Versionen). Die meisten älteren Modpacks zielen ebenfalls auf Forge.

Forge updated langsamer als Fabric. Neue Minecraft-Versionen brauchen oft Wochen bis Monate, bis es ein Forge-Release gibt.

### NeoForge

Ein 2023er Fork von Forge, entstanden nach einem Streit in der Forge-Community. NeoForge behält den Forge-API-Stil (Mods sind meist quellkompatibel), erscheint aber schneller, und ein Großteil der Forge-Mod-Entwicklung ist dorthin gewandert.

Ab Minecraft 1.20.4 ist NeoForge der aktivere der beiden. Viele große Mods bringen inzwischen NeoForge-Builds entweder gleichzeitig mit Forge oder statt Forge raus.

### Fabric

Andere Philosophie: klein, schnell, modular. Fabric kommt nahezu am Release-Tag einer neuen Minecraft-Version, manchmal innerhalb von Stunden. Das Mod-Ökosystem geht eher in Richtung Performance (Sodium, Lithium, FerriteCore), QoL (Mod Menu, Iris) und neuere High-Quality-Content-Mods.

Wenn Performance die Priorität ist oder du auf einer ganz frischen Minecraft-Version spielst, ist Fabric der Loader der Wahl.

### Quilt

Ein 2022er Fork von Fabric mit anderem Governance-Modell und ein paar zusätzlichen APIs. Quilt führt Fabric-Mods direkt aus, der praktische Unterschied ist klein: Quilt nimmst du, wenn ein bestimmter Mod ihn verlangt, sonst kommt Fabric aufs gleiche Ergebnis raus.

Quilt hat ein kleineres eigenes Ökosystem als Fabric, ist aber mit den meisten Fabric-Inhalten voll kompatibel.

## Kompatibilitäts-Matrix

| Mod gebaut für | Läuft auf Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Ja | Manchmal (frühes NeoForge konnte unmodifizierte Forge-Mods laufen lassen, weil es als frischer Fork startete; die APIs sind seitdem auseinandergedriftet, die meisten aktuellen Forge-Mods brauchen einen NeoForge-Build) | Nein | Nein |
| NeoForge | Nein | Ja | Nein | Nein |
| Fabric | Nein | Nein | Ja | Ja |
| Quilt | Nein | Nein | Quilt-API-Mods: nein; Rest: ja | Ja |

Eine Cross-Loader-Bridge gibt's nicht in der Praxis. Die JARs in `mods/` müssen zum Loader der Instanz passen.

## Wahl bei neuer Instanz

Meist wählen Mods oder Modpacks für dich:

- **Modpack aus CurseForge oder Modrinth installieren?** GDLauncher liest die Pack-Manifest und installiert den darin angegebenen Loader. Keine Wahl.
- **Custom-Instanz um einen einzelnen Mod aufbauen?** Mod-Seite checken. Steht "Fabric 1.21.x" da, machst du eine Fabric-1.21.x-Instanz.
- **Custom-Instanz aus mehreren Mods?** Pro Mod prüfen, für welche Loader es Builds gibt, und die Schnittmenge nehmen. Performance-Mods sind oft Fabric-only; große Content-Mods Forge/NeoForge.

Ohne Vorgabe: **Fabric** für Performance- und Optik-fokussiertes Setup, **NeoForge** für content-schweres Modded Survival.

## Loader auf bestehender Instanz wechseln

GDLauncher erlaubt es, den Loader einer Instanz nach der Erstellung zu ändern, siehe [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). Kurz: Rechtsklick auf Instanz → Edit → anderen Loader wählen. Der mods-Ordner wird nicht geleert, JARs des alten Loaders bleiben. Vor dem Start manuell die nicht-kompatiblen entfernen.

## Hinweis zu Loader-Versionen

Jeder Loader hat seine eigene Versionierung, unabhängig von Minecraft. Wer "Forge" wählt, wählt auch eine Forge-Version (für Minecraft 1.20.1 z. B. `47.2.0`). Für Mods reicht meist "gleiche Major-Version wie vom Pack erwartet", manche brauchen eine Mindest-Loader-Version. Die CurseForge- oder Modrinth-Seite des Mods sagt's.
