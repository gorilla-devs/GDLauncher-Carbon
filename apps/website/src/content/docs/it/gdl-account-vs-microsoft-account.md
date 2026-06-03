---
title: "Account GDL vs Account Microsoft"
description: "GDLauncher usa due tipi di account diversi. Microsoft per giocare a Minecraft, GDL per la condivisione e le funzionalità social. Cosa sono, cosa sbloccano, e quali ti servono davvero."
faq:
  - question: "Mi serve un account GDL per giocare a Minecraft?"
    answer: "No. Per giocare a Minecraft ti serve solo un account Microsoft (quello con cui hai comprato Minecraft Java Edition). Un account GDL è opzionale e sblocca solo le funzionalità proprie di GDLauncher: condivisione cloud di istanze, codici amici, cronologia del display name, modifica del profilo. Puoi usare GDLauncher tranquillamente senza."
  - question: "Cosa sblocca un account GDL?"
    answer: "Oggi soprattutto la condivisione cloud di istanze: generi un codice da tasto destro → Share, e un altro utente GDLauncher lo incolla per importare l'istanza. Hai anche un display name stabile con cronologia dei rename e una scheda profilo con un codice amico che ti identifica nelle anteprime dei share. Tutto quello che riguarda parlare con altri utenti GDLauncher passa per l'account GDL."
  - question: "Posso usare GDLauncher senza un account Microsoft?"
    answer: "No. L'account Microsoft è ciò che dimostra che possiedi Minecraft e ottiene il token di avvio da Mojang. Senza, il launcher non ha nulla con cui autenticarsi ai server di Minecraft."
  - question: "Posso avere più account Microsoft in GDLauncher?"
    answer: "Sì. Settings → Accounts mostra tutti gli account Microsoft connessi in una tabella. Puoi aggiungerne altri, rimuovere quelli che non usi, e scegliere quale è attivo (quello che Play usa). L'account attivo è evidenziato nella colonna a sinistra dello username."
  - question: "Cos'è il codice amico sul mio profilo GDL?"
    answer: "Un identificatore corto e stabile per il tuo account GDL. Non cambia quando rinomini il display name, e viene mostrato nelle anteprime dei share così gli altri utenti possono capire chi sta condividendo. Copialo da Settings → Accounts → scheda profilo GDL Account."
---

# Account GDL vs Account Microsoft

## Due sistemi di account, un solo launcher

GDLauncher ha due sistemi di account. **Microsoft** è quello che dimostra che possiedi Minecraft ed è obbligatorio per giocare. **GDL** è l'account opzionale di GDLauncher, usato per funzionalità che coinvolgono il backend GDL (condivisione cloud di istanze, profilo, cronologia del display name).

### Account Microsoft

L'account con cui hai comprato Minecraft Java Edition, quello che possiede la licenza del gioco. Microsoft lo richiede per avviare Minecraft. GDLauncher si autentica con Microsoft, conserva i token risultanti, e li passa a Mojang al momento del launch così i server di Minecraft sanno che possiedi il gioco.

Ti serve almeno un account Microsoft connesso per giocare. Senza, Play non ha nulla da fare.

Memorizzato localmente per account: access token, refresh token, ID token, username e UUID di Minecraft, un riferimento alla skin, e la scadenza dell'access token. Il launcher rinnova l'access token in background usando il refresh token; di solito non te ne accorgi.

Cosa sblocca: avviare Minecraft, entrare nei server, possedere il gioco.

### Account GDL

Il sistema di account proprio di GDLauncher. È opzionale ed esiste per alimentare funzionalità che GDLauncher stesso fornisce, cose di cui Microsoft non si interessa e non dovrebbe interessarsi.

Ti registri con un'email e un display name, e ottieni un codice amico stabile. Da lì puoi usare le funzionalità che coinvolgono altri utenti GDLauncher.

Memorizzato localmente c'è solo il collegamento: a quale account Microsoft appartiene questa identità GDL, e un JWT per dialogare con il backend GDL. Display name, codice amico, email, foto profilo, e così via vivono sul backend GDL e la UI li recupera quando servono.

Cosa sblocca:

- **Cloud Instance Sharing.** Tasto destro su un'istanza → Share genera un codice che altri utenti GDLauncher possono incollare per importare l'istanza.
- **Cronologia del display name.** Rinominando il display name viene tracciata la cronologia dei cambi; puoi vedere i nomi passati dalla scheda profilo e cancellarli se vuoi.
- **Modifica profilo.** Display name, foto profilo, impostazioni di recupero email, tutto dalla scheda profilo GDL in Settings → Accounts.

## Quando ti serve ciascuno

| Scenario | Microsoft | GDL |
|---|---|---|
| Avviare Minecraft e basta | Obbligatorio | Non serve |
| Installare Mod e Modpack da CurseForge/Modrinth | Obbligatorio | Non serve |
| Condividere un'istanza con un amico | Obbligatorio | Obbligatorio |
| Ricevere un codice di share di un'istanza | Obbligatorio | Obbligatorio |
| Usare il sistema di amici | Obbligatorio | Obbligatorio |
| Giocare offline (istanza già installata) | L'auth in cache funziona per un po' | Non serve |

## Come gestirli

Entrambi vivono in **Settings → Accounts**.

La sezione GDL Account sta in cima alla pagina. Quando sei disconnesso mostra un pulsante Sign in / Sign up. Quando sei connesso mostra una scheda profilo con display name, codice amico (copiabile), email di recupero e stato di verifica. Una "Danger Zone" in fondo ti permette di pianificare la cancellazione dell'account (con un cooldown di 7 giorni).

La sezione Microsoft Accounts è una tabella sotto. Colonne: Active, Username, Type, Status, UUID, Actions. La colonna Status ti dice se il token di ogni account è fresco:

- **ok** (spunta verde): il token è valido, l'account può avviare.
- **expired** (avviso giallo): il token è scaduto. La colonna Actions mostra un'icona di refresh; cliccarla ti rimanda al flusso di sign-in Microsoft.
- **refreshing** (refresh giallo): il launcher sta rinnovando il token in background. Nessuna azione richiesta.
- **invalid** (X rossa): il token non è stato rinnovato. Stessa icona di refresh di expired; cliccarla ti guida nel flusso di sign-in Microsoft.

Per cambiare quale account è attivo, clicca la cella Active nella riga che vuoi. La riga attiva mostra un'icona double-check; le altre la mostrano sbiadita al passaggio del mouse.

## Rimuovere account

Rimuovere l'unico account Microsoft ti fa uscire completamente da GDLauncher e vieni rimandato alla home.

Rimuovere un account Microsoft che è il proprietario collegato del tuo account GDL fa apparire un modal di conferma; ti viene chiesto se vuoi davvero spezzare il collegamento prima che la cancellazione proceda.

Cancellare il tuo account GDL è un'azione differita di 7 giorni. Durante il cooldown puoi annullarla dalla stessa pagina.
