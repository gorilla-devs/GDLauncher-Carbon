---
title: "Offline-Modus"
description: "Was du in GDLauncher mit und ohne Internetverbindung machen kannst. Was gecached ist, was nach Hause telefoniert, und wie sich das Token-Ablaufverhalten wirklich verhält."
faq:
  - question: "Kann ich Minecraft offline über GDLauncher spielen?"
    answer: "Ja. Singleplayer funktioniert komplett offline. Ist dein gecachtes Token noch gültig, drückst du Play und Minecraft startet ganz normal. Ist es abgelaufen, zeigt der Launcher einen 'Account Expired'-Prompt mit einem 'Launch anyway'-Button, wähl den und du kannst Singleplayer trotzdem spielen. Online-Auth ist nur für Multiplayer-Server nötig, die deine Identität über Mojang prüfen."
  - question: "Wie lange kann ich offline bleiben, bevor Tokens ablaufen?"
    answer: "Hängt davon ab, was du machen willst. Für Singleplayer gibt's quasi kein Zeitlimit, der Launcher bietet 'Launch anyway' an, sobald das Token abgelaufen ist. Multiplayer-Server, die Identität über Mojang prüfen, brauchen ein frisches Token, also musst du dafür wieder online gehen, um zu refreshen. Der Launcher refresht das Minecraft-Auth-Token proaktiv etwa 12 Stunden vor Ablauf seiner 24 Stunden, solange du kürzlich online warst, klappt Multiplayer also weiter."
  - question: "Kann ich offline neue Mods oder Modpacks installieren?"
    answer: "Nein. Mod-Downloads kommen von den CDNs von CurseForge und Modrinth, beide brauchen Internet. Gleiches für Java-Downloads, Minecraft-Asset-Downloads und Modpack-Manifeste. Alles Install-bezogene braucht Verbindung."
  - question: "Kann ich eine bestehende Instanz offline updaten?"
    answer: "Nein. Selber Grund: Updates holen neue Dateien aus den CDNs. Der Launcher reiht das Update ein und versucht's wieder, sobald er eine Verbindung sieht."
  - question: "Funktioniert der GDL-Account offline?"
    answer: "Teilweise. Der Launcher merkt sich, dass du in GDL eingeloggt bist, aber alles, was den GDL-Dienst kontaktiert (Instanz-Share, Profil-Edits, Liste deiner Shares), braucht Internet. Der Microsoft-Account ist der, der den Launch gatet; GDL ist für Features darüber hinaus."
---

# Offline-Modus

## Was "offline" hier wirklich heißt

GDLaunchers Offline-Verhalten hängt von drei verschiedenen Netzwerk-Bedürfnissen ab:

1. **Microsoft-Auth** (beweisen, dass du Minecraft besitzt, gegenüber Mojang).
2. **Mod- und Asset-Downloads** (CurseForge, Modrinth, Mojangs Libraries-CDN).
3. **GDL-Account-Features** (Instanz-Share, Profil, Anzeigenamen-Historie etc.).

Jedes scheitert offline anders, und der Launcher reagiert entsprechend unterschiedlich.

## Installierte Instanz offline starten

Der häufigste Fall: im Flugzeug, in der Hütte, oder daheim ohne Internet, du willst was Installiertes spielen.

**Klappt meistens**, weil GDLauncher die zum Launch nötigen Daten cached:

- Mojangs Auth-Tokens liegen lokal mit ihren Ablauf-Timestamps.
- Minecrafts Libraries und Assets sind auf der Platte (Runtime Path).
- Modded Instanzen haben ihre Mods lokal installiert.

Klickst du offline auf Play, läuft im Launcher:

1. Check, ob das Minecraft-Auth-Token des aktiven Microsoft-Accounts noch gültig ist (nicht abgelaufen).
2. Wenn ja, startet Minecraft direkt mit dem Token. Minecraft selbst braucht für Singleplayer-Welten kein Internet.
3. Ist das Access-Token abgelaufen, das Refresh-Token aber noch gültig, versucht der Launcher Microsofts Refresh-Endpoint anzurufen, was Internet braucht. Offline scheitert das, und der Status des Accounts wechselt in Settings → Accounts auf "expired".
4. Ist der Account abgelaufen und du klickst trotzdem auf Play, ploppt der Account-Expired-Modal mit zwei Buttons auf: **Launch anyway** (nimmt das gecachte Token, reicht für Singleplayer) und **Back to login** (schickt dich durch den Microsoft-Sign-in-Flow, braucht Internet).

Für Singleplayer klappt 'Launch anyway' also egal wie lange du schon offline bist, das Token wird nach dem Start von Minecraft nicht mehr geprüft. Für Multiplayer-Server, die Identität verifizieren, brauchst du ein nicht abgelaufenes Token, also musst du kürzlich online genug gewesen sein, um zu refreshen.

### Warum Tokens ablaufen

Das wird von Microsofts und Mojangs Auth-Servern gesetzt, nicht von GDLauncher. Die Auth-Kette erzeugt zwei Tokens, die für den Launcher relevant sind:

- Ein **Microsoft-OAuth-Access-Token** (~1 Stunde). Damit redet der Launcher mit Microsoft, Xbox und Mojangs Auth-APIs. Kurzlebig, wird aber vom Launcher mit einem Refresh-Token erneuert, sobald online; merkst du selten.
- Ein **Minecraft-Auth-Token** (~24 Stunden). Das wird Minecraft beim Launch übergeben, also entscheidet das über Offline-Play. GDLauncher refresht es online proaktiv etwa 12 Stunden vor Ablauf.

Microsofts Refresh-Token hält Monate, kann aber serverseitig entwertet werden, z. B. wenn du dein Microsoft-Passwort änderst, ein neues Sicherheits-Feature aktivierst oder dich auf Microsofts Webseite abmeldest. Wird dein Refresh-Token während der Offline-Zeit entwertet, kann der Launcher nichts machen, bis du wieder online bist und neu authentifizieren kannst.

## Offline Multiplayer-Servern beitreten

**Geht nicht**, weil Multiplayer-Server deine Identität gegen Mojangs Session-Server prüfen, was auf beiden Seiten Internet braucht. LAN-Multiplayer zwischen Rechnern im selben Offline-LAN klappt, wenn beide kürzlich online authentifiziert haben.

## Offline neue Instanzen, Mods oder Modpacks installieren

**Geht nicht.** Jeder Install-Flow lädt aus dem CDN:

- Modpacks ziehen ihr Manifest und dann einzelne Mod-Dateien.
- Ein Mod aus dem Addons-Tab hinzufügen lädt sein JAR.
- Eine Custom-Instanz für eine Minecraft-Version anlegen, die du nicht hast, lädt das JSON-Manifest dieser Version, die Version-JAR, Assets und den Modloader-Installer.

Alles davon fällt offline mit Timeout oder DNS-Fehlern aus. Der Launcher versucht es nicht endlos, du siehst den Fehler im Erstellungs-Modal oder im Tasks-Panel.

Wenn du weißt, dass du offline gehst, installier die Instanzen vorher.

## GDL-Account-Features offline

**Klappt meistens nicht**, weil GDL-Features per Definition "mit dem GDL-Backend reden" sind. Konkret:

- Instanz-Share (Code generieren): scheitert, GDL-Dienst nicht erreichbar.
- Geteilte Instanz importieren: scheitert aus demselben Grund.
- GDL-Profil bearbeiten: scheitert.
- Liste deiner Shares ansehen: zeigt gecachten Stand, kann nicht refreshen.

Der Launcher merkt sich offline, dass du in GDL eingeloggt bist, die UI zeigt aber alte Daten und lehnt Aktionen ab, die einen Netzwerk-Call brauchen.

## TL;DR

- Bereits installierte Instanz, frisches Token: Offline-Launch geht.
- Bereits installierte Instanz, abgelaufenes Token: Launcher fragt nach; für Singleplayer 'Launch anyway' wählen.
- Multiplayer mit abgelaufenem Token: blockiert, bis du Microsoft zum Refreshen erreichst.
- Alles, was lädt: blockiert.
- Alles, was mit dem GDL-Backend redet: blockiert.
- Singleplayer-Welten: 100% offline-fähig, sobald die Instanz auf der Platte ist.
