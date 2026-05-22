---
title: "Compatibilité du format de sauvegarde"
description: "Pourquoi mettre à jour un monde Minecraft vers une nouvelle version est généralement à sens unique, comment le format de sauvegarde change vraiment, et comment sauvegarder proprement avant."
faq:
  - question: "Puis-je ouvrir un monde Minecraft 1.21 en 1.20 ?"
    answer: "Pas en toute sécurité. Minecraft migre les mondes uniquement vers l'avant, jamais en arrière. Un monde ouvert en 1.21 a son level.dat et ses region files réécrits dans le nouveau format ; les anciennes versions refusent de le charger ou crashent. Si tu as besoin des deux, fais une copie du monde avant de lancer la version plus récente."
  - question: "GDLauncher prévient-il avant de mettre à jour un monde ?"
    answer: "Le launcher en lui-même non, l'avertissement vient de Minecraft. En ouvrant un monde sauvegardé dans une version plus ancienne, Minecraft affiche une boîte de dialogue 'Ce monde a été sauvegardé avec une version différente' avant de charger. C'est le moment de revenir en arrière et de copier le dossier du monde ailleurs."
  - question: "Qu'est-ce qui est réécrit lors d'une mise à jour de monde ?"
    answer: "level.dat (métadonnées du monde), les region files dans region/ (données de chunks), playerdata/ (état par joueur) et tous les data packs propres au monde. Le champ Data Version dans level.dat est mis à jour pour correspondre à la nouvelle version Minecraft ; c'est ce champ que les versions plus récentes/anciennes lisent pour décider si elles peuvent ouvrir le monde."
  - question: "Le downgrade est-il impossible ?"
    answer: "Au sens strict oui. Il n'y a pas de chemin de downgrade officiel. Certains outils communautaires prétendent rétablir le Data Version mais ne réécrivent pas réellement les chunks, et le monde finit partiellement corrompu (biomes/blocs/entités plus récents que la version ne connaît pas). Considère les mises à jour comme à sens unique."
  - question: "Comment sauvegarder un monde avant la mise à jour ?"
    answer: "Clic droit sur l'instance dans GDLauncher → Open Folder. Va dans instance/saves et copie le dossier du monde (nommé comme dans la liste) quelque part en dehors du dossier d'instance. Garde cette copie tant que tu n'as pas la certitude que la nouvelle version marche bien."
---

# Compatibilité du format de sauvegarde

## Pourquoi les formats de sauvegarde changent

Le format de fichier des mondes Minecraft n'est pas figé. Chaque grosse mise à jour révise la structure des données sur disque. Nouveaux blocs = nouvelles IDs. Nouvelles entités = nouvelles formes NBT. Nouveaux biomes = nouveau registre de biomes. En coulisses, chaque monde a un nombre appelé **Data Version** dans `level.dat`, et Minecraft s'en sert pour décider quoi faire à l'ouverture.

Si la Data Version de ton monde est plus ancienne que celle de la version actuelle de Minecraft, Minecraft lance un passage de **DataFixer** unique qui réécrit le monde au nouveau format. Chunks, entités, états de blocs, données de joueur, tout est converti au nouveau schéma. Le Data Version dans `level.dat` est mis à jour pour matcher.

Cette conversion est **destructive et à sens unique**. Une fois les chunks réécrits, la version Minecraft plus ancienne ne peut plus les lire.

## Ce que veut vraiment dire "sens unique"

Imagine un monde 1.20.1. Tu l'ouvres en 1.21. Minecraft affiche l'avertissement "version différente", tu cliques "Convertir" (ou charges quand même), et le jeu démarre. En coulisses :

- `level.dat` est réécrit de sorte que son champ `DataVersion` corresponde à 1.21 au lieu de 1.20.1.
- Chaque region file dans `region/` qui est chargé (au minimum tout dans la distance d'affichage) est réécrit chunk par chunk.
- De nouveaux blocs 1.21 comme Crafter ou Trial Spawner peuvent désormais exister dans le monde ; ils n'existent pas dans le registre de blocs 1.20.1.
- Les entités et tile entities 1.20.1 existantes sont migrées vers les schémas 1.21.

Si tu essaies maintenant d'ouvrir le même dossier en 1.20.1 :

- Minecraft compare `DataVersion` avec le sien et refuse de charger (ou crash en chargeant certains chunks).
- Même en contournant le check de version, les blocs spécifiques à 1.21 apparaîtraient comme blocs manquants/erreurs dans le client plus ancien.

Donc : **mettre à jour un monde vers une version plus récente est permanent**. Le seul rollback sûr est de restaurer depuis un backup *avant* la mise à jour.

## Les mondes moddés rendent ça pire

Le DataFixer du Minecraft vanilla est au moins exhaustif et bien testé. Les saves moddées ajoutent une couche de risque :

- Les mods retirés laissent des erreurs **bloc manquant** et **entité manquante**. Le monde charge mais les cubes qui étaient des blocs mod deviennent des placeholders "?".
- Les mods remplacés (ancienne → nouvelle version) changent parfois les IDs de blocs ou les clés NBT. La migration est à la charge de l'auteur du mod et n'est pas toujours fluide.
- Les sauts de version majeurs dans un modpack (Forge 1.20.1 → 1.21.x par exemple) coïncident souvent avec la plupart des mods migrant vers des APIs entièrement nouvelles. Des mondes qui marchaient sous l'ancienne version peuvent avoir un comportement non défini sous la nouvelle.

Pour les instances moddées, traite tout saut de version comme un potentiel événement de corruption et fais un backup avant.

## Sauvegarder un monde correctement

Le backup le plus simple est une copie de dossier. Dans GDLauncher :

1. Clic droit sur l'instance → **Open Folder**.
2. Ouvre `instance/saves/`.
3. Copie le dossier nommé comme ton monde (même nom que dans la liste des mondes) quelque part en dehors de l'instance. Un autre disque, un dossier `~/Documents/mc-backups/`, n'importe où qui ne sera pas écrasé.

Cette copie est un snapshot du monde au moment où tu l'as copiée. Garde-la jusqu'à être certain que la nouvelle version marche.

Pour des backups continus, des outils tiers comme FTBBackups (un mod) prennent des snapshots en jeu à intervalles. Ils écrivent dans `backups/` dans l'instance et sont restaurables par snapshot.

## Ce que signifient les avertissements "version snapshot"

Si tu ouvres par mégarde un monde sauvegardé dans une snapshot Minecraft (build de développement, comme `24w11a`), le jeu officiel affiche un avertissement supplémentaire parce que les Data Versions des snapshots sont parfois en avance sur n'importe quelle version stable. Un monde issu d'une snapshot peut être inouvrable dans la prochaine version stable si la snapshot apportait des changements de format roulés en arrière avant le release. La voie sûre : ne pas jouer des mondes importants sur des snapshots, ou accepter que le monde est verrouillé sur la snapshot.

## TL;DR

- Les mises à jour de monde sont à sens unique ; backup avant ouverture dans une version plus récente.
- Les mondes moddés sont plus fragiles ; tout saut de version est un potentiel événement de corruption.
- Pour les mises à jour de modpack qui bumpent la version Minecraft, copie tout le dossier saves d'abord, puis mets à jour.
