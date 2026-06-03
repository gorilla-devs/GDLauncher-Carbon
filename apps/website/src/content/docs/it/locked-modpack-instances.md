---
title: "Istanze Modpack bloccate"
description: "Cosa significa quando un'istanza Modpack è bloccata, perché GDLauncher la blocca, e come sbloccarla o scollegarla quando serve."
---

## Cos'è un'istanza bloccata?

Quando installi un Modpack da CurseForge o Modrinth in GDLauncher, l'istanza è **bloccata** di default. Un'icona di lucchetto appare accanto all'istanza e le azioni che modificherebbero il contenuto del pack, aggiungere, rimuovere o aggiornare singole mod, sono disabilitate. Puoi comunque giocare l'istanza, cambiare le impostazioni di Java o RAM, fare screenshot e tutto il resto; il lock protegge solo il *set di mod gestito dal pack*.

Il lock esiste perché un Modpack è una collezione di mod testata e con versioni fissate. Gli autori dei pack compongono la loro lista di mod deliberatamente e fissano versioni specifiche per la compatibilità. Se sostituisci una mod con una versione più nuova, puoi rompere una mod fratello che dipendeva da quella più vecchia. Il lock intercetta questo errore prima che tu lo faccia.

## Cosa puoi e cosa non puoi fare con il lock attivo

**Puoi** fare tutto quanto segue mentre un'istanza è bloccata:

- Avviare e giocare l'istanza.
- Cambiare RAM, argomenti Java e override Java.
- Fare screenshot e sfogliare i log.
- Cambiare nome e icona dell'istanza (tramite Edit Instance).
- Aggiornare l'intero Modpack a una release più nuova (Settings → Change Modpack Version).

**Non puoi** fare queste cose con il lock attivo:

- Aggiungere niente tramite la tab Addons, e questo include **Mod, Shader, Resource Pack, Data Pack e Mappa**. Il pulsante Add è disabilitato per ogni tipo di addon mentre il lock è attivo.
- Rimuovere o disabilitare una Mod o un addon gestito dal pack.
- Aggiornare singole Mod gestite dal pack a versioni più nuove.

Le tab Mods e Addons mostrano un suggerimento *"This instance is locked, changes can't be applied"* accanto alle azioni disabilitate. Installare dal browser Addons in un'istanza bloccata è bloccato in modo analogo sul pulsante di installazione.

## Tre stati: bloccata, sbloccata, scollegata

Questi tre termini compaiono in GDLauncher e non sono sinonimi.

- **Locked.** L'istanza è accoppiata con un Modpack di CurseForge o Modrinth e il set di mod gestito dal pack è in sola lettura. Questo è il default per i Modpack installati.
- **Unlocked.** Ancora accoppiata col Modpack (il nome e la versione del pack sono ancora tracciati), ma puoi modificare il set di mod liberamente. GDLauncher si ricorda il pack così puoi comunque aggiornare a una release più nuova in seguito, ma ti prendi la responsabilità di mantenere le mod consistenti.
- **Unpaired.** Non più associata col Modpack. L'istanza diventa una custom, stessi file, ma GDLauncher non traccerà più aggiornamenti del pack né la tratterà come istanza Modpack. Andare da unlocked a unpaired è un'azione a senso unico.

## Come sbloccare un'istanza

1. Apri l'istanza e clicca sull'icona dell'ingranaggio (oppure tasto destro sull'istanza → Settings).
2. Scorri fino alla sezione **Modpack Info** in cima alla pagina dei settings. Vedrai l'icona del pack, il nome e la versione corrente, con una riga di pulsanti sotto.
3. Clicca il pulsante **Unlock** (quello con l'icona del lucchetto accanto a "Unlock"). L'istanza passa allo stato sbloccato immediatamente.

Una volta sbloccata, l'intestazione della sezione cambia in un indicatore "Unlocked" con l'icona del lucchetto aperto. Puoi bloccare di nuovo l'istanza passando per lo stesso flusso, ma in pratica, una volta che hai cominciato a gestire il set di mod tu, non c'è molta ragione di riassociare il lock.

## Come scollegare un'istanza

1. Nella stessa sezione Modpack Info, clicca il pulsante **Unpair** (l'icona git-branch).
2. Conferma nel modal che si apre. GDLauncher ti avvisa che l'azione è permanente.

Dopo lo scollegamento, la sezione Modpack Info sparisce completamente. L'istanza è ora una custom e le opzioni **Change Modpack Version** e **Reinstall** non si applicano più.

## Reinstall vs unlock

La sezione Modpack Info ha anche un'azione **Reinstall**. È separata dallo sbloccare e ha uno scopo diverso: reinstalla il Modpack alla sua versione corrente, sovrascrivendo le mod e le configurazioni gestite dal pack con quello che il manifest dice debbano essere. Usala per recuperare un'install rotta (un jar di mod si è corrotto, le configurazioni sono saltate, ecc.) senza perdere le tue mappe.

| Azione | Effetto sulle Mod gestite dal pack | Associazione col pack |
|--------|----------------------------|------------------|
| Unlock | Restano, ma ora modificabili | Mantenuta |
| Unpair | Restano come file, non più "mod del pack" | Rimossa |
| Reinstall | Reset alla versione del manifest | Mantenuta |
| Change Modpack Version | Sostituite dal manifest della nuova versione | Mantenuta (solo a una nuova versione) |

## Quando sbloccare e quando no

Sblocca quando:
- Una Mod specifica gestita dal pack ha un bug critico o un fix di sicurezza e il pack non è stato aggiornato.
- Vuoi aggiungere una Mod personale, uno Shader, un Resource Pack, un Data Pack o una Mappa oltre a quello che il pack include, il pulsante Add della tab Addons è bloccato dal lock, quindi devi sbloccare per installare dalla UI.
- Stai mantenendo da solo un pack non più mantenuto.

Resta col lock attivo quando:
- Il pack è attivamente mantenuto, lascia che l'autore gestisca il pinning delle versioni aspettando la prossima release del pack.
- Stai giocando un'esperienza curata e non vuoi allontanarti dal set di mod previsto.

Un pattern comune è: sblocca brevemente, installa i tuoi extra, poi lascia l'istanza sbloccata. Le cose che hai aggiunto restano anche se riassoci il lock, dato che il lock governa solo il set *gestito dal pack*, ma in pratica c'è poca ragione di riassociare il lock una volta che hai cominciato a gestire l'istanza.

## Cosa il lock non fa

Il lock non è un sistema di permessi né un confine di sicurezza. È un guard rail per evitare modifiche accidentali alle mod nella UI di GDLauncher. La cartella dell'istanza su disco è ancora una cartella normale, qualunque cosa scriva direttamente nella directory `mods` (un tool di terze parti, una copia manuale di file) bypassa il lock completamente.

Se lo fai e poi guardi la tab Mods, GDLauncher mostrerà il file aggiunto manualmente accanto alle mod gestite dal pack. Rimuovere un file del genere richiede di passare per il file system, non dalla UI.

## Troubleshooting rapido

- **"Non riesco ad aggiornare una singola mod."** È il lock che funziona come previsto. Sblocca (Settings → Unlock) o usa Change Modpack Version per aggiornare l'intero pack.
- **"Il pulsante Update All è grigio su un'istanza bloccata."** Stesso motivo. Usa Change Modpack Version, oppure sblocca prima.
- **"Perché la mia mod aggiunta dall'utente resta visibile nella tab Mods dopo aver riassociato il lock?"** Il lock si applica alle mod gestite dal pack, qualunque cosa tu abbia aggiunto sopra resta visibile comunque.
- **"L'opzione Reinstall ha sovrascritto una configurazione che avevo modificato."** È previsto. Reinstall resetta al manifest del pack. Fai il backup delle configurazioni modificate prima di reinstallare.
