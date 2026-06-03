---
title: "Fehlerbehebung"
description: "Behebe häufige Probleme beim Start von GDLauncher und Minecraft. App-Datenpfad, Runtime-Pfad, Speicherorte der Logs und bewährte Lösungen."
faq:
  - question: "Wo speichert GDLauncher seine Daten?"
    answer: "Unter Windows: C:\\Users\\<du>\\AppData\\Roaming\\gdlauncher_carbon. Unter macOS: /Users/<du>/Library/Application Support/gdlauncher_carbon. Unter Linux: $XDG_DATA_HOME/gdlauncher_carbon (oder ~/.local/share/gdlauncher_carbon, falls XDG nicht gesetzt ist)."
  - question: "Wo finde ich die GDLauncher-Logs?"
    answer: "GDLauncher schreibt zwei App-Level-Logs in unterschiedliche Dateien: main.log (Electron) im App-Data-Ordner und Zeitstempel-Dateien <timestamp>.log im __gdl_logs__-Ordner des Runtime-Pfads (Rust-Core; die letzten 10 werden behalten). Beim Melden von Problemen beide senden. Genaue Pfade stehen im Share-App-Logs-Guide."
  - question: "GDLauncher startet nicht. Was nun?"
    answer: "Prüfe zuerst die Logs im Datenordner auf Fehlermeldungen. Häufige Ursachen: beschädigte Runtime, ein Antivirenprogramm blockiert die ausführbare Datei oder ein nur teilweise installiertes Update. Eine saubere Neuinstallation von GDLauncher und das Wiederherstellen der Instanzen behebt meist beides."
  - question: "Warum stürzt mein Modpack beim Start ab?"
    answer: "Die meisten Startabstürze entstehen durch eine Inkompatibilität von Minecraft-Version, Modloader und Mods. Schau in die neueste Datei in __gdl_logs__ nach dem Fehler. Wird ein einzelner Mod genannt, ist das meist die Ursache, deaktiviere ihn im Tab Addons und starte neu. Bei einem OutOfMemoryError erhöhe den Arbeitsspeicher in den Instanzeinstellungen."
  - question: "Wie verschiebe ich GDLauncher auf ein anderes Laufwerk oder einen anderen Ordner?"
    answer: "Öffne Einstellungen → Allgemein → Runtime-Pfad. Ändere den Pfad auf den neuen Speicherort und GDLauncher migriert deine Instanzen und Downloads automatisch. Die Migration läuft einmal beim nächsten Start."
  - question: "Kann ich GDLauncher offline verwenden?"
    answer: "Bereits installierte Instanzen kannst du offline spielen. Die Authentifizierung erfordert einmalig eine Online-Anmeldung (Microsoft-Konto), und das Herunterladen neuer Mods oder Modpacks benötigt eine Internetverbindung."
---

## App-Datenpfad

Dies ist der Pfad, unter dem GDLauncher die Daten von Electron sowie standardmäßig den Runtime-Pfad des Core-Moduls speichert.

### Windows

`C:\Users\\{{Dein Benutzername}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Dein Benutzername}}/Library/Application Support/gdlauncher_carbon`

### Linux

- wenn die Umgebungsvariable `$XDG_DATA_HOME` gesetzt ist: `$XDG_DATA_HOME/gdlauncher_carbon`
- ansonsten: `{{homedir}}/.local/share/gdlauncher_carbon`

[Mehr Details zu homedir](https://nodejs.org/api/os.html#oshomedir)

## Runtime-Pfad des Core-Moduls

Dies ist der Pfad, unter dem das Core-Modul alle Daten speichert, einschließlich aller Instanzen, Assets und Bibliotheken.
Er liegt in der Regel im selben Verzeichnis wie der App-Datenpfad, im Unterordner `data`, sofern du keinen anderen Speicherort festlegst.

### App-Datenbank

Die App-Datenbank befindet sich im Runtime-Pfad des Core-Moduls und ist eine SQLite-Datenbankdatei mit dem Namen `gdl_conf.db`.

**SENDE DIESE DATEI NIEMANDEM, SIE ENTHÄLT SENSIBLE DATEN.**

### App-Logs

GDLauncher schreibt zwei App-Level-Logs in unterschiedliche Dateien. Beim Support **immer beide** senden, die zwei Prozesse des Launchers reichen Arbeit aneinander weiter, und die Ursache eines Fehlers auf einer Seite zeigt sich oft im Log der anderen Seite.

- **`main.log`** im App Data Path: das Log des Electron-Hauptprozesses. Erfasst Fenstererzeugung, IPC, Auto-Update, native Dialoge und harte Abstürze der Desktop-Shell.
- **`__gdl_logs__/<timestamp>.log`** im Core Module Runtime Path: das Log des Rust-Cores. Erfasst Account-Login, Asset-Downloads, Installation der Mod-Loader, Instanz-Starts, Settings-Änderungen. Die letzten 10 werden behalten.

Per-OS-Pfade und Screenshots im [Share App Logs](/guides/share-app-logs)-Guide.

**LOGS KÖNNEN SENSIBLE DATEN ENTHALTEN. SEI BEIM TEILEN VORSICHTIG.**

### Runtime-Pfad ändern

Wenn du den Runtime-Pfad änderst, verschiebt die App automatisch alle deine Instanzen und Konfigurationsdateien an den neuen Ort.

Wird der Zielordner bereits genutzt, ändert die App lediglich die Runtime-Pfad-Konfiguration; es werden keine Dateien verschoben oder kopiert.

#### Migrationsfehler

Schlägt die Migration fehl, zeigt die App eine Fehlermeldung an.

Versuche zunächst zu verstehen, was die Meldung bedeutet.
Wenn alle Dateien erfolgreich kopiert wurden, ist die Migration vermutlich beim Löschen der alten Dateien fehlgeschlagen. Schließe die App und entferne die alten Dateien manuell.

Achte darauf, die Datei `runtime_path_override` im alten Runtime-Pfad NICHT ZU LÖSCHEN, sie wird von der App benötigt, um eine Pfadänderung zu erkennen.

Bist du dir unsicher, tritt unserem [Discord-Server](https://discord.gdlauncher.com) bei und frag nach Hilfe.
