---
title: "Aufbau eines Instanz-Ordners"
description: "Was in einem GDLauncher-Instanz-Ordner steckt. Wo Mods, Welten, Configs, Screenshots und Logs liegen, und was du gefahrlos von Hand löschen kannst."
faq:
  - question: "Wo werden meine Minecraft-Welten gespeichert?"
    answer: "Jede Welt liegt unter <runtime_path>/instances/<instance>/instance/saves/<welt-name>/. Der saves/-Ordner wird beim ersten Erzeugen oder Importieren einer Welt angelegt. Rechtsklick auf die Instanz → Open Folder, dann in instance/saves wechseln. Zum Backup den Welt-Ordner kopieren."
  - question: "Wo sind die Crash-Reports?"
    answer: "Innerhalb der Instanz: instance/crash-reports/. Der Ordner existiert nur, wenn Minecraft tatsächlich gecrashed ist; wenn das Spiel bei dir nie hart gestorben ist, ist der Ordner schlicht noch nicht da. Jeder Report ist eine Textdatei mit Zeitstempel (crash-<datum>-server.txt o. ä.)."
  - question: "Wo gehören Mod-JARs hin?"
    answer: "Nach instance/mods/. Manuelles Reinwerfen funktioniert, umgeht aber GDLaunchers Tracking, bevorzuge den Addons-Tab. Bei gesperrten Modpack-Instanzen ist Add im Addons-Tab deaktiviert; ein File-Copy ins Verzeichnis schmuggelt sich aber durch."
  - question: "Was ist der Unterschied zwischen Launcher- und Minecraft-Logs?"
    answer: "Zwei verschiedene Sätze. Die Launcher-Session-Logs liegen unter <instance>/logs/. Minecrafts eigene Game-Logs (latest.log, crash-reports, alles was das Spiel schreibt) eine Ebene tiefer in <instance>/instance/logs/ und <instance>/instance/crash-reports/."
  - question: "Was sind instance.json und packinfo.json?"
    answer: "Metadaten-Dateien, die GDLauncher auf oberster Ebene jeder Instanz schreibt. instance.json enthält Instanzname, Icon, Modloader, Minecraft-Version, letztes Spielen und gesamte Spielzeit. packinfo.json (nur bei mit einem CurseForge- oder Modrinth-Modpack gepairten Instanzen) trackt, welche Dateien zum Pack gehören, damit der Launcher Pack-Mods von eigens hinzugefügten unterscheidet. Beide nicht von Hand löschen."
---

# Aufbau eines Instanz-Ordners

## Wo der Instanz-Ordner liegt

Jede GDLauncher-Instanz lebt als Unterordner im Runtime Path:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← der Instanz-Ordner
```

`<shortpath>` ist eine bereinigte Form des Anzeigenamens. Rechtsklick im GDLauncher → **Open Folder** bringt dich direkt hin.

## Was drin ist

Der Instanz-Ordner gliedert sich in ein paar Dinge, die GDLauncher auf oberster Ebene trackt, plus einen `instance/`-Unterordner, der eigentliche Game-Verzeichnis von Minecraft. Manches ist immer da; vieles wird beim ersten Schreibzugriff erst erzeugt.

```
<shortpath>/
├── instance.json          ← GDLauncher-Metadaten zur Instanz (immer da)
├── packinfo.json          ← Modpack-Pairing-Info (nur bei gepairten Modpacks)
├── icon.png | icon.webp   ← Eigenes Icon (nur wenn gesetzt)
├── logs/                  ← GDLauncher-Per-Instanz-Logs
└── instance/              ← Minecrafts Game-Verzeichnis
    ├── mods/              ← Mod-JARs
    ├── config/            ← Mod-Configs
    ├── shaderpacks/       ← Shader Packs (falls installiert)
    ├── options.txt        ← Minecraft-Client-Einstellungen (nach erstem Start)
    ├── logs/              ← Minecraft-Session-Logs (latest.log etc.)
    ├── saves/             ← Welten (entsteht beim ersten Welt-Erzeugen)
    ├── screenshots/       ← F2-Screenshots (entsteht beim ersten F2)
    ├── crash-reports/     ← Crash-Dumps (nur wenn's einen Crash gab)
    ├── resourcepacks/     ← Eigene Resource Packs (beim Hinzufügen)
    ├── datapacks/         ← Globale Data Packs (welt-spezifische unter saves/<welt>/datapacks/)
    └── (pack-spezifisch)  ← kubejs/, defaultconfigs/, packmenu/ usw., nur wenn der Pack sie mitbringt
```

Wundere dich nicht, wenn bei einer frischen Instanz vieles fehlt. Launcher und Minecraft legen nur an, was sie brauchen, wenn sie's brauchen. Eine nie gespielte Instanz hat kein `saves/`, kein `screenshots/`, keine `options.txt`. Eine Vanilla-Instanz hat kein `mods/` und kein `config/`.

## Was die einzelnen Dinge halten

### Top-Level-Dateien

- **`instance.json`**: GDLaunchers Metadaten, Name, Icon-Pfad, Modloader und Version, Minecraft-Version, Erstellungszeit, letztes Spielen, gesamte Spielzeit. Immer da.
- **`packinfo.json`**: Hash-Manifest, welche Dateien aus dem Modpack stammen. Lässt den Launcher Pack-Mods von eigenen unterscheiden. Nur bei Instanzen, die mit einem CurseForge- oder Modrinth-Pack gepairt sind.
- **`icon.png`** oder **`icon.webp`**: das hochgeladene Custom-Icon. Bei Default-Icon nicht vorhanden.

### `logs/` (Top-Level)

GDLaunchers eigene Per-Instanz-Logs. Was du im Rechtsklick-Menü unter **View Logs** siehst. Sie protokollieren den Launch aus *Launcher*-Sicht (Java-Argumente, Asset-Downloads, Modloader-Install, Exit Code) und sind wertvoll, wenn das Spiel gar nicht erst seine eigenen Logs schreibt.

### `instance/mods/`

Die Mod-JAR-Dateien. Minecraft lädt beim Start alles hier (nach den Regeln des Mod-Loaders). Welche Mods zu einer Instanz gehören, trackt der Launcher in seiner Datenbank (geschlüsselt nach Dateiname und Content-Hash), Sidecar-Dateien gibt es nicht. Manuell reingelegte JARs werden auch erkannt, der Launcher hat dafür dann nur keine CurseForge-/Modrinth-Metadaten.

### `instance/config/`

Ein Unterordner oder eine Datei pro Mod. Hier liegen Mod-Einstellungen. Die meisten Mods schreiben `config/<modid>.toml` oder einen `config/<modid>/`-Ordner. Manuelles Editieren ist meist sicher, viele Mods lesen Änderungen beim Spielstart neu.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Asset Packs. Resource Packs ändern Texturen und Sounds, Shader Packs das Rendering (brauchen Iris/OptiFine als Mod), Data Packs liefern Rezepte, Loot, Functions. Welt-spezifische Data Packs gehören unter `saves/<welt>/datapacks/`. Diese Ordner entstehen erst, wenn du tatsächlich Inhalte dafür hast.

### `instance/saves/`

Pro Welt ein Unterordner. Drin: `level.dat` (Welt-Master), `region/` (Chunk-Daten), `playerdata/` (pro Spieler), `datapacks/` (welt-skopierte Data Packs). Für ein Welt-Backup den gesamten `<welt>/`-Ordner kopieren. `saves/` selbst entsteht beim Erzeugen der ersten Welt.

### `instance/screenshots/`

Alles, was du in-game per F2 fotografiert hast. PNGs, nach Zeitstempel benannt. Entsteht beim ersten Screenshot.

### `instance/logs/` und `instance/crash-reports/`

Minecrafts eigene Diagnose-Ausgabe. `logs/latest.log` ist immer der letzte Start (wird beim nächsten Start zu `logs/<datum>-1.log.gz` rotiert). `crash-reports/` enthält komplette Crash-Dumps und taucht erst auf, wenn's tatsächlich gecrashed hat.

### `instance/options.txt`

Minecraft-Client-Einstellungen (Grafik, Steuerung, Sound). Klartext, key=value. Editierbar, wenn man's drauf anlegt.

### Pack-spezifische Ordner

Viele große Modpacks bringen Extra-Ordner mit. Die häufigsten:

- **`kubejs/`**: KubeJS-Skripte (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Pack-Autoren nutzen das für Runtime-Anpassungen.
- **`defaultconfigs/`**: Snapshot von "wie sollten die Configs default aussehen". Das Startup-Skript des Packs kopiert fehlende Einträge bei jedem Start in `config/`.
- **`packmenu/`**: Pack-thematische Main-Menü-Assets (eigene Buttons, Hintergründe, Splash-Texte).
- **`defaultsettings/`**: Wie `defaultconfigs/`, aber für `options.txt` und Tastenbelegung.

Existieren nur, wenn der Pack sie mitbringt. Vanilla und die meisten Custom-Instanzen haben sie nicht.

## Was du löschen darfst

| Ordner | Löschen sicher? | Folge |
|---|---|---|
| `instance/mods/` (ein bestimmtes JAR) | Ja | Der Mod ist weg. Welten, die ihn nutzen, können brechen. |
| `instance/config/<modid>/` | Ja | Mod fällt beim nächsten Start auf Defaults zurück. |
| `instance/resourcepacks/`, `instance/shaderpacks/`-Inhalte | Ja | Pack weg. |
| `instance/saves/<welt>/` | Ja | Welt für immer weg. Vorher Backup machen. |
| `instance/logs/`, `crash-reports/` | Ja | Frees Plattenplatz. |
| `instance/screenshots/` | Ja | Alte Screenshots tschüss. |
| `logs/` (Launcher-Logs) | Ja | Wie oben. |
| `instance/options.txt` | Ja | Game-Settings auf Default. |
| `instance.json` | Nicht löschen | Der Launcher verliert die Instanz aus dem Tracking. |
| `packinfo.json` | Möglich (entkoppelt effektiv) | Der Launcher behandelt die Instanz nicht mehr als gepairtes Modpack. Wie der Unpair-Button im Settings-Tab der Instanz, nur unsauber. |
| Den gesamten `instance/`-Ordner | Nicht löschen | Die Instanz wird kaputt. Im Launcher Delete verwenden. |

## Hinweis zu gesperrten Modpack-Instanzen

Der Instanz-Ordner ist auf der Platte ein normaler Ordner; die Sperre, die GDLauncher für Modpack-Instanzen einsetzt, gilt nur in der UI, nicht auf Dateisystem-Ebene. Ein JAR per Hand in `instance/mods/` einer gesperrten Instanz zu kippen funktioniert, der Launcher kennt es im Addons-Tab nur nicht. So eingefügte Dateien lassen sich nur per Dateisystem entfernen. Sauberer: die Instanz öffnen, im **Settings**-Tab unter Modpack auf **Unlock** klicken.
