---
title: "Mode hors ligne"
description: "Ce que tu peux et ne peux pas faire dans GDLauncher sans connexion internet. Ce qui est mis en cache, ce qui doit téléphoner à la maison, et comment se passe réellement l'expiration des tokens."
faq:
  - question: "Puis-je jouer à Minecraft hors ligne via GDLauncher ?"
    answer: "Oui. Le solo fonctionne entièrement hors ligne. Si ton token en cache est encore valide, tu cliques Play et Minecraft démarre normalement. S'il a expiré, le launcher affiche un prompt 'Account Expired' avec un bouton 'Launch anyway' ; choisis-le et tu peux toujours jouer en solo. L'auth en ligne n'est requise que pour les serveurs multijoueurs qui vérifient l'identité via Mojang."
  - question: "Combien de temps puis-je rester hors ligne avant que les tokens expirent ?"
    answer: "Ça dépend de ce que tu veux faire. Pour le solo, pas vraiment de limite de temps : le launcher proposera 'Launch anyway' une fois le token expiré. Pour les serveurs multijoueurs qui vérifient l'identité via Mojang, tu as besoin d'un token frais, donc retour en ligne pour rafraîchir. Le launcher rafraîchit le token d'auth Minecraft proactivement environ 12 heures avant son expiration à 24 heures, donc tant que tu as été en ligne récemment, le multijoueur continue de marcher."
  - question: "Puis-je installer de nouveaux mods ou modpacks hors ligne ?"
    answer: "Non. Les téléchargements de mods passent par les CDN de CurseForge et Modrinth, qui demandent internet. Pareil pour les téléchargements Java, les assets Minecraft et les manifests de modpacks. Tout ce qui est lié à l'installation a besoin d'une connexion."
  - question: "Puis-je mettre à jour une instance existante hors ligne ?"
    answer: "Non. Même raison : les mises à jour tirent de nouveaux fichiers depuis les CDN. Le launcher met la mise à jour en attente et réessaiera quand il verra une connexion."
  - question: "Et le compte GDL, marche-t-il hors ligne ?"
    answer: "Partiellement. Le launcher se souvient que tu es connecté en GDL, mais tout ce qui demande de parler au service GDL (partage d'instance, édition de profil, consulter tes partages) demande internet. Le compte Microsoft est ce qui contrôle le lancement ; GDL c'est pour les fonctionnalités au-delà du lancement."
---

# Mode hors ligne

## Ce que veut vraiment dire "hors ligne" ici

Le comportement hors ligne de GDLauncher dépend de trois besoins réseau différents :

1. **Auth Microsoft** (prouver à Mojang que tu possèdes Minecraft).
2. **Téléchargements de mods et d'assets** (CurseForge, Modrinth, CDN libraries de Mojang).
3. **Fonctionnalités du compte GDL** (partage d'instance, profil, historique de pseudo, etc.).

Chacun échoue différemment quand internet est coupé, et le comportement du launcher est différent dans chaque cas.

## Lancer une instance installée hors ligne

Le scénario le plus courant : tu es dans un avion, dans un chalet, ou ton internet à la maison est coupé, et tu veux jouer à quelque chose que tu as déjà installé.

**Marche en général**, parce que GDLauncher met en cache les données nécessaires au lancement :

- Les tokens d'auth Mojang sont stockés localement avec leurs timestamps d'expiration.
- Les bibliothèques et assets de Minecraft sont déjà sur le disque (dans le runtime path).
- Les instances moddées ont leurs mods installés localement.

Quand tu cliques Play hors ligne, le launcher :

1. Vérifie si le token d'auth Minecraft du compte Microsoft actif est encore valide (non expiré).
2. Si oui, lance Minecraft directement avec ce token. Minecraft lui-même n'a pas besoin d'internet pour lancer un monde solo.
3. Si le token d'accès est expiré mais le refresh token encore valide, le launcher essaie d'appeler l'endpoint de refresh de Microsoft, ce qui demande internet. Hors ligne, cet appel échoue, et le statut du compte passe à "expired" dans Settings → Accounts.
4. Si le compte est expiré et que tu cliques quand même Play, le launcher ouvre un modal Account Expired avec deux boutons : **Launch anyway** (utilise le token en cache, ok pour le solo) et **Back to login** (t'envoie dans le flow de sign-in Microsoft, demande internet).

Donc pour le solo, 'Launch anyway' marche peu importe depuis quand tu n'as pas été en ligne : le token n'est pas vérifié par quoi que ce soit une fois Minecraft lancé. Pour les serveurs multijoueurs qui vérifient l'identité, il te faut un token non expiré, donc avoir été en ligne assez récemment pour rafraîchir.

### Pourquoi les tokens expirent

C'est défini par les serveurs d'auth de Microsoft et de Mojang, pas par GDLauncher. La chaîne d'auth produit deux tokens qui comptent pour le launcher :

- Un **token d'accès OAuth Microsoft** (~1 heure). C'est ce que le launcher utilise pour parler aux APIs d'auth Microsoft / Xbox / Mojang. Court mais le launcher le renouvelle avec un refresh token dès qu'il est en ligne ; tu le remarques rarement.
- Un **token d'auth Minecraft** (~24 heures). C'est celui qui est passé à Minecraft au lancement, donc c'est celui qui contrôle le jeu hors ligne. GDLauncher le rafraîchit proactivement environ 12 heures avant expiration tant que tu es en ligne.

Le refresh token de Microsoft dure des mois mais peut être invalidé côté serveur, par exemple quand tu changes ton mot de passe Microsoft, actives une nouvelle fonctionnalité de sécurité, ou te déconnectes depuis le site Microsoft. Si ton refresh token est invalidé pendant que tu es hors ligne, il n'y a rien que le launcher puisse faire tant que tu n'es pas revenu en ligne pour te réauthentifier.

## Rejoindre des serveurs multijoueurs hors ligne

**Ne marche pas**, parce que les serveurs multijoueurs vérifient ton identité contre le session server de Mojang, ce qui demande internet des deux côtés. Le multijoueur LAN peut marcher entre des machines sur le même LAN hors ligne tant que les deux ont récemment authentifié en ligne.

## Installer de nouvelles instances, mods ou modpacks hors ligne

**Ne marche pas.** Chaque flow d'installation télécharge depuis un CDN :

- Les modpacks tirent leur manifest puis les fichiers de mods individuels.
- Ajouter un mod depuis l'onglet Addons télécharge son JAR.
- Créer une instance custom pour une version Minecraft que tu n'as pas télécharge le manifest JSON de cette version, plus le JAR de la version, plus les assets, plus l'installeur du mod loader.

Tout ça échouera hors ligne avec des erreurs de timeout ou DNS. Le launcher ne réessaie pas indéfiniment, tu verras un échec dans le modal de création d'instance ou dans le panneau Tasks.

Si tu sais que tu vas dans un endroit hors ligne, pré-installe les instances que tu voudras avant de partir.

## Fonctionnalités du compte GDL hors ligne

**Ne marche pas pour la plupart**, parce que les fonctionnalités du compte GDL sont par définition "parler au backend GDL". Spécifiquement :

- Partage d'instance (générer un code) : échec, service GDL inaccessible.
- Importer une instance partagée : échec pour la même raison.
- Éditer ton profil GDL : échec.
- Voir tes partages : affiche l'état en cache, ne peut pas rafraîchir.

Le launcher se souvient que tu es connecté en GDL pendant que tu es hors ligne, mais l'UI affiche des données périmées et refuse les actions qui demanderaient un appel réseau.

## TL;DR

- Instance déjà installée, token frais : lancement hors ligne marche.
- Instance déjà installée, token expiré : le launcher demande, choisis 'Launch anyway' pour le solo.
- Multijoueur avec token expiré : bloqué jusqu'à pouvoir atteindre Microsoft pour rafraîchir.
- Tout ce qui télécharge : bloqué.
- Tout ce qui parle au backend GDL : bloqué.
- Mondes solo : 100% capables hors ligne dès que l'instance est sur disque.
