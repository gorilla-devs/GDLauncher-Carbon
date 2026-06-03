---
title: "Risoluzione dei problemi"
description: "Risolvi i problemi comuni di lancio di GDLauncher e Minecraft. Path dei dati dell'app, runtime path, posizioni dei log e soluzioni testate."
faq:
  - question: "Dove salva i suoi dati GDLauncher?"
    answer: "Su Windows: C:\\Users\\<tu>\\AppData\\Roaming\\gdlauncher_carbon. Su macOS: /Users/<tu>/Library/Application Support/gdlauncher_carbon. Su Linux: $XDG_DATA_HOME/gdlauncher_carbon (o ~/.local/share/gdlauncher_carbon se XDG non è impostato)."
  - question: "Dove sono i log di GDLauncher?"
    answer: "GDLauncher scrive due log a livello app in file diversi. main.log (Electron) vive nella cartella dei dati dell'app, e i file <timestamp>.log con timestamp vivono nella cartella __gdl_logs__ del runtime path (core Rust; vengono tenuti i 10 più recenti). Quando segnali problemi, invia entrambi, vedi la guida Share App Logs per le posizioni esatte."
  - question: "GDLauncher non si apre. Cosa faccio?"
    answer: "Prima di tutto, controlla i log nella cartella dei dati per cercare un errore. Cause comuni: runtime corrotto, antivirus che blocca l'eseguibile, o un aggiornamento applicato parzialmente. Reinstallare GDLauncher pulito e ripristinare le istanze di solito risolve entrambi."
  - question: "Perché il mio Modpack crasha al launch?"
    answer: "La maggior parte dei crash al launch viene da un mismatch tra versione di Minecraft / mod loader / Mod. Controlla il file latest.log per l'errore. Se viene nominata una singola Mod, di solito è la colpevole, disabilitala nella tab Addons e rilancia. Se è un OutOfMemoryError, aumenta la RAM nelle impostazioni dell'istanza."
  - question: "Come sposto GDLauncher su un drive o cartella diversa?"
    answer: "Apri Settings → General → Runtime Path. Cambialo nella nuova posizione e GDLauncher migrerà automaticamente le tue istanze e i download. La migrazione gira una volta al prossimo launch."
  - question: "Posso usare GDLauncher offline?"
    answer: "Puoi giocare offline le istanze che hai già installato. L'autenticazione richiede comunque di andare online almeno una volta inizialmente (account Microsoft), e scaricare nuove Mod o Modpack richiede una connessione internet."
---

## Path dei dati dell'app

È il path dove GDLauncher salva i dati di Electron, oltre al runtime path del Core Module per default.

### Windows

`C:\Users\\{{Il tuo nome utente}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Il tuo nome utente}}/Library/Application Support/gdlauncher_carbon`

### Linux

- se la variabile d'ambiente `$XDG_DATA_HOME` è disponibile: `$XDG_DATA_HOME/gdlauncher_carbon`
- altrimenti: `{{homedir}}/.local/share/gdlauncher_carbon`

[Maggiori dettagli su homedir](https://nodejs.org/api/os.html#oshomedir)

## Runtime path del Core Module

È il path dove il Core Module salva tutti i suoi dati, incluse tutte le istanze, gli asset e le librerie.
Si trova di solito nello stesso path dell'app data path, dentro la cartella `data`, a meno che tu non l'abbia esplicitamente impostato in un'altra posizione.

### Database dell'app

Il database dell'app si trova nel runtime path del Core Module, ed è un file di database SQLite chiamato `gdl_conf.db`.

**NON INVIARE QUESTO FILE A NESSUNO, CONTIENE DATI SENSIBILI.**

### Log dell'app

GDLauncher scrive due log a livello app in file diversi. Per il supporto, **invia sempre entrambi**, le due metà del launcher si passano lavoro a vicenda e la causa di un fallimento da una parte si manifesta spesso nel log dell'altra.

- **`main.log`** nell'App Data Path: il log del main process di Electron. Copre la creazione delle finestre, IPC, auto-update, finestre native, e crash duri della shell desktop.
- **`__gdl_logs__/<timestamp>.log`** nel Core Module Runtime Path: il log del core Rust. Copre l'accesso agli account, i download degli asset, gli install dei mod loader, i lanci delle istanze, i cambi di impostazioni. Il launcher tiene i 10 più recenti e rimuove i più vecchi automaticamente; il più recente è quello che ti serve.

Vedi [Share App Logs](/guides/share-app-logs) per screenshot e path per OS.

**I LOG POSSONO CONTENERE DATI SENSIBILI, ATTENZIONE QUANDO LI CONDIVIDI.**

### Cambiare runtime path

Se cambi il runtime path, l'app sposterà automaticamente tutte le tue istanze e i file di configurazione nella nuova posizione.

Se la cartella di destinazione è già in uso, l'app si limiterà a cambiare la configurazione del runtime path e nessun file verrà spostato o copiato.

#### Errore di migrazione

Se la migrazione fallisce, l'app mostrerà un messaggio di errore.

La prima cosa da fare è cercare di capire cosa significa il messaggio di errore.
Se tutti i file sono stati copiati correttamente, probabilmente l'errore è capitato mentre si cercava di cancellare i vecchi file. Puoi chiudere l'app e cancellare manualmente i vecchi file.

Assicurati di NON CANCELLARE il file chiamato `runtime_path_override` nel vecchio runtime path, perché viene usato dall'app per rilevare se il runtime path è stato cambiato.

In caso di dubbi, entra sul nostro [server Discord](https://discord.gdlauncher.com) e chiedi aiuto.
