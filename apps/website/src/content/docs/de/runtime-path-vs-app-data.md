---
title: "Runtime Path vs App Data Path"
description: "GDLauncher nutzt zwei verschiedene Pfade zum Speichern von Daten. Was beide enthalten, warum sie getrennt sind und welcher in der Regel umgezogen werden sollte."
faq:
  - question: "Was ist der Unterschied zwischen App Data Path und Runtime Path?"
    answer: "Der App Data Path ist das Standard-Pro-User-Verzeichnis, in dem Electron seine Caches und den runtime_path_override-Marker ablegt. Der Runtime Path ist, wo GDLauncher alles Schwere lagert: Instanzen, Minecraft-Assets und -Libraries, Java-Installs, Launcher-Datenbank und app-weite Logs. Standardmäßig liegt der Runtime Path im App Data Path, aber zu verschieben ist der Runtime Path."
  - question: "Wo liegt der App Data Path?"
    answer: "Windows: C:\\Users\\<du>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<du>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, sonst ~/.local/share/gdlauncher_carbon."
  - question: "Soll ich den Runtime Path oder den App Data Path umziehen?"
    answer: "Den Runtime Path. Er wächst mit jeder Instanz. Der App Data Path bleibt klein und ist an dein OS-Profil gebunden. GDLauncher bietet den Umzug in Settings → Runtime Path; den App Data Path verwaltet Electron, ohne UI-Option zum Verschieben."
  - question: "Was macht die Datei runtime_path_override?"
    answer: "Nach einem Runtime-Path-Wechsel schreibt GDLauncher eine kleine Textdatei namens runtime_path_override in den App Data Path. Inhalt ist der neue Runtime Path; bei jedem Start liest der Launcher die Datei, um zu wissen, wo deine Daten liegen. Wird sie gelöscht, fällt der Launcher auf den Default-Runtime-Path zurück."
  - question: "Kann ich den Runtime Path zwischen zwei Rechnern teilen?"
    answer: "Nein. Die Launcher-Datenbank speichert maschinenspezifischen Zustand (Pfade, Tokens, Java-Installs) und ist nicht für parallele Nutzung ausgelegt. Wenn du dieselben Instanzen auf zwei Rechnern willst: einzeln kopieren oder Cloud Instance Share nutzen."
---

# Runtime Path vs App Data Path

## Zwei Pfade, zwei Aufgaben

GDLauncher splittet seine Dateien auf zwei Orte: einen **App Data Path** für die kleinen Electron-Sachen und einen **Runtime Path** für die schweren (Instanzen, Assets, Java, Datenbank). Den Runtime Path will man hin und wieder mal verschieben, den App Data Path normalerweise nicht.

### App Data Path

Das OS-Standard-Per-User-App-Verzeichnis, also das, worauf Electrons `userData` zeigt. GDLauncher nutzt es für:

- Den `runtime_path_override`-Marker, eine einzeilige Textdatei, die dem Launcher sagt, wo der Runtime Path tatsächlich liegt
- Den Default-Runtime-Path in einem `data/`-Unterordner, wenn du ihn nicht verschoben hast
- Electrons eigene Chromium-Caches (Network/, GPUCache/, Cookies usw.)
- Die Electron-Main-Process-Logs

Standardpfad:

- **Windows:** `C:\Users\<du>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<du>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, sonst `~/.local/share/gdlauncher_carbon`

Ohne den `data/`-Unterordner meist klein. GDLauncher bietet keine Option, dieses Verzeichnis zu verschieben; OS-Konventionen und Electron erwarten es dort.

### Runtime Path (Core Module)

Hier legt der Rust-Core alles andere ab:

- Instanzen (unter `instances/`)
- Minecrafts geteilte Assets (Mojangs Texturen, Sounds)
- Minecrafts geteilte Libraries (JARs von Mojang und den Modloadern)
- Java-Installationen, die GDLauncher heruntergeladen hat
- Launcher-Datenbank `gdl_conf.db`
- App-weite Logs in `__gdl_logs__/`

Standardmäßig `<App Data Path>/data/`, also innerhalb des App-Data-Ordners. Über **Settings → Runtime Path** lässt sich der Pfad verschieben. Bei großen Modpacks knackt diese Path schnell die 50-GB-Marke.

## Wann den Runtime Path umziehen

Zwei gängige Gründe:

1. **SSD läuft voll.** Instanzen auf eine größere HDD oder zweite SSD verschieben.
2. **Backups unabhängig vom OS-Profil halten.** Auf einem separat gebackuppten Laufwerk ist okay; nicht aktiv beim Spielen synchronisieren, sonst gibt es Datei-Lock-Konflikte.

Für den normalen Betrieb musst du nichts verschieben. Der Default passt.

## Wie der Umzug abläuft

**Settings → Runtime Path** öffnen. Neuen Pfad eintippen oder per Ordner-Icon wählen. Der Apply-Button (das Kreis-Pfeil-Icon rechts in der Zeile) wird aktiv, sobald der Pfad neu und gültig ist. Klick öffnet ein Bestätigungs-Modal, das alten und neuen Pfad zeigt.

Ist der Zielordner leer (oder existiert noch nicht), startet die volle Migration, wenn du bestätigst: Ein Overlay zeigt Scan, dateiweises Kopieren, dateiweises Entfernen der Quelle. Nicht App oder Rechner schließen, solange das läuft. Danach startet der Launcher automatisch neu.

Enthält das Ziel schon einen GDLauncher-Runtime-Path (du hast ihn früher verschoben und willst eine frische Installation darauf zeigen lassen), warnt das Modal gelb, dass der Ordner nicht leer ist. Bestätigen macht einen "Switch only": Der Marker wird umgeschrieben, keine Dateien werden kopiert, der Launcher startet neu. Die Daten am alten Ort bleiben verwaist liegen, die kannst du händisch löschen.

Bricht eine Migration mittendrin ab, wird das Overlay rot und zeigt den Fehler. Der Launcher rollt zurück: Im neuen Pfad angelegte Dateien werden entfernt, der Marker bleibt auf den alten Pfad zeigend, du kannst ohne Datenverlust neu versuchen. Häufige Ursachen sind fehlende Schreibrechte auf dem Ziellaufwerk und zu wenig freier Speicher.

### Die Marker-Datei runtime_path_override

Beim Verschieben schreibt GDLauncher eine kleine Textdatei namens `runtime_path_override` in den **App Data Path** (nicht in den Runtime Path). Inhalt ist der neue Runtime Path als Klartext. Jeden Start liest der Launcher die Datei, um zu wissen, wo deine Daten liegen.

Wird der Marker gelöscht, fällt GDLauncher auf den Default-Runtime-Path (`<App Data>/data/`) zurück. Deine Daten sind nicht weg, sie liegen noch da, wo du sie hin verschoben hast, aber der Launcher sieht sie nicht, bis du in **Settings → Runtime Path** den Ordner wieder eingibst. Da der Ordner schon GDLauncher-Daten enthält, behandelt der Launcher das als "Switch only" und aktualisiert nur den Marker, ohne zu kopieren.

## Warum die Datenbank nicht geteilt werden darf

`gdl_conf.db` im Runtime Path enthält Account-Tokens, Microsoft-Refresh-Tokens, GDL-Account-Status und Instanz-Metadaten. Sensible Daten, maschinenlokal: **nicht weitergeben.** Zwei Rechner mit derselben DB landen beide ausgeloggt, weil sie sich um die Token-Refreshes streiten.

Für gleiche Instanzen auf zwei Rechnern: die Instanzen-Ordner aus `instances/` manuell kopieren oder Cloud Instance Share verwenden.
