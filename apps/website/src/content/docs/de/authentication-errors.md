---
title: "Microsoft-Authentifizierungsfehler"
description: "Behebe häufige Microsoft-Authentifizierungsfehler in GDLauncher. Lösungen für Invalid Grant, gesperrte Konten, erforderlichen Konsolenzugriff und Xbox-Live-Fehler."
faq:
  - question: "Warum erhalte ich in GDLauncher einen 'Invalid Grant'-Fehler?"
    answer: "Ein 'Invalid Grant'-Fehler bedeutet meist, dass es ein Problem mit der Sicherheit deines Microsoft-Kontos gibt. Häufige Lösungen: Aktiviere die Zwei-Faktor-Authentifizierung für dein Microsoft-Konto, lege ein Passwort fest, falls noch keines gesetzt ist, oder melde dich ab und wieder an."
  - question: "Warum sagt GDLauncher, dass mein Konto gesperrt ist?"
    answer: "Wenn GDLauncher dein Konto als gesperrt meldet, kommt die Sperre von Mojang oder Microsoft, nicht von GDLauncher. Melde dich auf minecraft.net oder bei deinem Microsoft-Konto an, um den Sperrgrund einzusehen. GDLauncher gibt nur die Authentifizierungsantwort weiter, es gibt keine GDLauncher-eigene Sperrliste."
  - question: "Warum sagt GDLauncher, dass ich Konsolenzugriff benötige?"
    answer: "Das tritt typischerweise bei Kinder-Konten oder Konten mit Familiengruppen-Einschränkungen auf. Das Eltern-Konto muss dem Kind-Konto die Erlaubnis erteilen, Minecraft auf der gewünschten Plattform zu spielen. Passe die Familieneinstellungen unter account.microsoft.com/family an."
  - question: "Ich erhalte ständig Xbox-Live-Authentifizierungsfehler. Was tun?"
    answer: "Xbox-Live-Fehler bedeuten meist, dass die Land-/Region-Einstellung deines Microsoft-Kontos kein Xbox Live erlaubt oder das Konto den Xbox-Live-Nutzungsbedingungen noch nicht zugestimmt hat. Melde dich einmal auf xbox.com mit demselben Microsoft-Konto an, um die Bedingungen zu akzeptieren, und versuche es dann erneut in GDLauncher."
  - question: "Muss ich Minecraft erneut kaufen, um GDLauncher zu nutzen?"
    answer: "Nein. GDLauncher verwendet dein bestehendes Microsoft-/Mojang-Minecraft-Konto. Es gibt keinen separaten Kauf und kein Abo. Wenn du Minecraft Java Edition bereits besitzt, kannst du dich mit demselben Konto in GDLauncher anmelden."
---

# Microsoft-Authentifizierungsfehler

Wenn du dich in GDLauncher mit einem Microsoft-Konto anmeldest, redet der Launcher in deinem Namen mit Microsofts OAuth-Service und Mojangs Authentifizierungs-API. Fehler aus diesen Diensten werden direkt im Launcher angezeigt; der Wortlaut kommt von Microsoft, nicht von GDLauncher.

Hier die häufigsten und was sie bedeuten.

## Invalid Grant

Erscheint, wenn Microsoft den OAuth-Austausch ablehnt. Häufige Ursachen:

- Das Konto hat kein Passwort gesetzt (ein Microsoft-Konto, das per E-Mail-Link oder Social-Login erstellt wurde). Setze ein Passwort unter [account.microsoft.com](https://account.microsoft.com).
- Das Konto nutzt einen älteren Anmelde-Flow ohne Zwei-Faktor-Authentifizierung. 2FA bei [account.microsoft.com/security](https://account.microsoft.com/security) aktivieren behebt es bei den meisten.
- Die gecachten Tokens sind veraltet. Melde dich in **Settings → Accounts** ab und wieder an.

## Konto gesperrt

GDLauncher reicht Mojangs Antwort unverändert weiter. Die Sperre kommt von Mojang; GDLauncher führt keine eigene Sperrliste. Melde dich bei [minecraft.net](https://minecraft.net) mit demselben Konto an, um den Sperrgrund und Einspruchsmöglichkeiten zu sehen.

## Konsolenzugriff erforderlich

Erscheint meistens bei Kinder-Konten in einer Microsoft-Familiengruppe. Das Eltern-Konto muss dem Kind Minecraft Java Edition unter [account.microsoft.com/family](https://account.microsoft.com/family) freigeben. Nach der Freigabe in GDLauncher abmelden und wieder anmelden.

## Xbox-Live-Fehler

Xbox-Live-Probleme fallen meist in eine von zwei Kategorien:

- Die Land-/Region-Einstellung des Microsoft-Kontos erlaubt kein Xbox Live. Unter [account.microsoft.com/profile](https://account.microsoft.com/profile) anpassen.
- Das Konto hat den Xbox-Live-Nutzungsbedingungen noch nicht zugestimmt. Einmal bei [xbox.com](https://xbox.com) mit demselben Microsoft-Konto anmelden, akzeptieren, dann in GDLauncher erneut versuchen.

## Konto abgelaufen

Microsofts Refresh-Token ist abgelaufen oder widerrufen (meistens, weil du dein Konto-Passwort woanders geändert hast). GDLauncher zeigt einen "Account expired"-Prompt und bietet eine neue Authentifizierung an. Aus **Settings → Accounts** wieder anmelden.

## Wenn nichts passt

Wenn die Fehlermeldung zu keinem der obigen passt, beide App-Level-Logs auf unserem [Discord](https://discord.gdlauncher.com) teilen: `main.log` (Electron) und das neueste `__gdl_logs__/<timestamp>.log` (Rust-Core). Wo sie liegen, siehe [Share App Logs](/guides/share-app-logs). Wir brauchen fast immer beide, der Authentifizierungs-Flow läuft über beide Prozesse.
