---
title: "Modalità offline"
description: "Cosa puoi e non puoi fare in GDLauncher senza connessione internet. Cosa è in cache, cosa deve fare 'telefonata a casa', e come si comporta davvero la scadenza dei token."
faq:
  - question: "Posso giocare a Minecraft offline tramite GDLauncher?"
    answer: "Sì. Il singleplayer funziona completamente offline. Se il tuo token in cache è ancora valido, clicchi Play e Minecraft parte normalmente. Se il token in cache è scaduto, il launcher mostra un prompt Account Expired con un pulsante 'Launch anyway', sceglilo e potrai comunque giocare in singleplayer. L'autenticazione online è richiesta solo per i server multiplayer che verificano l'identità tramite Mojang."
  - question: "Quanto a lungo posso restare offline prima che i token scadano?"
    answer: "Dipende da cosa vuoi fare. Per il singleplayer, non c'è un vero limite di tempo, il launcher offrirà 'Launch anyway' una volta che il token è scaduto. Per i server multiplayer che verificano l'identità tramite Mojang, ti serve un token fresco, il che significa tornare online per rinnovarlo. Il launcher rinnova proattivamente il token di auth Minecraft circa 12 ore prima della sua scadenza a 24 ore, quindi finché sei stato online di recente, il multiplayer continua a funzionare."
  - question: "Posso installare nuove Mod o Modpack offline?"
    answer: "No. I download delle Mod vengono dai CDN di CurseForge e Modrinth, entrambi richiedono internet. Lo stesso per i download di Java, gli asset di Minecraft e i manifest dei Modpack. Qualunque cosa legata all'install ha bisogno di una connessione."
  - question: "Posso aggiornare un'istanza esistente offline?"
    answer: "No. Stessa ragione: gli aggiornamenti tirano nuovi file dai CDN. Il launcher metterà in coda l'aggiornamento e riproverà quando vedrà una connessione."
  - question: "E l'account GDL, funziona offline?"
    answer: "Parzialmente. Il launcher ricorda che sei connesso a GDL, ma qualunque cosa richieda di parlare al servizio GDL (condivisione cloud di istanze, modifiche al profilo, vedere i tuoi share) ha bisogno di internet. L'account Microsoft è quello che governa il launch; GDL è per le funzionalità oltre il launch."
---

# Modalità offline

## Cosa significa davvero "offline" qui

Il comportamento offline di GDLauncher dipende da tre diversi bisogni di rete:

1. **Auth Microsoft** (provare a Mojang che possiedi Minecraft).
2. **Download di Mod e asset** (CurseForge, Modrinth, CDN delle librerie di Mojang).
3. **Funzionalità dell'account GDL** (condivisione cloud di istanze, profilo, cronologia del display name, ecc.).

Ognuna fallisce diversamente quando internet è giù, e il comportamento del launcher è di conseguenza diverso in ogni caso.

## Avviare un'istanza installata offline

Questo è lo scenario più comune: sei in aereo, in una baita, o l'internet di casa è giù, e vuoi giocare a qualcosa che hai già installato.

**Di solito funziona**, perché GDLauncher mette in cache i dati necessari al launch:

- I token di auth Mojang sono salvati localmente con i loro timestamp di scadenza.
- Le librerie e gli asset di Minecraft sono già su disco (nel runtime path).
- Le istanze moddate hanno le loro Mod installate localmente.

Quando clicchi Play offline, il launcher:

1. Controlla se il token di auth Minecraft dell'account Microsoft attivo è ancora valido (non scaduto).
2. Se sì, avvia Minecraft direttamente con quel token. Minecraft stesso non ha bisogno di internet per avviare una mappa singleplayer.
3. Se l'access token è scaduto ma il refresh token è ancora valido, il launcher prova a chiamare l'endpoint di refresh di Microsoft, che ha bisogno di internet. Offline, questa chiamata fallisce, e lo stato dell'account passa a "expired" in Settings → Accounts.
4. Se l'account è scaduto e clicchi comunque Play, il launcher apre un modal Account Expired con due pulsanti: **Launch anyway** (usa il token in cache, ok per il singleplayer) e **Back to login** (ti rimanda nel flusso di sign-in Microsoft, ha bisogno di internet).

Quindi per il singleplayer, "Launch anyway" funziona indipendentemente da quanto tempo fa eri online, il token non viene verificato da nulla una volta che Minecraft è partito. Per i server multiplayer che verificano l'identità, ti serve un token non scaduto, e questo significa essere stato online abbastanza recentemente da rinnovarlo.

### Perché i token scadono

È deciso dai server di auth di Microsoft e Mojang, non da GDLauncher. La catena di auth produce due token che contano per il launcher:

- Un **access token OAuth Microsoft** (~1 ora). È quello che il launcher usa per parlare con le API di auth di Microsoft / Xbox / Mojang. È a vita breve ma il launcher lo rinnova con un refresh token ogni volta che è online; di solito non te ne accorgi.
- Un **token di auth Minecraft** (~24 ore). È quello che viene passato a Minecraft al launch, quindi è quello che controlla il gioco offline. GDLauncher lo rinnova proattivamente circa 12 ore prima della scadenza mentre sei online.

Il refresh token di Microsoft dura mesi ma può essere invalidato server-side, per esempio quando cambi la password Microsoft, attivi una nuova feature di sicurezza, o ti disconnetti dal sito di Microsoft. Se il tuo refresh token viene invalidato mentre sei offline, non c'è nulla che il launcher possa fare finché non torni online per riautenticarti.

## Entrare in server multiplayer offline

**Non funziona**, perché i server multiplayer verificano la tua identità contro il session server di Mojang, che ha bisogno di internet da entrambe le parti. Il multiplayer LAN può funzionare tra macchine sulla stessa LAN offline finché entrambe hanno fatto l'auth online di recente.

## Installare nuove istanze, Mod o Modpack offline

**Non funziona.** Ogni flusso di install scarica da un CDN:

- I Modpack tirano il loro manifest e poi i file delle singole Mod.
- Aggiungere una Mod dalla tab Addons scarica il suo JAR.
- Creare un'istanza custom per una versione di Minecraft che non hai scarica il manifest JSON di quella versione, più il JAR della versione, più gli asset, più l'installer del mod loader.

Tutto questo fallirà offline con errori di timeout o DNS. Il launcher non riprova all'infinito, vedrai un fallimento nel modal di creazione dell'istanza o nel pannello Tasks.

Se sai che stai andando in un posto offline, pre-installa le istanze che vorrai prima di partire.

## Funzionalità dell'account GDL offline

**Per lo più non funzionano**, perché le funzionalità dell'account GDL sono per definizione "parlare al backend GDL". Nello specifico:

- Cloud Instance Sharing (generare un codice): fallisce, il servizio GDL è irraggiungibile.
- Importare un'istanza condivisa: fallisce per la stessa ragione.
- Modificare il tuo profilo GDL: fallisce.
- Elencare i tuoi share: mostra lo stato in cache, non può aggiornarsi.

Il launcher ricorda che sei connesso a GDL mentre sei offline, ma la UI mostra dati obsoleti e rifiuta le azioni che richiederebbero una chiamata di rete.

## TL;DR

- Istanza già installata, token fresco: il launch funziona offline.
- Istanza già installata, token scaduto: il launcher fa il prompt; scegli "Launch anyway" per il singleplayer.
- Multiplayer con token scaduto: bloccato finché non puoi raggiungere Microsoft per rinnovarlo.
- Qualunque cosa scarichi: bloccato.
- Qualunque cosa parli al backend GDL: bloccato.
- Mappe singleplayer: 100% capaci di funzionare offline una volta che l'istanza è su disco.
