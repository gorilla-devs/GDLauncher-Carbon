---
title: "Runtime path vs App data path"
description: "GDLauncher usa due percorsi diversi per salvare i dati. Cosa contiene ognuno, perché sono separati, e quale di solito vuoi spostare."
faq:
  - question: "Qual è la differenza tra il path dei dati dell'app e il runtime path?"
    answer: "Il path dei dati dell'app è la cartella per-utente standard che Electron usa per le cache e il marker runtime_path_override. Il runtime path è dove il core di GDLauncher salva tutto il pesante: istanze, asset e librerie di Minecraft, install di Java, il database del launcher, e i log a livello app. Di default il runtime path sta dentro il path dei dati dell'app, ma è quello pensato per essere spostato."
  - question: "Dov'è il path dei dati dell'app sul mio OS?"
    answer: "Windows: C:\\Users\\<tu>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<tu>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, o ~/.local/share/gdlauncher_carbon se XDG non è impostato."
  - question: "Dovrei spostare il runtime path o il path dei dati dell'app?"
    answer: "Il runtime path. È quello che cresce con ogni istanza che installi. Il path dei dati dell'app resta piccolo ed è legato al tuo profilo utente dell'OS. GDLauncher espone lo spostamento del runtime path in Settings → Runtime Path; il path dei dati dell'app è gestito da Electron e non si può spostare dalla UI."
  - question: "A cosa serve il file runtime_path_override?"
    answer: "Dopo che cambi il runtime path, GDLauncher scrive un piccolo file di testo chiamato runtime_path_override dentro il path dei dati dell'app. Contiene il nuovo runtime path. Al prossimo launch, GDLauncher lo legge per sapere dove vivono i tuoi dati. Se manca, il launcher torna al runtime path di default."
  - question: "Posso condividere il runtime path tra due computer?"
    answer: "No. Il database del launcher traccia stato specifico della macchina (percorsi, token degli account, install di Java) e non è pensato per uso simultaneo da due install. Se vuoi le stesse istanze su un secondo computer, copia le istanze individualmente o usa la feature di share delle istanze."
---

# Runtime path vs App data path

## Due path, due scopi

GDLauncher suddivide i suoi file in due posizioni: un **app data path** per le piccole cose lato Electron, e un **runtime path** per la roba grossa (istanze, asset, Java, il database). Il runtime path è quello che occasionalmente vorrai spostare, l'app data path di solito non lo tocchi mai.

### App data path

È la cartella per-utente standard dell'app, la posizione a cui punta il `userData` di Electron. GDLauncher la usa per:

- Il marker `runtime_path_override`, un file di testo di una riga che dice al launcher dove vive davvero il runtime path
- Il runtime path di default, in una sottocartella `data/`, se non l'hai spostato
- Le cache Chromium di Electron stesso (Network/, GPUCache/, Cookies, ecc.)
- I log del main process di Electron

Vive nella posizione standard dell'OS:

- **Windows:** `C:\Users\<tu>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<tu>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, o `~/.local/share/gdlauncher_carbon` se `$XDG_DATA_HOME` non è impostato

Senza la sottocartella `data/` di solito è piccolo. GDLauncher non espone un'impostazione per spostare questa directory, le convenzioni dell'OS e Electron si aspettano che stia lì.

### Runtime path (Core Module)

È dove il core Rust di GDLauncher mette tutto il resto:

- Le tue istanze (sotto `instances/`)
- Gli asset condivisi di Minecraft (le texture e i suoni che Mojang distribuisce)
- Le librerie condivise di Minecraft (i file JAR che Mojang e i mod loader forniscono)
- Le install di Java scaricate da GDLauncher
- Il database del launcher, `gdl_conf.db`
- I log del launcher a livello app, in `__gdl_logs__/`

Di default il runtime path è `<app data path>/data/`, quindi sta dentro la cartella dei dati dell'app. Puoi puntarlo ovunque con **Settings → Runtime Path**. È il path che diventa grande, Modpack grandi e una manciata di istanze moddate lo spingono facilmente oltre i 50 GB.

## Quando spostare il runtime path

Le due ragioni comuni:

1. **Il tuo SSD si sta riempiendo.** Sposta le istanze su un HDD più grande o un SSD secondario.
2. **Vuoi i backup separati dal tuo profilo utente dell'OS.** Mettere il runtime path su un drive che backuppi indipendentemente va bene; solo non sincronizzarlo attivamente mentre giochi, il launcher e il tool di sync si combatteranno per i file handle.

Non hai bisogno di spostare il runtime path per uso normale. La posizione di default è corretta per la maggior parte dei setup.

## Come funziona lo spostamento

Apri **Settings → Runtime Path**. Digita la nuova posizione o scegliela con l'icona della cartella. Il pulsante di applicazione (l'icona della freccia circolare a destra della riga) si illumina quando il path differisce da quello corrente ed è valido. Cliccarlo apre un modal di conferma che mostra i path vecchio e nuovo.

Se la cartella target è vuota (o non esiste ancora), confermare lancia una migrazione completa: un overlay mostra la scansione, poi la copia file per file, poi la rimozione file per file dalla sorgente. Non chiudere l'app né spegnere la macchina mentre questo è in corso. Quando finisce il launcher si riavvia.

Se il target contiene già un runtime path GDLauncher (l'hai spostato prima e vuoi che un'install fresca lo raccolga), il modal ti avvisa in giallo che la cartella non è vuota. Confermare lì fa uno "switch only": il marker viene riscritto per puntare ai dati esistenti, nessun file viene copiato, e il launcher si riavvia. I dati lasciati nella vecchia posizione diventano orfani e puoi cancellarli a mano.

Se una migrazione fallisce a metà, l'overlay diventa rosso e mostra l'errore. Il launcher fa rollback: i file che ha creato nel nuovo path vengono rimossi e il marker resta a puntare al vecchio path, così puoi riprovare senza perdere dati. Le due cause comuni sono permessi di scrittura mancanti sul drive di destinazione e spazio libero esaurito.

### Il marker runtime_path_override

Quando cambi il runtime path, GDLauncher scrive un piccolo file di testo chiamato `runtime_path_override` dentro l'**app data path** (non dentro il runtime path). Il file contiene il nuovo runtime path come testo. A ogni launch l'app lo legge per sapere dove sono i tuoi dati.

Se cancelli il marker, GDLauncher torna al suo runtime path di default (`<app data>/data/`). I tuoi dati non sono persi, sono ancora dove li hai spostati, ma il launcher non li vedrà finché non vai in **Settings → Runtime Path** e lo punti di nuovo a quella cartella. Dato che la cartella contiene già dati di GDLauncher, il launcher la tratta come operazione "switch only" e aggiorna solo il marker senza copiare nulla.

## Cosa contiene il database, e perché non dovresti condividerlo

Il file `gdl_conf.db` nel runtime path contiene i token degli account, i refresh token Microsoft, lo stato dell'account GDL, e i metadati per istanza. È specifico della macchina e contiene credenziali sensibili. **Non condividerlo con nessuno**, e non provare a usare lo stesso database su due computer, la seconda macchina combatterà la prima per il refresh dei token ed entrambe finiranno scollegate.

Se vuoi le stesse istanze su un secondo computer, copia le istanze fuori da `instances/` manualmente, o usa la feature di share delle istanze, che è pensata per quello.
