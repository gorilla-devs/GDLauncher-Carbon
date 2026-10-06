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

Lies die Fehlermeldung und beachte, welcher Schritt fehlgeschlagen ist. Bewahre den alten Runtime-Ordner, das Ziel und alle Wiederherstellungskopien auf, bis du den Zielinhalt und den nach einem Neustart tatsächlich verwendeten Runtime-Pfad geprüft hast. Eine abgeschlossene Kopie beweist nicht, dass der Pfadzeiger veröffentlicht wurde. Ältere Versionen konnten kopierte Zieldateien nach einem späteren Fehler zurückrollen; das Löschen der Quelle konnte dann die einzige nutzbare Kopie entfernen.

**Zeigt Settings → Runtime Path nach dem Neustart weiterhin den alten Ort**, wurde der Umzug vor dem Ende abgebrochen und deine Daten sind am alten Ort sicher. Der Launcher entfernt, was er in den neuen Ordner kopiert hatte; behebe also die Ursache aus der Fehlermeldung (zum Beispiel Platz auf dem Ziellaufwerk freigeben) und starte den Umzug erneut, an denselben Pfad oder in einen anderen leeren Ordner. Konnte er einige dieser Dateien nicht entfernen (etwa weil ein anderes Programm sie geöffnet hatte), behält der neue Ordner sie, markiert durch eine Datei `.gdl_migration_in_progress`. Der Launcher verwendet diesen Ordner nicht und zieht auch nicht erneut dorthin um; wählst du ihn in den Settings, wird er als unfertiger Umzug gemeldet. Du kannst diesen Ordner löschen und den Umzug dann erneut starten.

Scheitert ein Umzug, bevor der Runtime-Zeiger veröffentlicht ist, entfernt der aktuelle Launcher die Zieldaten, die er kopiert hat; was er nicht entfernen kann, bleibt mit `.gdl_migration_in_progress` markiert, und die alte Runtime bleibt ausgewählt. Nach Veröffentlichung des Zeigers ist das Ziel ausgewählt, sofern kein expliziter Override Vorrang hat; spätere Warnungen rollen es nicht zurück. Danach löscht der Launcher jedes Original in der alten Runtime, sobald er geprüft hat, dass die Datei noch ihrer Kopie entspricht; der Umzug gibt also den Platz auf dem alten Laufwerk frei. Eine Datei, die sich nach dem Kopieren geändert hat, bleibt in der alten Runtime, ebenso eine, die sich nicht löschen ließ (ein anderes Programm hatte sie geöffnet, oder dir fehlt die Berechtigung). Der Launcher startet trotzdem neu und listet diese Dateien beim ersten Start vom neuen Ort in zwei Gruppen auf. Geänderte Dateien können neuer sein als ihre Kopie am neuen Ort: Brauchst du eine davon, kopiere sie an dieselbe Stelle am neuen Ort, bevor du spielst. Nicht gelöschte Dateien entsprechen ihrer Kopie, du kannst sie also löschen. Kann der Launcher nicht bestätigen, dass der neue Ort auf dem Datenträger gespeichert wurde, löscht er nichts und meldet das nach dem Neustart; sobald du geprüft hast, dass deine Instanzen am neuen Ort sind, kannst du den alten Ordner löschen.

Der Zeiger `runtime_path_override` liegt in Electrons `userData`-Ordner (den Anwendungsdaten des Launchers), nicht in der alten Runtime. Bewahre ihn auf. Ein expliziter Runtime-Pfad auf der Kommandozeile hat Vorrang vor einem Umgebungs-Pfad; beide haben Vorrang vor dem Zeiger. Prüfe den effektiven Ort in der Runtime-Pfadeinstellung des Launchers. Verlangt der Wiederherstellungsdialog Restart oder Retry, folge dieser Aktion und behalte bei erneutem Fehler beide Ordner.

Bist du dir unsicher, tritt unserem [Discord-Server](https://discord.gdlauncher.com) bei und frag nach Hilfe.
