---
title: "GDL-Account vs Microsoft-Account"
description: "GDLauncher kennt zwei verschiedene Account-Typen. Microsoft zum Spielen, GDL für Sharing und soziale Features. Was sie sind, was sie freischalten und welche du brauchst."
faq:
  - question: "Brauche ich einen GDL-Account, um Minecraft zu spielen?"
    answer: "Nein. Zum Spielen brauchst du nur einen Microsoft-Account (den, mit dem du Minecraft Java Edition gekauft hast). Ein GDL-Account ist optional und schaltet nur GDLauncher-eigene Features frei: Instanz-Sharing, Friend Codes, Anzeigenamen-Historie, Profil-Bearbeitung. GDLauncher funktioniert auch ohne."
  - question: "Was schaltet ein GDL-Account frei?"
    answer: "Aktuell vor allem Instanz-Sharing: Du erzeugst per Rechtsklick → Share einen Code, ein anderer GDLauncher-Nutzer fügt ihn ein und importiert die Instanz. Dazu gibt's einen stabilen Anzeigenamen mit Rename-Historie und eine Profilkarte mit Friend Code, der dich in Share-Previews identifiziert. Alles, was mit anderen GDLauncher-Nutzern zu tun hat, läuft über den GDL-Account."
  - question: "Kann ich GDLauncher ohne Microsoft-Account nutzen?"
    answer: "Nein. Der Microsoft-Account beweist Minecraft-Besitz und liefert das Launch-Token von Mojang. Ohne ihn hat der Launcher nichts, womit er sich gegenüber Minecrafts Servern authentifizieren könnte."
  - question: "Kann ich mehrere Microsoft-Accounts in GDLauncher haben?"
    answer: "Ja. Settings → Accounts zeigt alle eingeloggten Microsoft-Accounts in einer Tabelle. Du kannst hinzufügen, entfernen und den aktiven wechseln (den nimmt Play). Der aktive Account ist in der linken Spalte markiert."
  - question: "Was ist der Friend Code in meinem GDL-Profil?"
    answer: "Eine kurze, stabile ID für deinen GDL-Account. Sie ändert sich nicht, wenn du deinen Anzeigenamen änderst, und sie taucht in Share-Previews auf, damit andere wissen, wer geteilt hat. Kopierbar aus Settings → Accounts → GDL-Account-Profilkarte."
---

# GDL-Account vs Microsoft-Account

## Zwei Account-Systeme, ein Launcher

GDLauncher hat zwei Account-Systeme. **Microsoft** beweist, dass dir Minecraft gehört, und ist zum Spielen Pflicht. **GDL** ist der optionale GDLauncher-eigene Account, für Features, die das GDL-Backend nutzen (Instanz-Sharing, Profil, Anzeigenamen-Historie).

### Microsoft-Account

Der Account, mit dem du Minecraft Java Edition gekauft hast, der die Lizenz hält. Microsoft verlangt ihn zum Starten. GDLauncher loggt sich bei Microsoft ein, behält die Tokens und reicht sie beim Launch an Mojang weiter, damit Minecrafts Server wissen, dass du das Spiel besitzt.

Mindestens ein eingeloggter Microsoft-Account ist Pflicht. Ohne hat der Play-Button nichts zu tun.

Lokal pro Account gespeichert: Access Token, Refresh Token, ID Token, Minecraft-Username und UUID, eine Skin-Referenz und die Token-Ablaufzeit. Das Access Token erneuert der Launcher im Hintergrund per Refresh Token; du merkst's normalerweise gar nicht.

Schaltet frei: Minecraft starten, Servern beitreten, das Spiel besitzen.

### GDL-Account

GDLaunchers eigenes Account-System. Optional. Existiert für Features, die GDLauncher selbst bringt, also Dinge, die Microsoft nicht interessieren sollten.

Du registrierst dich mit E-Mail und Anzeigenamen und bekommst einen stabilen Friend Code. Damit kannst du die Features nutzen, die andere GDLauncher-Nutzer einbeziehen.

Lokal gespeichert wird nur die Verknüpfung: welcher Microsoft-Account zu dieser GDL-Identität gehört, und ein JWT für die Kommunikation mit dem GDL-Backend. Anzeigename, Friend Code, E-Mail, Profilbild usw. liegen im GDL-Backend, die UI lädt sie bei Bedarf nach.

Schaltet frei:

- **Instanz-Sharing.** Rechtsklick → Share generiert einen Code, den andere GDLauncher-Nutzer per Paste importieren.
- **Anzeigenamen-Historie.** Beim Umbenennen wird die Änderungshistorie getrackt; alte Namen kannst du in der Profilkarte einsehen und bei Bedarf löschen.
- **Profil-Bearbeitung.** Anzeigename, Profilbild, Recovery-E-Mail-Einstellungen, alles aus der GDL-Profilkarte in Settings → Accounts.

## Was wann nötig ist

| Szenario | Microsoft | GDL |
|---|---|---|
| Nur Minecraft starten | Pflicht | nicht nötig |
| Mods/Modpacks aus CurseForge/Modrinth installieren | Pflicht | nicht nötig |
| Instanz mit Freund teilen | Pflicht | Pflicht |
| Share-Code empfangen | Pflicht | Pflicht |
| Friend-System nutzen | Pflicht | Pflicht |
| Offline spielen (bestehende Instanz) | Cache-Auth reicht kurz | nicht nötig |

## Verwaltung

Beide leben in **Settings → Accounts**.

Der GDL-Account-Bereich oben. Ausgeloggt zeigt er einen Sign in / Sign up-Button. Eingeloggt: Profilkarte mit Anzeigenamen, Friend Code (kopierbar), Recovery-E-Mail und Verifizierungsstatus. Eine "Danger Zone" unten bietet Account-Löschung mit 7-Tage-Cooldown.

Die Microsoft-Accounts-Sektion ist darunter eine Tabelle. Spalten: Active, Username, Type, Status, UUID, Actions. Status zeigt pro Account den Token-Zustand:

- **ok** (grüner Haken): Token gültig, Account kann starten.
- **expired** (gelbe Warnung): Token abgelaufen. In der Actions-Spalte erscheint ein Refresh-Icon, Klick schickt dich zurück durch den Microsoft-Sign-in-Flow.
- **refreshing** (gelbes Refresh): Der Launcher erneuert das Token gerade im Hintergrund. Kein Handeln nötig.
- **invalid** (rotes X): Token war nicht erneuerbar. Gleiches Refresh-Icon wie expired, Klick führt durch den Microsoft-Sign-in-Flow.

Aktiv wechseln per Klick auf die Active-Zelle der gewünschten Zeile. Die aktive Zeile zeigt ein Doppel-Häkchen, andere zeigen es schwach beim Hover.

## Accounts entfernen

Der einzige Microsoft-Account weg = komplett ausgeloggt, zurück zur Startseite.

Microsoft-Account weg, der mit deinem GDL-Account verlinkt ist = Bestätigungs-Modal, ob die Verbindung wirklich gebrochen werden soll, bevor gelöscht wird.

GDL-Account löschen ist eine 7-Tage-verzögerte Aktion. Während des Cooldowns kannst du sie auf derselben Seite abbrechen.
