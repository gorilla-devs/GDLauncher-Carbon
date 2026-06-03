---
title: "Save-Format-Kompatibilität"
description: "Warum das Hochziehen einer Minecraft-Welt auf eine neue Version meist eine Einbahnstraße ist, wie sich das Save-Format konkret ändert, und wie du vorher richtig sicherst."
faq:
  - question: "Kann ich eine Minecraft-1.21-Welt in 1.20 öffnen?"
    answer: "Nicht ohne Risiko. Minecraft migriert Welten nur vorwärts, nie zurück. Eine in 1.21 geöffnete Welt bekommt level.dat und Region-Dateien im neueren Format neu geschrieben; ältere Versionen verweigern den Load oder crashen. Wenn du beide brauchst, vor dem Start mit der neueren Version ein Backup der Welt anlegen."
  - question: "Warnt GDLauncher vor dem Welt-Upgrade?"
    answer: "Der Launcher selbst nicht, die Warnung kommt von Minecraft. Beim Öffnen einer Welt aus einer älteren Version zeigt Minecraft einen Dialog 'Diese Welt wurde mit einer anderen Version gespeichert', bevor sie lädt. Das ist der Moment, abzubrechen und den Welt-Ordner woanders hinzukopieren."
  - question: "Was wird beim Upgrade neu geschrieben?"
    answer: "level.dat (Welt-Metadaten), die Region-Dateien in region/ (Chunk-Daten), playerdata/ (Spielerzustände) und alle welt-spezifischen Data Packs. Das Feld Data Version in level.dat wird auf den Wert der neuen Minecraft-Version gesetzt; dieses Feld lesen neuere/ältere Versionen, um zu entscheiden, ob sie die Welt öffnen können."
  - question: "Ist ein Downgrade unmöglich?"
    answer: "Strenggenommen ja. Es gibt keinen offiziellen Downgrade-Pfad. Manche Community-Tools behaupten, die Data Version zurückzudrehen, schreiben aber Chunks nicht wirklich neu, sodass die Welt teilweise korrupt zurückbleibt (neue Biome, Blöcke, Entities, die die alte Version nicht kennt). Behandle Upgrades als Einbahnstraße."
  - question: "Wie sichere ich eine Welt vor dem Upgrade?"
    answer: "In GDLauncher Rechtsklick auf die Instanz → Open Folder. Nach instance/saves wechseln und den Welt-Ordner (heißt wie die Welt) irgendwohin außerhalb der Instanz kopieren. Die Kopie behalten, bis du sicher bist, dass die neue Version sauber läuft."
---

# Save-Format-Kompatibilität

## Warum sich Save-Formate ändern

Das Welt-Dateiformat von Minecraft ist nicht fix. Jede große Minecraft-Version überarbeitet die Struktur auf der Platte. Neue Blöcke brauchen neue IDs, neue Entities neue NBT-Shapes, neue Biome neue Registries. Im Hintergrund hat jede Welt eine Zahl namens **Data Version** in `level.dat`, und Minecraft entscheidet darüber, was beim Öffnen passiert.

Ist die Data Version der Welt älter als die der aktuellen Minecraft-Version, läuft ein einmaliger **DataFixer**-Pass und schreibt die Welt ins neue Format um. Chunks, Entities, Block-States, Player-Daten, alles bekommt die neue Form. Die Data Version in `level.dat` wird auf den neuen Wert gesetzt.

Diese Umwandlung ist **destruktiv und Einbahnstraße**. Sobald Chunks umgeschrieben sind, kann die ältere Minecraft-Version sie nicht mehr lesen.

## Was "Einbahnstraße" praktisch heißt

Du hast eine 1.20.1-Welt. Du öffnest sie in 1.21. Minecraft zeigt die "andere Version"-Warnung, du klickst "Konvertieren" (oder lädst trotzdem), das Spiel startet. Im Hintergrund:

- `level.dat` wird so neu geschrieben, dass das `DataVersion`-Feld auf den 1.21-Wert statt 1.20.1 zeigt.
- Jede Region-Datei in `region/`, die geladen wird (mindestens alles in Sichtweite), wird Chunk für Chunk umgeschrieben.
- Neue 1.21-Blöcke wie Crafter oder Trial Spawner können nun in der Welt existieren, sie existieren in 1.20.1s Block-Registry nicht.
- Vorhandene 1.20.1-Entities und -Tile Entities werden auf 1.21-Schemata migriert.

Wenn du nun versuchst, denselben Ordner in 1.20.1 zu öffnen:

- Minecraft vergleicht `DataVersion` mit dem eigenen und weigert sich zu laden (oder crasht beim Laden bestimmter Chunks).
- Selbst mit umgangener Version-Prüfung würden 1.21-only-Blöcke in der älteren Version als Missing-/Error-Blöcke erscheinen.

Deshalb: **Welt-Upgrades sind permanent**. Der einzige sichere Rollback geht über ein *vorher* angelegtes Backup.

## Modded Welten machen es schlimmer

Vanilla-Minecrafts DataFixer ist immerhin gründlich und gut getestet. Modded Saves haben eine zusätzliche Risikoschicht:

- Entfernte Mods hinterlassen **Missing-Block-** und **Missing-Entity-**Fehler. Die Welt lädt, aber Würfel, die mal Mod-Blöcke waren, werden zu "?"-Platzhaltern.
- Ausgetauschte Mods (alt → neu) ändern manchmal Block-IDs oder Entity-NBT-Keys. Die Migration liegt beim Mod-Autor und ist nicht immer glatt.
- Große Minecraft-Sprünge innerhalb eines Modpacks (Forge 1.20.1 → 1.21.x z. B.) fallen meist mit einer Welle von Mod-Umstellungen auf neue APIs zusammen. Welten, die unter Alt liefen, können unter Neu undefiniert reagieren.

Bei modded Instanzen: Versionsprung als potenzielles Korruptions-Event behandeln, vorher sichern.

## Welt richtig sichern

Einfachstes Backup ist eine Ordner-Kopie. In GDLauncher:

1. Rechtsklick auf Instanz → **Open Folder**.
2. `instance/saves/` öffnen.
3. Den Welt-Ordner (heißt wie die Welt in der Liste) irgendwohin außerhalb der Instanz kopieren. Eine andere Platte, `~/Documents/mc-backups/`, irgendwas, das nicht überschrieben wird.

Diese Kopie ist ein Snapshot der Welt zum Zeitpunkt der Kopie. Aufheben, bis die neue Version sicher rund läuft.

Für laufende Backups gibt's Third-Party-Tools wie FTBBackups (ein Mod), die in-game in Intervallen Snapshots ziehen. Schreiben nach `backups/` in der Instanz, Snapshot-weise wiederherstellbar.

## Was "Snapshot-Versions"-Warnungen bedeuten

Eine in einem Minecraft-Snapshot (Entwicklerversion wie `24w11a`) gespeicherte Welt zeigt eine zusätzliche Warnung, weil Snapshot-Data-Versions manchmal weiter als alle Releases sind. Eine Snapshot-Welt kann im nächsten Stable-Release nicht öffnbar sein, wenn der Snapshot Format-Änderungen hatte, die vor dem Release zurückgerollt wurden. Sichere Wege: wichtige Welten nicht in Snapshots spielen, oder akzeptieren, dass die Welt Snapshot-locked ist.

## TL;DR

- Welt-Upgrades sind Einbahnstraße; vor dem Öffnen in neuerer Version sichern.
- Modded Welten sind besonders fragil; jeden Versionsprung als potenzielles Korruptions-Event behandeln.
- Bei Modpack-Updates, die Minecraft-Versionen springen: erst den ganzen saves-Ordner kopieren, dann updaten.
