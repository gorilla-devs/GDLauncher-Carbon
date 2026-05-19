---
title: "Migrazione dal vecchio GDLauncher"
---

# Migrazione dal vecchio GDLauncher

Se stai ancora usando GDLauncher Legacy, migrare a GDLauncher Carbon è semplice. Ecco come puoi farlo.

## Scarica GDLauncher Carbon

Puoi scaricare GDLauncher Carbon dal nostro [sito ufficiale](https://gdlauncher.com).

## Installa / Sostituisci GDLauncher Carbon

Una volta scaricata la nuova versione, puoi installarla seguendo le istruzioni di installazione per il tuo sistema operativo [qui](/docs/installation).
Installare GDLauncher Carbon sostituirà la vecchia versione dato che condividono lo stesso identico app id. Installando GDLauncher Carbon non perderai nessuna delle tue vecchie istanze, perché sono salvate in una cartella diversa.

## Importa automaticamente le tue vecchie istanze

Dopo aver installato GDLauncher Carbon ed esserti loggato, il flusso di onboarding ti chiede se vuoi importare le tue istanze esistenti dal vecchio GDLauncher. Se hai saltato l'onboarding, puoi lanciare lo stesso flusso in qualsiasi momento: apri la pagina Library, clicca il **+** nella toolbar della Library → tab **Import** → scegli **GDLauncher (legacy)** come sorgente. GDLauncher Carbon legge direttamente la lista di istanze del vecchio launcher e importa ogni istanza nella sua cartella.

## Importa manualmente le tue vecchie istanze

Se l'importer non trova il vecchio launcher (per esempio se è installato in una posizione non standard), indicagli la cartella manualmente dalla stessa tab Import.

Se qualcosa fallisce comunque, segnalalo sul nostro [Discord](https://discord.gdlauncher.com). Come ultima risorsa puoi copiare i file a mano:

- Crea una nuova istanza in GDLauncher Carbon con la stessa configurazione (stessa versione di Minecraft e mod loader; se è un Modpack, installa prima la stessa versione esatta del pack).
- Apri il data path del vecchio GDLauncher, trova la cartella dell'istanza sorgente.
- Apri la nuova istanza Carbon: tasto destro → **More Options** → **Open Folder**. Copia i contenuti dell'istanza vecchia nella sottocartella `instance/` di Carbon. Vedi la pagina [troubleshooting](/docs/troubleshooting) per le posizioni esatte dei data path.

## Cancellare le vecchie istanze (opzionale)

Quando hai finito di importare, puoi opzionalmente cancellare le tue vecchie istanze. Non verrà fatto automaticamente.
