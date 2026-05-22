---
title: "Modpack-Manifest-Format"
description: "Was in den drei Modpack-Archiven steckt, die GDLauncher liest und schreibt: CurseForge .zip, Modrinth .mrpack und GDLaunchers eigenes Hash-basiertes .gdlpack. Feld-für-Feld-Referenz, Vergleich nebeneinander und wie jedes Format Mods bei der Installation auflöst."
faq:
  - question: "Was ist ein Modpack-Manifest?"
    answer: "Eine JSON-Datei innerhalb eines Modpack-Archivs, die dem Launcher mitteilt, was im Pack steckt: Minecraft-Version, Mod-Loader, Mod-Liste und welche mitgelieferten Dateien (Overrides) in die Instanz extrahiert werden sollen. GDLauncher liest das Manifest als Allererstes, wenn ein Pack installiert wird."
  - question: "Was ist der Unterschied zwischen CurseForge-, Modrinth- und GDLauncher-Pack-Formaten?"
    answer: "Alle drei sind Zips mit einem JSON-Manifest. CurseForge nutzt manifest.json mit CurseForge-Projekt/Datei-IDs. Modrinth nutzt modrinth.index.json mit direkten Download-URLs plus Hashes. GDLauncher nutzt gdlpack.json nur mit Inhalts-Hashes (sha512 + sha1 + murmur2) und löst jede Datei gegen die Plattform auf, die sie tatsächlich hat. Sie sind nicht austauschbar; der Launcher entscheidet anhand des JSON im Archivroot, wie er das Pack behandelt."
  - question: "Was ist eine .gdlpack und warum existiert sie?"
    answer: "GDLaunchers eigenes Modpack-Format. Jeder Mod wird über Inhalts-Hashes identifiziert, sodass ein einzelnes Pack funktioniert, egal ob die Mods auf CurseForge, Modrinth oder beidem leben, und keine URLs veralten können. Außerdem unterstützt es optionale Features (überspringbare Bündel aus Mods + Konfigs) und eingebettete Icons. Empfohlen, wenn beide Seiten GDLauncher nutzen; CurseForge .zip bleibt die sicherere Wahl für plattformübergreifende Verteilung."
  - question: "Wo finde ich das Manifest in einer installierten Instanz?"
    answer: "GDLauncher behält das Originalarchiv nach der Installation nicht. Die Pack-Inhalte landen entpackt im Instanzordner. Das Manifest, mit dem GDLauncher arbeitet, wird in der Datenbank des Launchers gegen die Instanz geführt; es ist nicht als Datei einsehbar. Um ein Manifest vor der Installation einzusehen, öffne die .zip / .mrpack / .gdlpack mit einem beliebigen Archivtool und lies die JSON-Datei darin direkt."
  - question: "Kann ich ein Manifest bearbeiten?"
    answer: "Du kannst eines im Archiv bearbeiten, wenn du dein eigenes Pack baust, aber du kannst das Manifest einer installierten Instanz nicht über die Launcher-UI bearbeiten. Um die Mods einer Instanz zu ändern, nutze den Addons-Tab (oder entsperre vorher eine gesperrte Instanz). Um die Version eines verknüpften Packs zu ändern, öffne die Instanz, gehe zum Settings-Tab und klicke auf Change Modpack Version."
  - question: "Was ist ein 'Override' in der Pack-Terminologie?"
    answer: "Dateien, die das Pack vorgebündelt mitliefert – keine Mods. Standardkonfigs, Skripte, Resource Packs, manchmal eine Starterwelt. Sie werden bei der Pack-Installation über die Mods hinweg in den Instanzordner extrahiert. CurseForge listet sie im overrides-Feld; Modrinth markiert Pack-Dateien mit env.client=required; GDLPack nutzt einen overrides-Ordner (über das Manifest konfigurierbar)."
---

# Modpack-Manifest-Format

## Warum das wichtig ist

Normalerweise musst du nicht wissen, was in einem Modpack-Archiv steckt. Aber wenn bei der Installation etwas schiefläuft, siehst du Meldungen wie „manifest", „invalid manifest format", „manifest missing field" oder „manifest version unsupported", und du willst wissen, worauf die sich beziehen. Diese Seite ist auch nützlich, wenn du ein Pack zum Teilen baust oder herausfinden willst, warum dasselbe Pack in einem Launcher sauber installiert und in einem anderen kaputtgeht.

GDLauncher liest und schreibt drei Formate:

- **CurseForge** (`.zip`, innen `manifest.json`).
- **Modrinth** (`.mrpack`, innen `modrinth.index.json`).
- **GDLauncher** (`.gdlpack`, innen `gdlpack.json`).

Alle drei sind unter der Haube einfache ZIP-Archive; die Dateiendung deutet nur an, welches Manifestschema zu erwarten ist.

## Was in einem Modpack-Archiv steckt

Die allgemeine Form ist über alle drei Formate gleich:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← formatspezifisches Manifest
├── overrides/            ← gebündelte Konfigs, Skripte, optionale Starterwelt
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (nur CurseForge, optional, menschenlesbare Modliste)
```

Das Manifest ist die einzige Datei, die der Launcher zwingend braucht, um das Pack zu installieren. Der overrides-Ordner ist Inhalt, der in die resultierende Instanz entpackt wird.

## CurseForge: `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

Wichtige Felder:

- `minecraft.version`: die Minecraft-Version, auf die das Pack zielt.
- `minecraft.modLoaders`: welcher Loader und welche Version (Format: `<loader>-<version>`).
- `files`: jeder Mod wird per CurseForge-`projectID` und `fileID` referenziert. GDLauncher löst diese gegen die CurseForge-API auf, um tatsächlich herunterzuladen.
- `overrides`: Name des Ordners im Zip, dessen Inhalt nach Abschluss der Mod-Downloads in die Instanz kopiert wird.

Wenn die `projectID` oder `fileID` einer Datei nicht mehr auf CurseForge existiert (der Autor hat sie entfernt), schlägt die Installation für diesen Mod mit einem „file not found"-Fehler fehl.

## Modrinth: `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

Wesentliche Unterschiede zu CurseForge:

- `dependencies` deklariert Minecraft- und Loader-Versionen direkt (kein verschachteltes Objekt).
- Jeder `files`-Eintrag enthält die **exakte Download-URL** und den **Hash**, sodass Installationen nicht von einem Metadaten-API-Aufruf abhängen.
- Jede Datei deklariert über `env`, ob sie auf Client, Server oder beidem erforderlich ist.

Dieses Format ist simpler und einfacher offline zu installieren (URLs sind eingebettet). Es ist auch strenger: Hash-Mismatches führen zu einer harten Installationsverweigerung, nicht zu einer Warnung.

## GDLauncher: `gdlpack.json`

GDLaunchers eigenes Pack-Format ist das, was GDLauncher schreibt, wenn du in **Export Instance** **GDLauncher .gdlpack** wählst. Die definierende Eigenschaft: jeder Mod wird nur über Inhalts-Hashes identifiziert, nicht über eine plattformspezifische ID oder URL. GDLauncher löst jeden Hash zur Installationszeit gegen CurseForge und Modrinth auf und lädt von der Plattform herunter, die die Datei tatsächlich hat.

### Warum Hashes statt IDs oder URLs

- **Plattformübergreifend aus einer Quelle.** Ein Mod, der sowohl auf CurseForge als auch auf Modrinth lebt, hat auf jeder Seite andere IDs. Ein `.gdlpack` ist das egal; eine Hash-Liste funktioniert auf beiden.
- **Keine URL-Fäule.** Modrinth-CDN-URLs sind inhaltsadressiert und stabil, aber das Einbetten von URLs fixiert einen einzigen Downloadursprung. Hashes erlauben dem Launcher Fallbacks, wenn eine Plattform down ist.
- **Verifizierung qua Konstruktion.** Jedes Byte, das in deinen `mods/`-Ordner geschrieben wird, wird gegen den Manifest-Hash geprüft. Es gibt keinen Weg, ein anderes JAR unbemerkt unterzuschieben.

### Archivstruktur

```
mypack.gdlpack/
├── gdlpack.json          ← das Manifest
├── .gdl/
│   └── icon.png          ← eingebettetes Pack-Icon (optional)
└── overrides/            ← gebündelte Dateien (Konfigs, Skripte, nicht auflösbare Mods)
    ├── config/
    └── ...
```

Das Manifest liegt im Root. Der `.gdl/`-Ordner enthält Metadaten, die der Launcher einbettet (aktuell nur das Icon). Overrides funktionieren wie bei CurseForge: Inhalt wird nach dem Mod-Auflöseschritt in die resultierende Instanz extrahiert.

### Minimalmanifest

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

Nur `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries` und `overrides` sind zum Laden erforderlich.

### Volles Manifest, annotiert

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### Top-Level-Felder

| Feld | Typ | Pflicht | Bedeutung |
|---|---|---|---|
| `formatVersion` | Integer | ja | Manifestschemaversion. Aktuell immer `1`. |
| `name` | String | ja | Pack-Anzeigename. Dient beim Import als Standard-Instanzname. |
| `version` | String | nein | Pack-Version (Semver empfohlen, z. B. `1.0.0`, `2.1.0-beta.1`). |
| `summary` | String | nein | Einzeilige Tagline. |
| `author` | String | nein | Pack-Autor oder Team. |
| `createdAt` | RFC-3339-Zeitstempel | ja | Wann das Archiv exportiert wurde. |
| `icon` | String | nein | Pfad innerhalb des Archivs zur Icon-Datei (z. B. `.gdl/icon.png`). |
| `dependencies` | Objekt | ja | Minecraft- und Modloader-Anforderungen (siehe unten). |
| `entries` | Array | ja | Plattformdateien und optionale Features (siehe unten). Leeres Array ist für ein reines Overrides-Pack gültig. |
| `overrides` | String | ja (Standard `"overrides"`) | Name des Verzeichnisses im Archiv, das in die Instanz extrahiert werden soll. |
| `serverOverrides` | String | nein | Verzeichnis mit Server-only-Overrides. |
| `clientOverrides` | String | nein | Verzeichnis mit Client-only-Overrides. |
| `source` | Objekt | nein | Wenn das Pack von einem CurseForge- oder Modrinth-Pack abgeleitet ist, verweist dies auf das Original (siehe unten). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: erforderliche Minecraft-Version (z. B. `1.20.1`, `1.21.4`, `25w20a` für Snapshots).
- `modloaders`: null oder mehr Loader-Einträge. Jeder hat:
  - `type`: einer von `forge`, `neoforge`, `fabric`, `quilt` (kleingeschrieben).
  - `version`: exakte Loader-Version, gegen die das Pack gebaut wurde.
  - `primary`: markiert den empfohlenen Loader, wenn mehrere gelistet sind (z. B. ein Pack, das sowohl mit Fabric als auch mit Quilt kompatibel ist).

Ein Vanilla-Pack hat ein leeres `modloaders`-Array.

#### `entries`

Zwei Eintragstypen, unterschieden durch das `type`-Feld:

**`platform`**, ein erforderlicher Mod, der über Hash aufgelöst wird:

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

Alle drei Hashes müssen vorhanden sein. Jeder wird bei der Auflösung anders verwendet:

| Hash | Verwendet für |
|---|---|
| `sha512` | Modrinth-Auflösung (Modrinths `version_file`-Endpunkt), primäre Integritätsprüfung der heruntergeladenen Datei. |
| `sha1` | Modrinth-Auflösung (Fallback), wird von Minecraft selbst beim Starten zur Dateiverifizierung verwendet. |
| `murmur2` | CurseForge-Auflösung (CurseForges `fingerprints`-Endpunkt). |

Bei der Installation versucht der Launcher zuerst Modrinth (sha512-Lookup), greift auf CurseForge zurück (murmur2-Fingerprint), und schlägt den Eintrag fehl, wenn keine Plattform die Datei hat. Aufgelöste Dateien werden immer von der Plattform heruntergeladen, die geantwortet hat.

**`optional`**, eine überspringbare Gruppe aus Mods und/oder Override-Pfaden, die der Nutzer einbeziehen oder ausschließen kann:

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description`: wird dem Nutzer in der Import-Vorschau angezeigt, damit er entscheiden kann, ob dieses Feature einbezogen werden soll.
- `platforms`: null oder mehr Hash-Einträge, die nur heruntergeladen werden, wenn das Feature einbezogen wird.
- `overridePaths`: null oder mehr Pfade (Dateien oder Ordner) im `overrides/`-Verzeichnis, die nur extrahiert werden, wenn das Feature einbezogen wird. Pfade sind relativ zu `overrides/`.

Ein Feature kann nur Plattformdateien, nur Overrides oder beides haben. Damit lassen sich verwandte optionale Inhalte bündeln: ein Shader-Mod plus seine Konfig, ein Hardcore-Schwierigkeits-Preset, das nur aus Konfigs besteht, ein optionales Resource Pack.

#### `source`

Wenn ein Pack aus einer bestehenden CurseForge- oder Modrinth-Modpack-Instanz exportiert wurde, vermerkt GDLauncher, woher es stammt:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

oder

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

Das ist informativ; der Launcher nutzt es nicht zum Herunterladen (dafür ist bereits das `entries`-Array zuständig). Es existiert, damit Derivate öffentlicher Packs dem Original zugerechnet bleiben.

### Gebündelt vs. nur-Manifest-Exporte

Beim Export einer Instanz nach `.gdlpack` entscheidet der **Bundle Addons**-Schalter im Export-Wizard, was wohin gelangt:

- **Bundle aus (nur Manifest).** Jeder Mod, den GDLauncher über CurseForge oder Modrinth auflösen kann, wird zu einem `platform`-Eintrag in `entries`. Die eigentliche JAR ist *nicht* im Archiv. Der Launcher des Empfängers lädt sie beim Import von der Plattform neu. Mods, die GDLauncher nicht auflösen kann (z. B. von Hand abgelegte JARs), werden als Fallback in `overrides/mods/` gebündelt. Ergebnis: kleines Archiv, der Empfänger braucht beim Installieren Internet.
- **Bundle an (eigenständig).** Es werden keine `platform`-Einträge erzeugt; jeder Mod wird in `overrides/mods/` kopiert. Ergebnis: größeres Archiv (oft hunderte MB bis mehrere GB), offline installierbar, sobald die Minecraft-Assets im Cache sind.

Resource Packs und Shader Packs verhalten sich identisch: Auflösbare werden bei Bundle aus zu `platform`-Einträgen, bei Bundle an wird alles direkt gebündelt.

## Nebeneinander: die drei Formate

| Eigenschaft | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Manifestdateiname | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Mod-Identifikation | CurseForge-Projekt + Datei-ID | Download-URL + Hash | Nur Inhalts-Hashes |
| Auflösung aus | CurseForge-API | Im Manifest eingebettete URL | CurseForge **oder** Modrinth, je nachdem, welche antwortet |
| Offline-Installation | Nein (CDN nötig) | Ja (URLs brauchen evtl. noch CDN) | Ja, wenn `Bundle Addons` aktiv |
| Optionale Features | Pro-Mod-Flag `required: false` | Pro-Datei `env` (client/server) | Dedizierter `optional`-Eintrag mit mehreren Dateien |
| Overrides-Ordner | `overrides/` | Pfad pro Datei | `overrides/` (konfigurierbar) |
| Server-only-Dateien | Eigenes Pack | `env.server` | `serverOverrides`-Verzeichnis |
| Icon | Extern | Extern | Eingebettet in `.gdl/icon.<ext>` |
| Quellenverfolgung | Keine | Keine | `source`-Feld |
| Portabilität | Am breitesten (die meisten Launcher lesen es) | Modrinth-unterstützende Launcher | GDLauncher |

## Overrides im Detail

Wenn das Manifest auf einen Overrides-Ordner oder als client-required markierte Pack-Dateien verweist, extrahiert der Launcher diese nach dem Mod-Auflöseschritt in die Instanz. So bündeln Packs:

- Standard-Mod-Konfigs (damit das Pack out-of-the-box bei allen gleich spielt).
- KubeJS- oder CraftTweaker-Skripte.
- Eine Starterwelt (gelegentlich).
- Resource Packs, die das Pack aktiv erwartet.
- `options.txt` mit pack-spezifischen Spieleinstellungen.

Overrides schlagen automatisch generierte Defaults, verlieren aber gegen alles, was der Spieler später manuell ändert. GDLPack unterstützt zusätzlich `serverOverrides`- und `clientOverrides`-Verzeichnisse für Dateien, die nur auf einer Seite landen sollen – nützlich für Packs, die einen passenden Client- und Server-Bundle ausliefern.

## Wenn Manifeste veralten

Ein Pack-Manifest ist eine Momentaufnahme dessen, was der Pack-Autor zuletzt veröffentlicht hat. Wird ein vom Manifest referenzierter Mod nach dem Bau aus der Quellplattform entfernt, schlägt die Installation dieser Pack-Version fehl, bis der Autor eine neue veröffentlicht. Das passiert, wenn eine ältere Pack-Version sauber installiert wird, eine neuere aber kaputtgeht: die neuere verweist auf etwas, das nicht mehr verfügbar ist.

Der Fix liegt beim Autor; der Launcher kann nur tun, was das Manifest sagt. GDLPack hat hier einen Teilvorteil, weil es nicht an eine Plattform gebunden ist: Wird ein Mod aus CurseForge entfernt, ist aber noch auf Modrinth (oder umgekehrt), löst dasselbe `entries`-Array immer noch auf.

## Ein Manifest selbst lesen

Du brauchst nichts Spezielles. Benenne `mypack.mrpack` oder `mypack.gdlpack` in `mypack.zip` um, öffne es mit einem beliebigen Archivtool und sieh dir die JSON-Datei darin an (`modrinth.index.json` oder `gdlpack.json`). Dasselbe gilt für CurseForge-`.zip`-Dateien und `manifest.json`. Alle drei sind reines JSON und in jedem Texteditor lesbar.

## Ein Pack bauen

Wenn du ein Pack zum Teilen baust, ist der einfachste Weg, die Instanz in GDLauncher einzurichten und dann **Export Instance** (Rechtsklick → Export Instance) zu nutzen. Der Export-Wizard lässt dich das Zielformat wählen: CurseForge `.zip`, Modrinth `.mrpack` oder GDLaunchers eigenes `.gdlpack`. Wähle `.gdlpack`, wenn der Empfänger ebenfalls GDLauncher nutzt und du optionale Features oder eingebettete Icons willst; wähle `.zip` für die breiteste plattformübergreifende Kompatibilität.
