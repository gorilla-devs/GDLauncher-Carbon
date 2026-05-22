---
title: "Anatomia della cartella di un'istanza"
description: "Cosa c'è dentro la cartella di un'istanza GDLauncher, dove vivono mod, mappe, configurazioni, screenshot e log, e cosa puoi cancellare a mano senza problemi."
faq:
  - question: "Dove vengono salvate le mappe di Minecraft in GDLauncher?"
    answer: "Ogni mappa vive in <runtime_path>/instances/<instance>/instance/saves/<world-name>/. La cartella saves/ viene creata la prima volta che generi o importi una mappa. Apri la cartella dell'istanza con tasto destro → Open Folder, poi entra in instance/saves per trovarla. Copia la cartella della mappa per fare un backup."
  - question: "Dove sono i miei crash report?"
    answer: "Dentro l'istanza: instance/crash-reports/. La cartella esiste solo se Minecraft è effettivamente crashato; se non hai mai visto il gioco morire male, ancora non c'è. Ogni crash report è un file di testo con timestamp (crash-<date>-server.txt o simili)."
  - question: "Dove vanno i JAR delle mod?"
    answer: "In instance/mods/ dentro la cartella dell'istanza. Aggiungere un JAR a mano funziona ma bypassa il tracciamento di GDLauncher, meglio usare la tab Addons a meno che tu non abbia un motivo. Le istanze Modpack bloccate disabilitano il pulsante Add della tab Addons, ma una copia manuale del file passa lo stesso."
  - question: "Qual è la differenza tra i log del launcher e i log di Minecraft?"
    answer: "Sono due set diversi. I log del launcher sono i log di sessione di GDLauncher stesso e vivono in <instance>/logs/. I log di gioco di Minecraft (latest.log, crash-reports, qualunque cosa il gioco scriva) vivono un livello più in profondità in <instance>/instance/logs/ e <instance>/instance/crash-reports/."
  - question: "Cosa sono instance.json e packinfo.json?"
    answer: "File di metadati che GDLauncher scrive al livello superiore di ogni istanza. instance.json contiene il nome dell'istanza, l'icona, il modloader, la versione di Minecraft, l'ultima volta che hai giocato e il tempo di gioco. packinfo.json (presente solo per istanze accoppiate a un Modpack di CurseForge o Modrinth) traccia quali file appartengono al pack così il launcher può distinguere le mod gestite dal pack da quelle aggiunte dall'utente. Non cancellare nessuno dei due a mano."
---

# Anatomia della cartella di un'istanza

## Dove si trova la cartella dell'istanza

Ogni istanza GDLauncher sta in una sottocartella del runtime path:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← la cartella dell'istanza
```

`<shortpath>` è una versione sanificata del display name dell'istanza. Fai tasto destro sull'istanza in GDLauncher e scegli **Open Folder** per andarci.

## Cosa c'è dentro

La cartella dell'istanza si divide in alcune cose che GDLauncher traccia al livello superiore, più una sottocartella `instance/` che è la game directory effettiva di Minecraft. Alcune cartelle sono sempre presenti; molte vengono create on demand la prima volta che qualcosa ci scrive sopra.

```
<shortpath>/
├── instance.json          ← metadati di GDLauncher per questa istanza (sempre presente)
├── packinfo.json          ← info di accoppiamento col Modpack (solo per Modpack accoppiati)
├── icon.png | icon.webp   ← icona personalizzata (solo se ne hai impostata una)
├── logs/                  ← log del launcher per questa istanza
└── instance/              ← game directory di Minecraft
    ├── mods/              ← JAR delle Mod
    ├── config/            ← configurazioni delle Mod
    ├── shaderpacks/       ← Shader (se ne hai installati)
    ├── options.txt        ← impostazioni del client Minecraft (dopo il primo launch)
    ├── logs/              ← log di sessione di Minecraft (latest.log ecc.)
    ├── saves/             ← mappe (creata quando ne generi una)
    ├── screenshots/       ← screenshot da F2 (creata al primo F2)
    ├── crash-reports/     ← dump dei crash (solo se il gioco ha crashato)
    ├── resourcepacks/     ← Resource Pack personalizzati (quando ne aggiungi uno)
    ├── datapacks/         ← Data Pack globali (i Data Pack per mappa vivono in saves/<world>/datapacks)
    └── (specifici del pack) ← kubejs/, defaultconfigs/, packmenu/, ecc. solo se il Modpack li include
```

Non sorprenderti se alcune di queste mancano in un'istanza fresca. Il launcher e Minecraft creano solo ciò che serve, quando serve. Un'istanza che non hai mai giocato non ha `saves/`, niente `screenshots/`, niente `options.txt`. Un'istanza Vanilla non ha `mods/` né `config/`.

## Cosa contiene ognuno

### File al livello superiore

- **`instance.json`**: metadati di GDLauncher, nome, percorso icona, modloader e versione, versione di Minecraft, quando l'hai creata, quando l'hai giocata l'ultima volta, tempo di gioco totale. Sempre presente.
- **`packinfo.json`**: un manifest di hash di quali file vengono dal Modpack. Permette al launcher di distinguere le mod gestite dal pack da qualunque cosa tu abbia aggiunto. Presente solo per istanze accoppiate a un Modpack di CurseForge o Modrinth.
- **`icon.png`** o **`icon.webp`**: l'icona personalizzata che hai caricato. Manca se stai usando quella di default.

### `logs/` (al livello superiore)

I log per-istanza di GDLauncher stesso. Sono quelli che vedi cliccando **View Logs** dal menu tasto destro. Coprono la vista del *launcher* sul launch (argomenti Java, download degli asset, install del mod loader, exit code) e sono utili quando il gioco non arriva neanche a scrivere il suo log.

### `instance/mods/`

I file JAR delle Mod. Minecraft carica tutto quello che c'è qui dentro all'avvio, secondo le regole del mod loader. Il launcher tiene traccia delle mod che appartengono a un'istanza nel suo database (tramite filename e hash del contenuto), non ci sono file sidecar. I JAR che lasci cadere qui a mano vengono raccolti lo stesso; semplicemente il launcher non avrà metadati CurseForge/Modrinth per loro.

### `instance/config/`

Una sottocartella o file per ogni mod. È dove le impostazioni delle mod vengono salvate. Molte mod scrivono `config/<modid>.toml` o una directory `config/<modid>/`. Modificarle a mano di solito è sicuro; molte mod leggono i cambiamenti al riavvio del gioco.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Pacchetti di asset. I Resource Pack cambiano texture e suoni, gli Shader cambiano il rendering (richiedono Iris/OptiFine/ecc. installato come mod), i Data Pack aggiungono ricette/loot/funzioni. I Data Pack destinati a una mappa specifica vivono invece in `saves/<world>/datapacks/`. Queste cartelle vengono create solo quando hai effettivamente contenuti da metterci.

### `instance/saves/`

Ogni mappa è una sottocartella. Dentro, `level.dat` è il file master della mappa, `region/` ha i dati dei chunk, `playerdata/` ha lo stato per giocatore, `datapacks/` contiene i Data Pack a livello mappa. Per fare il backup di una mappa, copia tutta la cartella `<world>/`. La cartella `saves/` stessa appare la prima volta che generi una mappa.

### `instance/screenshots/`

Tutto quello che hai catturato con F2. PNG, nominati per timestamp. Creata la prima volta che fai uno screenshot.

### `instance/logs/` e `instance/crash-reports/`

L'output diagnostico di Minecraft stesso. `logs/latest.log` è sempre il launch più recente (ruotato in `logs/<date>-1.log.gz` al launch successivo). `crash-reports/` contiene i dump completi dei crash quando il gioco muore male, e appare solo dopo che c'è stato un crash.

### `instance/options.txt`

Le impostazioni del client Minecraft (grafica, controlli, suoni). Plain text, key=value. Modificabile se vuoi davvero.

### Cartelle specifiche dei Modpack

Molti Modpack grandi includono cartelle extra. Le più comuni:

- **`kubejs/`**: script KubeJS (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Gli autori dei pack li usano per tweak a runtime.
- **`defaultconfigs/`**: uno snapshot di "come dovrebbero essere le configurazioni di default". Lo script di avvio del pack copia le voci mancanti in `config/` a ogni avvio.
- **`packmenu/`**: asset per il menu principale a tema del pack (pulsanti personalizzati, sfondi, splash text).
- **`defaultsettings/`**: simile a `defaultconfigs/`, ma per `options.txt` e i keybinding.

Queste esistono solo se il pack le include. Vanilla e la maggior parte delle istanze custom non ne hanno.

## Cosa puoi cancellare in sicurezza

| Cartella | Si può cancellare? | Effetto |
|---|---|---|
| `instance/mods/` (un JAR specifico) | Sì | Quella mod sparisce. Può rompere mappe che la usavano. |
| `instance/config/<modid>/` | Sì | La mod torna ai default al prossimo launch. |
| Contenuto di `instance/resourcepacks/`, `instance/shaderpacks/` | Sì | Il pack sparisce. |
| `instance/saves/<world>/` | Sì | La mappa sparisce per sempre. Fai prima un backup. |
| `instance/logs/`, `crash-reports/` | Sì | Liberi solo spazio su disco. |
| `instance/screenshots/` | Sì | Addio vecchi screenshot. |
| `logs/` (log del launcher) | Sì | Idem. |
| `instance/options.txt` | Sì | Reset delle impostazioni di gioco ai default. |
| `instance.json` | No | Il launcher perde traccia dell'istanza. |
| `packinfo.json` | Possibile (di fatto scollega) | Il launcher smette di trattare l'istanza come Modpack accoppiato. Stesso effetto del pulsante Unpair nella tab Settings dell'istanza, ma più sporco. |
| Tutta la cartella `instance/` | No | L'istanza diventa malformata. Usa Delete dalla UI del launcher invece. |

## Una nota sulle istanze Modpack bloccate

La cartella dell'istanza è solo una cartella su disco; il lock che GDLauncher applica alle istanze Modpack viene fatto rispettare nella UI, non sul filesystem. Lasciare un JAR in `instance/mods/` di un'istanza bloccata funziona meccanicamente, il launcher semplicemente non se ne accorgerà tramite la tab Addons. Rimuovere un file del genere richiede di tornare sul filesystem. Per workflow più sicuri, apri l'istanza, vai sulla tab **Settings**, e clicca **Unlock** nella sezione Modpack prima.
