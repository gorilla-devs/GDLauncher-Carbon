---
title: "Compte GDL vs compte Microsoft"
description: "GDLauncher utilise deux types de comptes différents. Microsoft pour jouer à Minecraft, GDL pour le partage et les fonctionnalités sociales. Ce que chacun apporte et lesquels te sont nécessaires."
faq:
  - question: "Faut-il un compte GDL pour jouer à Minecraft ?"
    answer: "Non. Pour jouer, seul le compte Microsoft est nécessaire (celui avec lequel tu as acheté Minecraft Java Edition). Le compte GDL est optionnel et n'active que les fonctionnalités propres à GDLauncher : partage d'instance, friend code, historique de pseudo, édition de profil. Tu peux utiliser GDLauncher sans."
  - question: "Que débloque un compte GDL ?"
    answer: "Aujourd'hui, principalement le partage d'instance : tu génères un code via clic droit → Share, un autre utilisateur GDLauncher le colle pour importer l'instance. Tu obtiens aussi un pseudo stable avec historique de renommage et une carte de profil avec un friend code qui t'identifie dans les aperçus de partage. Tout ce qui implique d'autres utilisateurs GDLauncher passe par le compte GDL."
  - question: "Puis-je utiliser GDLauncher sans compte Microsoft ?"
    answer: "Non. Le compte Microsoft prouve que tu possèdes Minecraft et fournit le token de lancement Mojang. Sans lui, le launcher n'a rien à présenter aux serveurs de Minecraft."
  - question: "Puis-je avoir plusieurs comptes Microsoft dans GDLauncher ?"
    answer: "Oui. Settings → Accounts affiche tous les comptes Microsoft connectés dans un tableau. Tu peux en ajouter, en supprimer, et choisir lequel est actif (celui qu'utilise Play). Le compte actif est mis en évidence dans la colonne de gauche."
  - question: "C'est quoi le friend code de mon profil GDL ?"
    answer: "Un identifiant court et stable pour ton compte GDL. Il ne change pas si tu changes de pseudo, et il s'affiche dans les aperçus de partage pour que les autres voient qui a partagé. Copiable depuis Settings → Accounts → carte du compte GDL."
---

# Compte GDL vs compte Microsoft

## Deux systèmes de comptes, un seul launcher

GDLauncher a deux systèmes de comptes. **Microsoft** prouve que tu possèdes Minecraft et est nécessaire pour jouer. **GDL** est le compte optionnel propre à GDLauncher, pour les fonctionnalités qui utilisent le backend GDL (partage d'instance, profil, historique de pseudo).

### Compte Microsoft

Le compte avec lequel tu as acheté Minecraft Java Edition, celui qui détient la licence. Microsoft l'exige pour lancer Minecraft. GDLauncher se connecte à Microsoft, garde les tokens reçus, et les passe à Mojang au moment du lancement pour que les serveurs sachent que tu possèdes le jeu.

Il te faut au moins un compte Microsoft connecté pour pouvoir jouer. Sans, le bouton Play ne fait rien.

Stocké localement par compte : access token, refresh token, ID token, le pseudo Minecraft et l'UUID, une référence de skin, et l'expiration de l'access token. Le launcher rafraîchit l'access token en arrière-plan via le refresh token, tu ne le remarques en général pas.

Ce qu'il débloque : lancer Minecraft, rejoindre des serveurs, posséder le jeu.

### Compte GDL

Le système de comptes propre à GDLauncher. Optionnel. Il existe uniquement pour activer les fonctionnalités que GDLauncher fournit, celles qui ne concernent pas Microsoft.

Tu t'inscris avec un email et un pseudo, et tu reçois un friend code stable. À partir de là, tu peux utiliser les fonctionnalités qui impliquent d'autres utilisateurs GDLauncher.

Localement, seul le lien est stocké : à quel compte Microsoft appartient cette identité GDL, et un JWT pour parler au backend GDL. Pseudo, friend code, email, photo de profil, etc. vivent dans le backend GDL et l'UI les charge à la demande.

Ce qu'il débloque :

- **Partage d'instance.** Clic droit sur instance → Share génère un code que d'autres utilisateurs GDLauncher collent pour importer l'instance.
- **Historique de pseudo.** Renommer ton pseudo enregistre l'historique des changements ; tu peux voir les anciens noms depuis ta carte de profil et les effacer si tu veux.
- **Édition de profil.** Pseudo, photo de profil, paramètres d'email de récupération, tout depuis la carte de profil GDL dans Settings → Accounts.

## Quand chacun est nécessaire

| Scénario | Microsoft | GDL |
|---|---|---|
| Juste lancer Minecraft | Requis | Pas besoin |
| Installer mods et modpacks depuis CurseForge/Modrinth | Requis | Pas besoin |
| Partager une instance avec un ami | Requis | Requis |
| Recevoir un code d'instance | Requis | Requis |
| Utiliser le système d'amis | Requis | Requis |
| Jouer hors ligne (instance déjà installée) | Auth en cache marche un moment | Pas besoin |

## Comment les gérer

Les deux sont dans **Settings → Accounts**.

La section GDL Account est en haut. Déconnecté : un bouton Sign in / Sign up. Connecté : carte de profil avec pseudo, friend code (copiable), email de récupération, statut de vérification. Une zone "Danger Zone" en bas permet de programmer une suppression de compte avec un cooldown de 7 jours.

La section Microsoft Accounts est en dessous, en tableau. Colonnes : Active, Username, Type, Status, UUID, Actions. Status indique l'état du token par compte :

- **ok** (coche verte) : token valide, le compte peut lancer.
- **expired** (alerte jaune) : token expiré. La colonne Actions affiche une icône de refresh, un clic te renvoie dans le flow de connexion Microsoft.
- **refreshing** (refresh jaune) : le launcher rafraîchit le token en arrière-plan. Rien à faire.
- **invalid** (X rouge) : le token n'a pas pu être rafraîchi. Même icône refresh que pour expired, un clic te fait refaire le flow de connexion Microsoft.

Pour changer de compte actif, clique sur la cellule Active de la ligne voulue. La ligne active affiche une icône double-coche ; les autres l'affichent en faible au hover.

## Supprimer un compte

Supprimer le seul compte Microsoft te déconnecte entièrement de GDLauncher et tu reviens à la page d'accueil.

Supprimer un compte Microsoft lié à ton compte GDL ouvre une modale de confirmation, demandant si tu veux vraiment casser le lien avant la suppression.

Supprimer ton compte GDL est une action différée de 7 jours. Pendant le cooldown, tu peux l'annuler depuis la même page.
