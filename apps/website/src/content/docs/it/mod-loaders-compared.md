---
title: "Mod loader a confronto: Forge, NeoForge, Fabric, Quilt"
description: "GDLauncher supporta quattro mod loader Minecraft. Cosa sono, come si differenziano e quale scegliere per una data Mod o un dato Modpack."
faq:
  - question: "Quale mod loader dovrei usare per Minecraft?"
    answer: "Usa quello richiesto dalla Mod o dal Modpack che vuoi, è quasi sempre così che si decide. Se stai scegliendo da zero senza vincoli: Fabric per Mod di performance/QoL sulle versioni più recenti, NeoForge per le nuove Mod di contenuto pesante, Forge per i Modpack più vecchi e la libreria storica più grande."
  - question: "Le Mod Forge sono compatibili con Fabric?"
    answer: "No. Le Mod Forge e le Mod Fabric non sono intercambiabili. Una Mod scritta per uno non si carica sull'altro. Molte Mod popolari hanno build Forge e Fabric separate; controlla la pagina della Mod per vedere quali versioni di quale loader sono supportate."
  - question: "NeoForge è un sostituto di Forge?"
    answer: "Di fatto sì per le versioni di Minecraft più nuove. NeoForge è nato come un fork di Forge del 2023 con la stessa API; le due si sono divergerse da allora, quindi una Mod attuale di solito esce con una build NeoForge separata invece che girare su entrambe. Molte Mod Forge dal 1.20.4 in poi sono ora costruite per NeoForge invece. Per Minecraft 1.20.1 e precedenti, Forge è ancora lo standard."
  - question: "Le Mod Fabric sono compatibili con Quilt?"
    answer: "In gran parte sì. Quilt è un fork di Fabric e fa girare le Mod Fabric direttamente. Alcune Mod solo-Quilt usano API Quilt che non girano su Fabric. Se hai una lista di Mod che vuoi e sono tutte Fabric, puoi usare entrambi i loader e funzionerà."
  - question: "Posso far girare due mod loader fianco a fianco?"
    answer: "Non nella stessa istanza. Ogni istanza sceglie esattamente un mod loader. Per usarli entrambi, crea due istanze separate. Il sistema di istanze di GDLauncher è progettato esattamente per questo: un'istanza Forge, un'istanza Fabric, passa dall'una all'altra con un click."
---

# Mod loader a confronto: Forge, NeoForge, Fabric, Quilt

## I quattro mod loader supportati da GDLauncher

GDLauncher può installare e far girare uno qualsiasi dei quattro mod loader principali per Minecraft Java Edition, più Vanilla (nessun loader). Ne scegli uno quando crei un'istanza custom; per le install di Modpack il loader è quello che dice il manifest del pack.

### Forge

Il mod loader originale, nato nel 2011. Forge ha la libreria di Mod storica più grande, specialmente per Mod ricche di contenuti che aggiungono tech tree, sistemi magici o nuovi mondi (Tinkers' Construct, Twilight Forest, Create nelle sue versioni precedenti). È anche il loader a cui puntano i Modpack più vecchi.

Forge si aggiorna più lentamente di Fabric. Le nuove versioni di Minecraft spesso vedono una release Forge settimane o mesi dopo l'uscita.

### NeoForge

Un fork di Forge del 2023, creato dopo una divisione di governance. NeoForge mantiene lo stile dell'API Forge (le Mod di solito sono compatibili a livello sorgente) ma esce più velocemente ed è dove molto sviluppo di Mod Forge è migrato.

Su Minecraft 1.20.4 e successivi, NeoForge è il più attivo dei due. Molte Mod grandi ora rilasciano build NeoForge alla pari con Forge o al posto di Forge.

### Fabric

Una filosofia di design diversa: piccolo, veloce, modulare. Fabric esce quasi il giorno stesso in cui una nuova versione di Minecraft viene rilasciata, a volte entro ore. Il suo ecosistema di Mod pende verso la performance (Sodium, Lithium, FerriteCore), la QoL (Mod Menu, Iris), e Mod di contenuto nuove di alta qualità.

Fabric è il loader che vuoi se la performance è la priorità o se stai giocando su una versione di Minecraft di taglio recente.

### Quilt

Un fork di Fabric del 2022 con un diverso modello di governance e qualche API extra. Quilt fa girare le Mod Fabric direttamente, quindi la differenza pratica è piccola: scegli Quilt se una Mod specifica lo richiede, altrimenti Fabric funziona allo stesso modo.

Quilt ha un ecosistema dedicato più piccolo di Fabric ma è pienamente compatibile con la maggior parte dei contenuti Fabric.

## Matrice di compatibilità

| Mod costruita per | Gira su Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Sì | A volte (le prime versioni di NeoForge potevano far girare Mod Forge non modificate perché è nato come fork fresco; le API si sono divergerse da allora, quindi la maggior parte delle Mod Forge attuali necessita di una build NeoForge) | No | No |
| NeoForge | No | Sì | No | No |
| Fabric | No | No | Sì | Sì |
| Quilt | No | No | Mod con API Quilt: no; il resto: sì | Sì |

Non c'è alcun ponte cross-loader in produzione. Un mod loader è essenzialmente un runtime diverso, i JAR che metti in `mods/` devono corrispondere al loader che l'istanza sta usando.

## Scegliere per una nuova istanza

Di solito non scegli tu, le Mod o il Modpack che vuoi scelgono per te:

- **Stai installando un Modpack da CurseForge o Modrinth?** GDLauncher legge il manifest del pack e installa il loader che il pack specifica. Non hai voce in capitolo.
- **Stai costruendo un'istanza custom attorno a una Mod specifica?** Guarda la pagina della Mod. Se dice "Fabric 1.21.x", crea un'istanza Fabric 1.21.x.
- **Stai costruendo un'istanza custom attorno a una lista di Mod?** Trova il loader supportato dalla maggior parte. Cerca ogni Mod, elenca per quali loader ha build, scegli l'intersezione. La maggior parte delle Mod di performance è solo Fabric; la maggior parte delle Mod di contenuto grandi è Forge/NeoForge.

Se davvero non hai vincoli e vuoi una raccomandazione Vanilla: **Fabric** per setup focalizzati sulla performance o visivamente curati, **NeoForge** per survival moddato con contenuti pesanti.

## Cambiare loader su un'istanza esistente

GDLauncher ti permette di cambiare il mod loader di un'istanza dopo la creazione, vedi [Come cambiare mod loader su un'istanza esistente](/guides/switch-mod-loader). In breve: tasto destro sull'istanza → Edit → scegli un loader diverso. La cartella mods non viene svuotata, quindi i JAR del vecchio loader restano in giro; rimuovi quelli incompatibili manualmente prima di avviare.

## Una nota sulle versioni del loader

Ogni loader ha il suo stream di versioni, indipendente da Minecraft. Quando scegli "Forge" scegli anche una versione di Forge (qualcosa tipo `47.2.0` per Minecraft 1.20.1). Per le Mod, la versione del loader di solito non conta oltre "stessa major che il pack si aspetta", ma alcune Mod richiedono una build minima del loader. La pagina CurseForge o Modrinth della Mod lo dirà.
