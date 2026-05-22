---
title: "Erreurs d'authentification Microsoft"
description: "Résous les erreurs d'authentification Microsoft les plus courantes dans GDLauncher. Solutions pour Invalid Grant, compte banni, accès console requis et erreurs Xbox Live."
faq:
  - question: "Pourquoi ai-je une erreur 'Invalid Grant' dans GDLauncher ?"
    answer: "Une erreur 'Invalid Grant' signifie généralement qu'il y a un problème de sécurité avec ton compte Microsoft. Les correctifs les plus fréquents : activer l'authentification à deux facteurs sur ton compte Microsoft, définir un mot de passe si tu n'en as pas, ou se déconnecter puis se reconnecter."
  - question: "Pourquoi GDLauncher dit-il que mon compte est banni ?"
    answer: "Si GDLauncher signale ton compte comme banni, le bannissement vient de Mojang ou Microsoft, pas de GDLauncher. Connecte-toi sur minecraft.net ou ton compte Microsoft pour voir la raison du bannissement. GDLauncher ne fait que relayer la réponse d'authentification, il n'y a pas de liste de bannis côté GDLauncher."
  - question: "Pourquoi GDLauncher dit-il que j'ai besoin d'un accès console ?"
    answer: "Cela apparaît typiquement pour les comptes enfants ou les comptes soumis à des restrictions de groupe familial. Le compte parent doit autoriser le compte enfant à jouer à Minecraft sur la plateforme utilisée. Ajuste les paramètres familiaux sur account.microsoft.com/family."
  - question: "Je reçois sans cesse des erreurs d'authentification Xbox Live. Que faire ?"
    answer: "Les erreurs Xbox Live signifient généralement que le pays/région du compte Microsoft n'autorise pas Xbox Live, ou que le compte n'a pas accepté les conditions d'utilisation Xbox Live. Connecte-toi une fois sur xbox.com avec le même compte Microsoft pour accepter les conditions, puis réessaie dans GDLauncher."
  - question: "Dois-je racheter Minecraft pour utiliser GDLauncher ?"
    answer: "Non. GDLauncher utilise ton compte Minecraft Microsoft / Mojang existant. Aucun achat ni abonnement séparé. Si tu possèdes déjà Minecraft Java Edition, tu peux te connecter à GDLauncher avec le même compte."
---

# Erreurs d'authentification Microsoft

Quand tu te connectes à GDLauncher avec un compte Microsoft, le launcher parle pour toi avec le service OAuth de Microsoft et l'API d'authentification de Mojang. Les erreurs renvoyées par ces services apparaissent directement dans le launcher ; les textes viennent de Microsoft, pas de GDLauncher.

Voici les plus fréquentes et ce qu'elles veulent dire.

## Invalid Grant

Apparaît quand Microsoft refuse l'échange OAuth. Les causes les plus courantes :

- Le compte n'a pas de mot de passe défini (compte Microsoft créé via un lien email ou un login social). Ajoute un mot de passe sur [account.microsoft.com](https://account.microsoft.com).
- Le compte utilise un ancien flow de connexion sans authentification à deux facteurs. Activer la 2FA sur [account.microsoft.com/security](https://account.microsoft.com/security) règle ça pour la plupart.
- Les tokens en cache sont périmés. Déconnecte le compte dans **Settings → Accounts** et reconnecte-toi.

## Compte banni

GDLauncher transmet la réponse de Mojang telle quelle. Le ban est côté Mojang ; GDLauncher ne maintient pas sa propre liste de bans. Connecte-toi sur [minecraft.net](https://minecraft.net) avec le même compte pour voir la raison du ban et les options de recours.

## Accès console requis

Apparaît surtout pour les comptes enfants dans un groupe familial Microsoft. Le compte parent doit autoriser Minecraft Java Edition pour l'enfant sur [account.microsoft.com/family](https://account.microsoft.com/family). Après autorisation, déconnecte-toi et reconnecte-toi dans GDLauncher.

## Erreurs Xbox Live

La plupart des erreurs Xbox Live tombent dans l'une des deux catégories :

- Le pays/région du compte Microsoft n'autorise pas Xbox Live. Ajuste sur [account.microsoft.com/profile](https://account.microsoft.com/profile).
- Le compte n'a pas accepté les conditions d'utilisation Xbox Live. Connecte-toi une fois sur [xbox.com](https://xbox.com) avec le même compte pour les accepter, puis réessaie dans GDLauncher.

## Compte expiré

Le refresh token Microsoft a expiré ou a été révoqué (le plus souvent parce que tu as changé le mot de passe du compte ailleurs). GDLauncher affiche un prompt "Account expired" et propose de te réauthentifier. Reconnecte-toi depuis **Settings → Accounts**.

## En cas de doute

Si le message d'erreur ne correspond à rien de tout ça, partage les deux logs au niveau de l'app sur notre [Discord](https://discord.gdlauncher.com) : `main.log` (Electron) et le plus récent `__gdl_logs__/<timestamp>.log` (Rust core). Pour savoir où les trouver, voir [Share App Logs](/guides/share-app-logs). On a presque toujours besoin des deux, l'authentification traverse les deux processus.
