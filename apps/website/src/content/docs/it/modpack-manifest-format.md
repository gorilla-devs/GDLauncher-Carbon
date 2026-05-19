---
title: "Formato del manifest dei Modpack"
description: "Cosa c'è dentro i tre archivi Modpack che GDLauncher legge e scrive: CurseForge .zip, Modrinth .mrpack, e il .gdlpack basato su hash di GDLauncher. Riferimento campo per campo, confronto affiancato, e note su come ognuno risolve le Mod al momento dell'install."
faq:
  - question: "Cos'è un manifest di Modpack?"
    answer: "Un file JSON dentro un archivio Modpack che dice al launcher cosa contiene il pack: versione di Minecraft, mod loader, lista delle Mod, e quali file inclusi (override) estrarre nell'istanza. GDLauncher legge il manifest prima di fare qualunque altra cosa quando installa un pack."
  - question: "Qual è la differenza tra i formati CurseForge, Modrinth e GDLauncher?"
    answer: "Tutti e tre sono zip con un manifest JSON. CurseForge usa manifest.json con project/file ID di CurseForge. Modrinth usa modrinth.index.json con URL di download diretti più hash. GDLauncher usa gdlpack.json con solo hash del contenuto (sha512 + sha1 + murmur2) e risolve ogni file su qualunque piattaforma effettivamente lo abbia. Non sono intercambiabili; il launcher decide in base a quale JSON trova alla radice dell'archivio."
  - question: "Cos'è un .gdlpack e perché esiste?"
    answer: "È il formato Modpack proprio di GDLauncher. Ogni Mod è identificata da hash del contenuto, così un singolo pack funziona indipendentemente dal fatto che le Mod vivano su CurseForge, Modrinth o entrambi, senza URL che vanno a scadere. Ha anche supporto nativo per feature opzionali (bundle di Mod + configurazioni saltabili) e icone embedded. Raccomandato quando entrambe le parti usano GDLauncher; CurseForge .zip resta la scelta più sicura per la distribuzione cross-launcher."
  - question: "Dove trovo il manifest in un'istanza installata?"
    answer: "GDLauncher non conserva l'archivio originale dopo l'install. I contenuti del pack finiscono spacchettati nella cartella dell'istanza. Il manifest da cui GDLauncher lavora è tracciato nel database del launcher contro l'istanza; non è navigabile come file. Per ispezionare un manifest prima di installare, apri il .zip / .mrpack / .gdlpack con qualunque tool di archivio e leggi direttamente il file JSON dentro."
  - question: "Posso modificare un manifest?"
    answer: "Puoi modificarne uno dentro un archivio se stai costruendo il tuo pack, ma non puoi modificare il manifest di un'istanza installata dalla UI del launcher. Per cambiare quali Mod ha un'istanza, usa la tab Addons (o sblocca prima un'istanza bloccata). Per cambiare la versione di un pack accoppiato, apri l'istanza, vai sulla tab Settings e clicca Change Modpack Version."
  - question: "Cos'è un 'override' nella terminologia dei pack?"
    answer: "File che il pack include preconfezionati, non Mod. Configurazioni di default, script, Resource Pack, a volte una mappa iniziale. Vengono estratti sopra le Mod nella cartella dell'istanza quando il pack si installa. CurseForge li elenca sotto il campo overrides; Modrinth marca i file del pack con env.client=required; GDLPack usa una cartella overrides (configurabile via manifest)."
---

# Formato del manifest dei Modpack

## Perché importa

Di solito non hai bisogno di sapere cosa c'è dentro un archivio Modpack. Ma quando qualcosa va storto durante l'install vedrai messaggi come "manifest", "invalid manifest format", "manifest missing field" o "manifest version unsupported", e vorrai sapere a cosa si riferiscono. Questa pagina è anche utile se stai costruendo un pack da condividere, o se cerchi di capire perché lo stesso pack si installa bene in un launcher e si rompe in un altro.

GDLauncher legge e scrive tre formati:

- **CurseForge** (`.zip`, `manifest.json` dentro).
- **Modrinth** (`.mrpack`, `modrinth.index.json` dentro).
- **GDLauncher** (`.gdlpack`, `gdlpack.json` dentro).

Tutti e tre sono archivi ZIP normali sotto il cofano; l'estensione del file suggerisce solo quale schema di manifest aspettarsi.

## Cosa c'è in un archivio Modpack

La forma generale è la stessa in tutti e tre i formati:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← manifest specifico del formato
├── overrides/            ← configurazioni incluse, script, mappa iniziale opzionale
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (solo CurseForge, opzionale, lista Mod leggibile)
```

Il manifest è l'unico file che il launcher deve conoscere per forza per installare il pack. La cartella overrides è contenuto da spacchettare nell'istanza risultante.

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

Campi chiave:

- `minecraft.version`: la versione di Minecraft a cui il pack mira.
- `minecraft.modLoaders`: quale loader e quale versione (il formato è `<loader>-<version>`).
- `files`: ogni Mod è referenziata per `projectID` e `fileID` di CurseForge. GDLauncher risolve questi contro l'API di CurseForge per scaricare davvero.
- `overrides`: nome della cartella dentro lo zip i cui contenuti vengono copiati nell'istanza dopo che i download delle Mod sono completi.

Se `projectID` o `fileID` di un file non esiste più su CurseForge (l'autore l'ha rimosso), l'install fallisce con un errore "file not found" per quella Mod specifica.

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

Differenze chiave rispetto a CurseForge:

- `dependencies` dichiara le versioni di Minecraft e del loader direttamente (nessun oggetto annidato).
- Ogni entry di `files` include l'**URL di download esatto** e l'**hash**, quindi le install non dipendono da una chiamata all'API dei metadati.
- Ogni file dichiara se è richiesto su client, server o entrambi via `env`.

Questo formato è più semplice e più facile da installare offline (gli URL sono cotti dentro). È anche più rigido, i mismatch di hash significano un rifiuto duro di installare invece che un warning.

## GDLauncher: `gdlpack.json`

Il formato Modpack proprio di GDLauncher è il formato che GDLauncher scrive quando scegli **GDLauncher .gdlpack** in **Export Instance**. La proprietà che lo definisce: ogni Mod è identificata solo dai suoi hash di contenuto, non da un ID specifico di piattaforma né da un URL. GDLauncher risolve ogni hash contro CurseForge e Modrinth al momento dell'install e scarica dalla piattaforma che effettivamente ha il file.

### Perché hash invece di ID o URL

- **Cross-piattaforma da una sola sorgente.** Una Mod che vive sia su CurseForge sia su Modrinth ha ID diversi per ciascuna parte. Un `.gdlpack` non se ne cura; una lista di hash funziona su entrambi.
- **Nessun URL rot.** Gli URL del CDN di Modrinth sono content-addressed e stabili, ma aggiungere URL hard-coda una singola origine di download. Gli hash permettono al launcher di fare fallback quando una piattaforma è giù.
- **Verifica per costruzione.** Ogni byte che viene scritto nella tua cartella `mods/` viene confrontato con l'hash del manifest. Non c'è modo di sostituire silenziosamente un JAR diverso.

### Layout dell'archivio

```
mypack.gdlpack/
├── gdlpack.json          ← il manifest
├── .gdl/
│   └── icon.png          ← icona del pack embedded (opzionale)
└── overrides/            ← file inclusi (configurazioni, script, mod non risolvibili)
    ├── config/
    └── ...
```

Il manifest sta alla radice. La cartella `.gdl/` contiene metadati che il launcher embedda (al momento solo l'icona). Gli override funzionano come in CurseForge: i contenuti vengono estratti nell'istanza risultante dopo lo step di risoluzione delle Mod.

### Manifest minimo

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

Solo `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries` e `overrides` sono richiesti per il caricamento.

### Manifest completo, annotato

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

#### Campi di primo livello

| Campo | Tipo | Richiesto | Significato |
|---|---|---|---|
| `formatVersion` | integer | sì | Versione dello schema del manifest. Attualmente sempre `1`. |
| `name` | string | sì | Nome visibile del pack. Usato come nome di default dell'istanza all'import. |
| `version` | string | no | Versione del pack (semver consigliato, es. `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | no | Tagline su una riga. |
| `author` | string | no | Autore o team del pack. |
| `createdAt` | timestamp RFC 3339 | sì | Quando l'archivio è stato esportato. |
| `icon` | string | no | Percorso dentro l'archivio del file icona (es. `.gdl/icon.png`). |
| `dependencies` | object | sì | Requisiti di Minecraft e modloader (vedi sotto). |
| `entries` | array | sì | File delle piattaforme e feature opzionali (vedi sotto). Un array vuoto è valido per un pack solo-override. |
| `overrides` | string | sì (default `"overrides"`) | Nome della directory dentro l'archivio da estrarre nell'istanza. |
| `serverOverrides` | string | no | Directory di override solo-server. |
| `clientOverrides` | string | no | Directory di override solo-client. |
| `source` | object | no | Se questo pack è derivato da un pack CurseForge o Modrinth, questo punta all'originale (vedi sotto). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: versione di Minecraft richiesta (es. `1.20.1`, `1.21.4`, `25w20a` per le snapshot).
- `modloaders`: zero o più entry di loader. Ognuna ha:
  - `type`: uno tra `forge`, `neoforge`, `fabric`, `quilt` (lowercase).
  - `version`: versione esatta del loader contro cui il pack è stato costruito.
  - `primary`: marca il loader raccomandato quando ne sono elencati più (es. un pack compatibile sia con Fabric che con Quilt).

Un pack Vanilla ha un array `modloaders` vuoto.

#### `entries`

Due tipi di entry, distinti dal campo `type`:

**`platform`**, una Mod richiesta risolta tramite hash:

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

Tutti e tre gli hash devono essere presenti. Ognuno è usato diversamente al momento della risoluzione:

| Hash | Usato per |
|---|---|
| `sha512` | Risoluzione Modrinth (endpoint `version_file` di Modrinth), check di integrità primario sul file scaricato. |
| `sha1` | Risoluzione Modrinth (fallback), usato da Minecraft stesso quando verifica il file al launch. |
| `murmur2` | Risoluzione CurseForge (endpoint `fingerprints` di CurseForge). |

Al momento dell'install, il launcher prova prima Modrinth (lookup sha512), fa fallback su CurseForge (fingerprint murmur2), e fa fallire l'entry se nessuna piattaforma ha il file. I file risolti vengono sempre scaricati dalla piattaforma che ha risposto.

**`optional`**, un gruppo saltabile di Mod e/o percorsi di override che l'utente può includere o escludere:

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

- `description`: mostrata all'utente nell'anteprima dell'import così può decidere se includere questa feature.
- `platforms`: zero o più entry di hash che vengono scaricate solo se la feature è inclusa.
- `overridePaths`: zero o più percorsi (file o cartelle) dentro la directory `overrides/` che vengono estratti solo se la feature è inclusa. I percorsi sono relativi a `overrides/`.

Una feature può avere solo file di piattaforma, solo override, o entrambi. Usali per impacchettare contenuti opzionali correlati: una Mod Shader più la sua configurazione, un preset di difficoltà hardcore che è solo configurazioni, un Resource Pack opzionale.

#### `source`

Quando un pack è stato esportato da un'istanza Modpack CurseForge o Modrinth esistente, GDLauncher registra da dove viene:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

oppure

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

Questa è informativa; il launcher non la usa per scaricare nulla (l'array `entries` copre già quello). Esiste perché le derivazioni di pack pubblici restino attribuite all'originale.

### Export con bundle o solo manifest

Quando esporti un'istanza in `.gdlpack`, il toggle **Bundle Addons** nel wizard di export decide cosa finisce dove:

- **Bundle off (solo manifest).** Ogni Mod che GDLauncher riesce a risolvere via CurseForge o Modrinth diventa un'entry `platform` in `entries`. Il JAR vero e proprio non viene incluso nell'archivio. Il launcher del destinatario riscarica dalla piattaforma all'import. Le Mod che GDLauncher non riesce a risolvere (es. JAR che hai lasciato cadere a mano) sono incluse in `overrides/mods/` come fallback. Risultato: archivio piccolo, il destinatario ha bisogno di internet al momento dell'install.
- **Bundle on (autonomo).** Nessuna entry `platform` viene emessa; ogni Mod viene copiata in `overrides/mods/`. Risultato: archivio più grande (spesso centinaia di MB o diversi GB), installabile offline una volta che gli asset Minecraft sono in cache.

I Resource Pack e gli Shader si comportano allo stesso modo: quelli risolvibili diventano entry `platform` con bundle off, tutto viene incluso direttamente con bundle on.

## Affiancati: i tre formati

| Proprietà | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Nome del manifest | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Identificazione delle Mod | project + file ID di CurseForge | URL di download + hash | Solo hash del contenuto |
| Risolve da | API CurseForge | URL embedded nel manifest | CurseForge **o** Modrinth, quale risponde |
| Install offline | No (serve CDN) | Sì (gli URL potrebbero comunque richiedere CDN) | Sì quando `Bundle Addons` è on |
| Feature opzionali | Flag `required: false` per Mod | `env` per file (client/server) | Entry `optional` dedicata con più file |
| Cartella overrides | `overrides/` | percorso per file | `overrides/` (configurabile) |
| File solo-server | Pack separato | `env.server` | Directory `serverOverrides` |
| Icona | Esterna | Esterna | Embedded in `.gdl/icon.<ext>` |
| Tracking della sorgente | Nessuno | Nessuno | Campo `source` |
| Portabilità | Più ampia (la maggior parte dei launcher lo legge) | Launcher che supportano Modrinth | GDLauncher |

## Override, in dettaglio

Quando il manifest fa riferimento a una cartella overrides o a file del pack marcati come client-required, il launcher estrae quelli nell'istanza dopo lo step di risoluzione delle Mod. È così che i pack impacchettano:

- Configurazioni di default delle Mod (così il pack si gioca allo stesso modo out of the box per tutti).
- Script KubeJS o CraftTweaker.
- Una mappa iniziale (occasionalmente).
- Resource Pack che il pack si aspetta siano attivi.
- `options.txt` con impostazioni di gioco tarate per il pack.

Gli override vincono sui default auto-generati ma perdono contro qualunque cosa il giocatore cambi successivamente a mano. GDLPack supporta in più le directory `serverOverrides` e `clientOverrides` per file che dovrebbero atterrare solo su un lato, utile per pack pensati per distribuire un bundle client e server abbinato.

## Quando i manifest diventano obsoleti

Un manifest del pack è uno snapshot di quello che l'autore del pack ha pubblicato l'ultima volta. Se una Mod referenziata dal manifest viene rimossa dalla piattaforma sorgente dopo che il manifest è stato costruito, l'install di quella versione del pack fallirà finché l'autore non ne pubblica una nuova. Questo è quello che succede quando una versione più vecchia del pack si installa bene ma una più nuova si rompe: la più nuova fa riferimento a qualcosa che non è più disponibile.

La correzione è lato autore; il launcher può solo fare quello che il manifest gli dice. GDLPack ha un vantaggio parziale qui perché non è legato a una singola piattaforma: se una Mod è rimossa da CurseForge ma è ancora su Modrinth (o viceversa), lo stesso array `entries` si risolve comunque.

## Leggere un manifest da solo

Non ti serve nulla di speciale. Rinomina `mypack.mrpack` o `mypack.gdlpack` in `mypack.zip`, aprilo con qualunque tool di archivio, e guarda il file JSON dentro (`modrinth.index.json` o `gdlpack.json`). Lo stesso con i `.zip` CurseForge e `manifest.json`. Tutti e tre sono JSON normale, leggibile in qualunque editor di testo.

## Costruire un pack

Se stai costruendo un pack da condividere, il percorso più facile è impostare l'istanza in GDLauncher, poi usare **Export Instance** (tasto destro → Export Instance). Il wizard di export ti lascia scegliere il formato target: CurseForge `.zip`, Modrinth `.mrpack`, o il `.gdlpack` proprio di GDLauncher. Scegli `.gdlpack` se anche il destinatario usa GDLauncher e vuoi feature opzionali o icone embedded; scegli `.zip` per la più ampia compatibilità cross-launcher.
