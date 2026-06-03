---
title: "Gesperrte Modpack-Instanzen"
description: "Was es heißt, wenn eine Modpack-Instanz gesperrt ist, warum GDLauncher sie sperrt und wie du sie entsperrst (Unlock) oder entkoppelst (Unpair), wenn nötig."
---

## Was ist eine gesperrte Instanz?

Wenn du ein Modpack aus CurseForge oder Modrinth installierst, ist die Instanz standardmäßig **gesperrt (locked)**. Neben der Instanz erscheint ein Schloss-Icon, und Aktionen, die den Pack-Inhalt verändern würden, einzelne Mods hinzufügen, entfernen oder aktualisieren, sind deaktiviert. Du kannst die Instanz weiterhin starten, Java- oder RAM-Einstellungen ändern, Screenshots machen und alles andere; das Schloss schützt nur das *vom Pack verwaltete Mod-Set*.

Das Schloss existiert, weil ein Modpack eine getestete, versionsgepinnte Sammlung von Mods ist. Pack-Autoren stellen ihre Mod-Liste bewusst zusammen und pinnen bestimmte Versionen für Kompatibilität. Tauschst du einen Mod gegen eine neuere Version, kann das einen anderen Mod brechen, der von der älteren Version abhing. Das Schloss fängt diesen Fehler ab, bevor er passiert.

## Was während des Locks geht, und was nicht

Während die Instanz gesperrt ist, **kannst** du:

- Die Instanz starten und spielen.
- RAM, Java-Argumente und Java-Override ändern.
- Screenshots machen und Logs durchsehen.
- Name und Icon der Instanz ändern (Edit Instance).
- Das gesamte Modpack auf ein neueres Release aktualisieren (Settings → Change Modpack Version).

Was du **nicht** kannst:

- Etwas über den Addons-Tab hinzufügen, das umfasst **Mods, Shader, Resource Packs, Data Packs und Worlds**. Solange das Schloss aktiv ist, ist der Add-Button bei allen Addon-Arten deaktiviert.
- Pack-verwaltete Mods oder Addons entfernen oder deaktivieren.
- Pack-verwaltete Mods einzeln auf neuere Versionen aktualisieren.

Im Mods- und Addons-Tab steht neben deaktivierten Aktionen der Hinweis „Diese Instanz ist gesperrt, Änderungen können nicht angewendet werden". Auch der Install-Button im Addons-Browser ist auf gesperrte Instanzen blockiert.

## Drei Zustände: Locked / Unlocked / Unpaired

Die drei Begriffe tauchen im GDLauncher auf und sind nicht dasselbe.

- **Locked (gesperrt)**: Die Instanz ist mit einem CurseForge- oder Modrinth-Modpack verknüpft, das vom Pack verwaltete Mod-Set ist read-only. Standard nach der Installation.
- **Unlocked (entsperrt)**: Weiterhin mit dem Modpack verknüpft (Pack-Name und -Version werden weiter geführt), aber das Mod-Set ist frei editierbar. GDLauncher merkt sich das Pack, du kannst also später noch auf eine neuere Version updaten, bist aber selbst dafür verantwortlich, das Mod-Set konsistent zu halten.
- **Unpaired (entkoppelt)**: Keine Verbindung zum Modpack mehr. Die Instanz wird zur Custom-Instanz, gleiche Dateien, aber GDLauncher verfolgt keine Pack-Updates und behandelt sie nicht mehr als Modpack-Instanz. Von Unlocked nach Unpaired ist eine Einbahnstraße.

## So entsperrst du eine Instanz (Unlock)

1. Instanz öffnen und auf das Zahnrad-Icon klicken (oder rechtsklick auf die Instanz → Settings).
2. Im Bereich **Modpack Info** ganz oben auf der Settings-Seite siehst du Pack-Icon, -Name und -Version, darunter eine Reihe Buttons.
3. Auf den **Unlock**-Button klicken (mit Schloss-Icon und „Unlock"). Die Instanz wechselt sofort in den entsperrten Zustand.

Nach dem Entsperren wird die Sektion zu „Unlocked" mit dem geöffneten Schloss. Du kannst über denselben Weg wieder sperren, in der Praxis gibt es aber wenig Grund, das nach eigenen Edits zu tun.

## So entkoppelst du eine Instanz (Unpair)

1. In derselben Modpack-Info-Sektion auf den **Unpair**-Button klicken (mit dem git-branch-Icon).
2. Im Modal bestätigen. GDLauncher warnt, dass die Aktion permanent ist.

Nach dem Entkoppeln verschwindet die Modpack-Info-Sektion komplett. Die Instanz ist jetzt eine Custom-Instanz, **Change Modpack Version** und **Reinstall** stehen nicht mehr zur Verfügung.

## Reinstall vs Unlock

In der Modpack-Info-Sektion gibt es auch eine **Reinstall**-Aktion. Sie ist getrennt vom Unlock und hat einen anderen Zweck: Sie installiert das Modpack in der aktuellen Version neu und überschreibt die vom Pack verwalteten Mods und Configs gemäß Manifest. Nutze sie, um eine kaputte Installation zu reparieren (zerstörter Mod-Jar, Configs hin), ohne deine Welten zu verlieren.

| Aktion | Effekt auf Pack-Mods | Pack-Verknüpfung |
|--------|----------------------|------------------|
| Unlock | Bleibt, aber editierbar | Bleibt |
| Unpair | Bleibt als Dateien, aber keine „Pack-Mods" mehr | Aufgelöst |
| Reinstall | Reset auf Manifest-Version | Bleibt |
| Change Modpack Version | Ersetzt durch neues Manifest | Bleibt (neue Version) |

## Wann entsperren, und wann nicht

Entsperren, wenn:
- Ein bestimmter Pack-Mod einen kritischen Bug oder Security-Fix hat und das Pack noch nicht aktualisiert wurde.
- Du einen eigenen Mod, Shader, Resource Pack, Data Pack oder eine World zusätzlich zum Pack-Inhalt installieren willst, der Add-Button im Addons-Tab ist durch das Schloss deaktiviert, also musst du zum Installieren über die UI entsperren.
- Du ein nicht mehr gepflegtes Pack selbst weiterführst.

Gesperrt lassen, wenn:
- Das Pack aktiv gepflegt wird, überlass dem Author das Versions-Pinning und warte aufs nächste Release.
- Du eine kuratierte Erfahrung spielen willst und nicht vom Mod-Set abweichen möchtest.

Üblicher Ablauf: kurz entsperren, eigene Sachen installieren, dann einfach entsperrt lassen. Was du selbst hinzugefügt hast, bleibt auch nach erneutem Sperren erhalten, das Schloss betrifft nur das *Pack-verwaltete* Set. In der Praxis gibt es aber wenig Grund, nach eigenen Edits wieder zu sperren.

## Was das Schloss nicht ist

Das Schloss ist kein Berechtigungssystem und keine Security-Grenze. Es ist eine Leitplanke gegen versehentliche Mod-Edits in der GDLauncher-UI. Der Instanz-Ordner auf der Platte ist ein normaler Ordner, alles, was direkt in das `mods`-Verzeichnis schreibt (Dritt-Tools, manuelles Kopieren), umgeht das Schloss komplett.

Solche manuell platzierten Jars erscheinen im Mods-Tab neben den Pack-Mods. Sie wieder loszuwerden geht nur über das Dateisystem, nicht über die UI.

## Schnelle Fehlerbehebung

- **„Ich kann einen einzelnen Mod nicht aktualisieren."** Das Schloss arbeitet wie geplant. Entweder Unlock (Settings → Unlock) oder Change Modpack Version, um das ganze Pack zu aktualisieren.
- **„Update All ist auf einer gesperrten Instanz ausgegraut."** Gleicher Grund. Change Modpack Version verwenden oder vorher Unlock.
- **„Mein selbst hinzugefügter Mod taucht nach Relock immer noch im Mods-Tab auf."** Das Schloss greift nur bei Pack-Mods. User-Mods bleiben sichtbar.
- **„Reinstall hat eine selbst editierte Config überschrieben."** Erwartet. Reinstall setzt auf das Pack-Manifest zurück. Vor dem Reinstall editierte Configs sichern.
