---
title: "Errori di autenticazione Microsoft"
description: "Risolvi i comuni errori di autenticazione Microsoft in GDLauncher. Soluzioni per Invalid Grant, account bannato, accesso console richiesto ed errori Xbox Live."
faq:
  - question: "Perché ricevo un errore 'Invalid Grant' in GDLauncher?"
    answer: "Un errore 'Invalid Grant' di solito significa che c'è un problema con la sicurezza del tuo account Microsoft. Le soluzioni più comuni sono attivare l'autenticazione a due fattori sull'account Microsoft, impostare una password se non ne avevi una, oppure disconnetterti e accedere di nuovo."
  - question: "Perché GDLauncher dice che il mio account è bannato?"
    answer: "Se GDLauncher segnala il tuo account come bannato, il ban viene da Mojang o Microsoft, non da GDLauncher. Accedi a minecraft.net o al tuo account Microsoft per vedere il motivo del ban. GDLauncher si limita a riportare la risposta dell'autenticazione, non c'è alcuna lista ban lato GDLauncher."
  - question: "Perché GDLauncher dice che mi serve l'accesso alla console?"
    answer: "Di solito compare per account bambino o account con restrizioni di gruppo famiglia. L'account genitore deve concedere all'account bambino il permesso di giocare a Minecraft sulla piattaforma che stai cercando di usare. Modifica le impostazioni della famiglia su account.microsoft.com/family."
  - question: "Continuo a ricevere errori di autenticazione Xbox Live. Cosa faccio?"
    answer: "Gli errori Xbox Live di solito significano che l'impostazione paese/regione sull'account Microsoft non permette Xbox Live, oppure che l'account non ha accettato i termini di servizio Xbox Live. Accedi una volta su xbox.com con lo stesso account Microsoft per accettare i termini, poi riprova GDLauncher."
  - question: "Devo ricomprare Minecraft per usare GDLauncher?"
    answer: "No. GDLauncher usa il tuo account Microsoft / Mojang di Minecraft esistente. Non c'è alcun acquisto o abbonamento separato. Se possiedi già Minecraft Java Edition, puoi accedere a GDLauncher con lo stesso account."
---

# Errori di autenticazione Microsoft

Quando accedi a GDLauncher con un account Microsoft, il launcher dialoga per tuo conto con il servizio OAuth di Microsoft e con l'API di autenticazione di Mojang. Gli errori di questi servizi vengono mostrati direttamente nel launcher; il testo arriva da Microsoft, non da GDLauncher.

Qui sotto trovi i più comuni e cosa significano.

## Invalid Grant

Compare quando Microsoft rifiuta lo scambio OAuth. Le cause più comuni:

- L'account non ha una password impostata (è un account Microsoft creato tramite link via email o accesso social). Aggiungi una password su [account.microsoft.com](https://account.microsoft.com).
- L'account usa un flusso di accesso datato senza autenticazione a due fattori. Attivare la 2FA su [account.microsoft.com/security](https://account.microsoft.com/security) risolve nella maggior parte dei casi.
- I token in cache sono obsoleti. Disconnetti l'account in **Settings → Accounts** e accedi di nuovo.

## Account bannato

GDLauncher riporta la risposta di Mojang invariata. Il ban è lato Mojang; GDLauncher non mantiene una propria lista ban. Accedi su [minecraft.net](https://minecraft.net) con lo stesso account per vedere il motivo del ban e le opzioni di ricorso.

## Console access required

Di solito compare per account bambino dentro un gruppo famiglia Microsoft. L'account genitore deve autorizzare Minecraft Java Edition per il bambino su [account.microsoft.com/family](https://account.microsoft.com/family). Dopo aver concesso il permesso, disconnetti e accedi di nuovo in GDLauncher.

## Errori Xbox Live

La maggior parte dei problemi con Xbox Live rientra in due categorie:

- Il paese/regione sull'account Microsoft non permette Xbox Live. Modificalo su [account.microsoft.com/profile](https://account.microsoft.com/profile).
- L'account non ha accettato i termini di servizio Xbox Live. Accedi una volta su [xbox.com](https://xbox.com) con lo stesso account Microsoft per accettarli, poi riprova GDLauncher.

## Account expired

Il refresh token Microsoft è scaduto o è stato revocato (spesso perché hai cambiato la password dell'account altrove). GDLauncher mostra un prompt "Account expired" e ti offre di riautenticarti. Accedi di nuovo da **Settings → Accounts**.

## Quando hai dubbi

Se il messaggio di errore non corrisponde a nessuno dei precedenti, condividi entrambi i log dell'app sul nostro [Discord](https://discord.gdlauncher.com): `main.log` (Electron) e il più recente `__gdl_logs__/<timestamp>.log` (core Rust). Vedi [Share App Logs](/guides/share-app-logs) per scoprire dove trovarli. Ne servono quasi sempre due, il flusso di autenticazione attraversa entrambi i processi.
