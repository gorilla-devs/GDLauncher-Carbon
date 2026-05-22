---
title: "Instances de modpack verrouillées"
description: "Ce que ça veut dire qu'une instance de modpack soit verrouillée, pourquoi GDLauncher la verrouille, et comment déverrouiller (Unlock) ou désappairer (Unpair) au besoin."
---

## Qu'est-ce qu'une instance verrouillée ?

Quand tu installes un modpack depuis CurseForge ou Modrinth dans GDLauncher, l'instance est **verrouillée (locked)** par défaut. Une icône cadenas apparaît à côté de l'instance, et les actions qui modifieraient le contenu du pack, ajouter, retirer ou mettre à jour des mods individuels, sont désactivées. Tu peux toujours jouer, changer Java ou la RAM, prendre des screenshots, et tout le reste ; le verrou ne protège que le *jeu de mods géré par le pack*.

Le verrou existe parce qu'un modpack est une collection de mods testée et figée en versions. Les auteurs choisissent leur liste de mods avec soin et fixent des versions précises pour la compatibilité. Si tu remplaces un mod par une version plus récente, ça peut casser un autre mod qui dépendait de l'ancienne. Le verrou empêche cette erreur d'arriver.

## Ce que tu peux et ne peux pas faire en mode verrouillé

Pendant que l'instance est verrouillée, tu **peux** :

- Lancer et jouer l'instance.
- Changer la RAM, les arguments Java, et le Java Override.
- Faire des screenshots, parcourir les logs.
- Changer le nom et l'icône de l'instance (Edit Instance).
- Mettre à jour tout le modpack vers une release plus récente (Settings → Change Modpack Version).

Tu **ne peux pas** :

- Ajouter quoi que ce soit via l'onglet Addons, ça inclut **mods, shaders, resource packs, data packs et worlds**. Tant que le verrou est actif, le bouton Add est désactivé pour tous les types d'addons.
- Retirer ou désactiver un mod ou addon géré par le pack.
- Mettre à jour individuellement un mod géré par le pack.

Les onglets Mods et Addons affichent un message « Cette instance est verrouillée, les changements ne peuvent pas être appliqués » à côté des actions désactivées. Le bouton Install du navigateur Addons est lui aussi bloqué pour les instances verrouillées.

## Trois états : Locked / Unlocked / Unpaired

Ces trois termes apparaissent dans GDLauncher et ne sont pas interchangeables.

- **Locked (verrouillée)** : l'instance est appairée à un modpack CurseForge ou Modrinth, et le set de mods géré par le pack est en lecture seule. État par défaut après l'installation.
- **Unlocked (déverrouillée)** : toujours appairée au modpack (le nom et la version restent suivis), mais le set de mods devient librement éditable. GDLauncher se souvient du pack, donc tu peux toujours mettre à jour vers une nouvelle release plus tard. C'est à toi de garder le set de mods cohérent.
- **Unpaired (désappairée)** : plus aucune liaison avec le modpack. L'instance devient une instance custom, mêmes fichiers, mais GDLauncher ne suit plus les mises à jour du pack et ne la traite plus comme une instance modpack. Aller de Unlocked vers Unpaired est sans retour.

## Comment déverrouiller (Unlock)

1. Ouvre l'instance et clique sur la roue dentée (ou clic droit sur l'instance → Settings).
2. Va dans la section **Modpack Info** en haut de la page Settings. Tu vois l'icône, le nom et la version actuelle du pack, avec une rangée de boutons en dessous.
3. Clique sur **Unlock** (le bouton avec l'icône cadenas). L'instance passe en mode déverrouillé immédiatement.

Une fois déverrouillée, l'en-tête de section devient « Unlocked » avec l'icône cadenas ouvert. Tu peux reverrouiller via le même flux, mais en pratique, une fois que tu as commencé à maintenir le set de mods, peu de raisons de le faire.

## Comment désappairer (Unpair)

1. Dans la même section Modpack Info, clique sur **Unpair** (icône git-branch).
2. Confirme dans le modal. GDLauncher avertit que l'action est permanente.

Après désappairage, la section Modpack Info disparaît complètement. L'instance devient une instance custom et les options **Change Modpack Version** et **Reinstall** ne s'appliquent plus.

## Reinstall vs Unlock

La section Modpack Info propose aussi une action **Reinstall**. Elle est distincte de Unlock et a un objectif différent : réinstaller le modpack à sa version actuelle, en écrasant les mods gérés par le pack et les configs selon le manifest. À utiliser pour réparer une install cassée (jar corrompu, configs détruites, etc.) sans perdre tes mondes.

| Action | Effet sur les mods du pack | Lien avec le pack |
|--------|----------------------------|--------------------|
| Unlock | Conservés, mais éditables | Maintenu |
| Unpair | Conservés en fichiers, mais plus « mods de pack » | Supprimé |
| Reinstall | Reset à la version du manifest | Maintenu |
| Change Modpack Version | Remplacés par le manifest de la nouvelle version | Maintenu (nouvelle version) |

## Quand déverrouiller, et quand pas

Déverrouiller quand :
- Un mod du pack a un bug critique ou un fix de sécurité et le pack n'a pas été mis à jour.
- Tu veux ajouter ton propre mod, shader, resource pack, data pack ou world par-dessus ce que le pack apporte, le bouton Add de l'onglet Addons est désactivé par le verrou, il faut donc déverrouiller pour installer via l'UI.
- Tu maintiens toi-même un pack non maintenu.

Rester verrouillé quand :
- Le pack est activement maintenu, laisse l'auteur gérer le pinning de versions et attends la prochaine release.
- Tu joues une expérience curatée et tu ne veux pas dériver du set prévu.

Pattern courant : déverrouille brièvement, installe tes ajouts, puis laisse l'instance déverrouillée. Ce que tu as ajouté toi-même reste si tu reverrouilles plus tard, parce que le verrou ne concerne que le set *géré par le pack*, mais en pratique, peu de raisons de reverrouiller une fois que tu as commencé à maintenir l'instance.

## Ce que le verrou n'est pas

Le verrou n'est pas un système de permissions ni une frontière de sécurité. C'est un garde-fou pour éviter les édits de mods accidentels dans l'UI. Le dossier de l'instance sur le disque reste un dossier normal, tout ce qui écrit directement dans `mods` (un outil tiers, un copy/paste manuel) contourne complètement le verrou.

Les jars ajoutés ainsi apparaissent dans l'onglet Mods à côté des mods du pack. Pour les retirer, il faut passer par le système de fichiers, pas par l'UI.

## Dépannage rapide

- **« Je ne peux pas mettre à jour un seul mod. »** Le verrou fonctionne comme prévu. Soit Unlock (Settings → Unlock), soit Change Modpack Version pour mettre à jour le pack entier.
- **« Update All est grisé sur une instance verrouillée. »** Même raison. Utilise Change Modpack Version, ou déverrouille d'abord.
- **« Pourquoi mon mod user-added est encore visible après reverrouillage ? »** Le verrou s'applique aux mods du pack ; ceux que tu as ajoutés en plus restent visibles.
- **« Reinstall a écrasé une config que j'avais éditée. »** Comportement attendu. Reinstall ré-applique le manifest. Sauvegarde tes configs avant un reinstall.
