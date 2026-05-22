---
title: "Compatibilità del formato dei salvataggi"
description: "Perché aggiornare una mappa Minecraft a una nuova versione è di solito un viaggio di sola andata, come cambia davvero il formato dei salvataggi, e il modo giusto di fare un backup prima."
faq:
  - question: "Posso aprire una mappa Minecraft 1.21 in 1.20?"
    answer: "Non in modo sicuro. Minecraft migra solo le mappe in avanti, mai indietro. Una mappa aperta in 1.21 ha il suo level.dat e i region file riscritti nel formato più recente; le versioni precedenti o si rifiutano di caricarla o crashano. Se ti servono entrambi, tieni una copia di backup della mappa prima di avviare la versione più nuova."
  - question: "GDLauncher mi avvisa prima di aggiornare una mappa?"
    answer: "Il launcher stesso no, l'avviso è lato Minecraft. Quando apri una mappa salvata in una versione più vecchia, Minecraft mostra una finestra 'This world was saved in a different version' prima di caricarla. Quello è il momento di tornare indietro e copiare la cartella della mappa altrove."
  - question: "Cosa viene riscritto quando una mappa viene aggiornata?"
    answer: "level.dat (i metadati della mappa), i region file in region/ (i dati dei chunk), la cartella playerdata/ (lo stato per giocatore), e qualunque Data Pack per mappa. Il campo Data Version in level.dat viene aggiornato per corrispondere alla nuova versione di Minecraft; quel campo è quello che le versioni più nuove/vecchie leggono per decidere se possono aprire la mappa."
  - question: "Fare downgrade è impossibile?"
    answer: "In senso stretto, sì. Non esiste un percorso ufficiale di downgrade. Alcuni tool della community sostengono di poter tornare indietro sulla Data Version, ma non riscrivono davvero i dati dei chunk, quindi la mappa finisce parzialmente corrotta (biomi, blocchi o entità più recenti che la versione più vecchia non capisce). Tratta gli upgrade come una sola andata."
  - question: "Come faccio il backup di una mappa prima di aggiornarla?"
    answer: "Tasto destro sull'istanza in GDLauncher → Open Folder. Vai in instance/saves e copia la cartella della mappa (quella nominata come la mappa) da qualche parte fuori dalla directory dell'istanza. Tieni quella copia finché non sei sicuro che la versione aggiornata funzioni."
---

# Compatibilità del formato dei salvataggi

## Perché i formati dei salvataggi cambiano

Il formato dei file delle mappe di Minecraft non è fisso. Ogni aggiornamento maggiore di Minecraft rivede come i dati su disco sono strutturati. I nuovi blocchi hanno bisogno di nuovi ID. Le nuove entità hanno bisogno di nuove forme NBT. I nuovi biomi hanno bisogno di nuovi registry di biomi. Dietro le quinte, ogni mappa ha un numero chiamato **Data Version** in `level.dat`, e Minecraft lo usa per decidere cosa fare quando apri una mappa.

Se la Data Version della tua mappa è più vecchia di quella della versione corrente di Minecraft, Minecraft lancia una passata unica del **DataFixer** che riscrive la mappa nel nuovo formato. Chunk, entità, stati dei blocchi, dati del giocatore, tutto viene convertito al nuovo schema. La Data Version in `level.dat` viene aggiornata di conseguenza.

Questa conversione è **distruttiva e a senso unico**. Una volta che i chunk sono riscritti, la versione di Minecraft più vecchia non può più leggerli.

## Cosa significa davvero "a senso unico"

Immagina di avere una mappa 1.20.1. La apri in 1.21. Minecraft mostra il warning "different version", clicchi "Convert" (o la carichi comunque), e il gioco parte. Dietro le quinte:

- `level.dat` viene riscritto così che il suo campo `DataVersion` corrisponda a 1.21 invece di 1.20.1.
- Ogni region file in `region/` che viene caricato (tutto quello entro la tua view distance come minimo) viene riscritto chunk per chunk.
- Nuovi blocchi 1.21 come Crafter o Trial Spawner possono ora esistere nella mappa; non esistono nel registry dei blocchi 1.20.1.
- Le entità e le tile esistenti di 1.20.1 vengono migrate agli schemi 1.21.

Se ora provi ad aprire la stessa cartella in 1.20.1:

- Minecraft confronta `DataVersion` con la propria e si rifiuta di caricare (o crasha caricando certi chunk).
- Anche bypassando il check di versione, i blocchi solo-1.21 resterebbero come missing/error block nel client più vecchio.

Quindi: **aggiornare una mappa a una versione più nuova è permanente**. L'unico rollback sicuro è ripristinare da un backup fatto *prima* dell'upgrade.

## Le mappe moddate peggiorano la cosa

Il DataFixer di Minecraft Vanilla è almeno esaustivo e ben testato. I salvataggi moddati aggiungono un altro livello di rischio:

- Le mod rimosse lasciano errori di **missing block** e **missing entity**. La mappa carica ma i cubi che erano blocchi della mod diventano placeholder "?".
- Le mod sostituite (una versione vecchia → una nuova) a volte cambiano gli ID dei blocchi o le chiavi NBT delle entità. La migrazione è in mano all'autore della mod e non è sempre fluida.
- I salti di versione maggiore di Minecraft dentro un Modpack spesso coincidono con la maggior parte delle mod che si aggiornano a API completamente nuove (Forge 1.20.1 → 1.21.x, per esempio). Le mappe che funzionavano sotto la vecchia versione possono avere comportamento indefinito sotto la nuova.

Per le istanze moddate, tratta qualunque salto di versione come un potenziale evento di corruzione e fai prima un backup.

## Backup di una mappa fatto come si deve

Il backup più semplice è una copia di cartella. In GDLauncher:

1. Tasto destro sull'istanza → **Open Folder**.
2. Apri `instance/saves/`.
3. Copia la cartella nominata come la tua mappa (lo stesso nome che vedi nella lista delle mappe) da qualche parte fuori dall'istanza. Un drive separato, una cartella `~/Documents/mc-backups/`, ovunque non venga sovrascritto.

Quella copia è uno snapshot della mappa al momento della copia. Tienila finché non sei certo che la nuova versione funzioni.

Per backup continui, tool di terze parti come FTBBackups (una mod) fanno snapshot in-game a intervalli. Scrivono in `backups/` dentro l'istanza e sono ripristinabili per snapshot.

## Cosa significano gli avvisi "snapshot version"

Se apri per sbaglio una mappa salvata in una snapshot di Minecraft (una build di sviluppo, tipo `24w11a`), il gioco ufficiale mostra un avviso aggiuntivo perché le Data Version delle snapshot sono a volte avanti rispetto a qualsiasi versione rilasciata. Una mappa da una snapshot può essere impossibile da aprire nella prossima release stabile se il formato della snapshot conteneva cambiamenti che sono stati rimossi prima del rilascio. Il percorso sicuro è: non giocare mappe importanti su snapshot, oppure accettare che la mappa sia bloccata sulla snapshot.

## TL;DR

- Gli upgrade delle mappe sono a senso unico; fai un backup prima di aprire in una versione più nuova.
- Le mappe moddate sono extra fragili; tratta qualunque salto di versione come potenziale evento di corruzione.
- Per gli aggiornamenti di Modpack che bumpano le versioni di Minecraft, copia tutta la cartella saves prima, poi aggiorna.
