---
title: "Mod loaders comparés : Forge, NeoForge, Fabric, Quilt"
description: "GDLauncher supporte quatre mod loaders Minecraft. Ce que chacun est, leurs différences, et lequel choisir pour un mod ou modpack donné."
faq:
  - question: "Quel mod loader choisir pour Minecraft ?"
    answer: "Celui que ton mod ou modpack exige, en pratique ça décide presque toujours. Si tu pars de zéro : Fabric pour les mods perf/QoL sur les versions récentes, NeoForge pour les gros mods de contenu modernes, Forge pour les vieux modpacks et la plus grande bibliothèque historique."
  - question: "Les mods Forge marchent-ils sur Fabric ?"
    answer: "Non. Forge et Fabric sont incompatibles. Un mod écrit pour l'un ne se chargera pas sur l'autre. Beaucoup de mods populaires proposent des builds séparés Forge et Fabric ; vérifie la page du mod pour voir les loaders et versions supportés."
  - question: "NeoForge remplace-t-il Forge ?"
    answer: "En pratique oui pour les nouvelles versions Minecraft. NeoForge a démarré en 2023 comme fork de Forge avec la même API ; les deux ont divergé depuis, donc un mod actuel publie en général un build NeoForge séparé plutôt que de tourner sur les deux. À partir de 1.20.4, beaucoup de mods Forge sont désormais buildés pour NeoForge. Pour 1.20.1 et avant, Forge reste le standard."
  - question: "Les mods Fabric tournent-ils sur Quilt ?"
    answer: "La plupart oui. Quilt est un fork de Fabric et exécute les mods Fabric directement. Quelques mods Quilt-only utilisent les APIs Quilt et ne tournent pas sur Fabric. Si tu as une liste de mods tous Fabric, n'importe quel loader des deux marche."
  - question: "Peut-on faire tourner deux mod loaders en même temps ?"
    answer: "Pas dans la même instance. Chaque instance pioche un seul loader. Pour les deux, crée deux instances. Le système d'instances de GDLauncher est fait exactement pour ça : une instance Forge, une instance Fabric, bascule en un clic."
---

# Mod loaders comparés : Forge, NeoForge, Fabric, Quilt

## Les quatre mod loaders supportés par GDLauncher

GDLauncher peut installer et lancer chacun des quatre grands mod loaders pour Minecraft Java Edition, plus vanilla (sans loader). Quand tu crées une instance custom, tu en choisis un. Quand tu installes un modpack, le loader est celui que le manifest du pack indique.

### Forge

Le mod loader original, démarré en 2011. Forge a la plus grosse bibliothèque historique de mods, particulièrement les mods de contenu lourd (tech trees, systèmes de magie, nouveaux mondes, comme Tinkers' Construct, Twilight Forest, Create dans ses versions anciennes). C'est aussi celui que ciblent la plupart des vieux modpacks.

Forge se met à jour plus lentement que Fabric. Les nouvelles versions Minecraft voient souvent un release Forge des semaines, voire des mois plus tard.

### NeoForge

Un fork 2023 de Forge, né d'une scission dans la communauté Forge. NeoForge garde le style d'API de Forge (les mods sont en général source-compatibles) mais sort plus vite, et beaucoup du développement Forge a migré vers lui.

Sur Minecraft 1.20.4 et plus récent, NeoForge est le plus actif des deux. Beaucoup de gros mods publient désormais des builds NeoForge à parité avec Forge ou à la place de Forge.

### Fabric

Autre philosophie de conception : petit, rapide, modulaire. Fabric arrive presque le jour d'une nouvelle version Minecraft, parfois en quelques heures. Son écosystème mod penche vers la performance (Sodium, Lithium, FerriteCore), la QoL (Mod Menu, Iris) et les mods de contenu modernes de haute qualité.

Si la performance est la priorité, ou si tu joues sur une version Minecraft bleeding-edge, Fabric est le loader que tu veux.

### Quilt

Un fork 2022 de Fabric avec un modèle de gouvernance différent et quelques APIs supplémentaires. Quilt exécute les mods Fabric directement, la différence pratique est petite : Quilt si un mod spécifique l'exige, sinon Fabric donne le même résultat.

Quilt a un écosystème dédié plus petit que Fabric mais est presque totalement compatible avec le contenu Fabric.

## Matrice de compatibilité

| Mod construit pour | Tourne sur Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Oui | Parfois (les premiers NeoForge pouvaient lancer des mods Forge sans modifications parce que c'était un fork récent ; les APIs ont divergé depuis, et la plupart des mods Forge actuels ont besoin d'un build NeoForge) | Non | Non |
| NeoForge | Non | Oui | Non | Non |
| Fabric | Non | Non | Oui | Oui |
| Quilt | Non | Non | Mods Quilt-API : non ; reste : oui | Oui |

Il n'existe pas de bridge cross-loader en production. Les JARs que tu mets dans `mods/` doivent correspondre au loader de l'instance.

## Choisir pour une nouvelle instance

D'habitude, ce sont les mods ou modpack qui choisissent pour toi :

- **Tu installes un modpack depuis CurseForge ou Modrinth ?** GDLauncher lit le manifest et installe le loader indiqué. Pas le choix.
- **Tu construis une instance custom autour d'un mod précis ?** Regarde la page du mod. S'il dit "Fabric 1.21.x", tu crées une instance Fabric 1.21.x.
- **Tu construis une instance custom autour d'une liste de mods ?** Trouve le loader que la plupart supportent. Liste pour chaque mod ses loaders, prends l'intersection. La majorité des mods perf sont Fabric-only ; la majorité des gros mods de contenu sont Forge/NeoForge.

Sans contrainte particulière, recommandation : **Fabric** pour un setup orienté performance/visuel, **NeoForge** pour de la survie modded à contenu lourd.

## Changer de loader sur une instance existante

GDLauncher permet de changer le mod loader d'une instance après création, voir [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). Bref : clic droit sur l'instance → Edit → choisis un autre loader. Le dossier mods n'est pas vidé, donc les JARs de l'ancien loader restent ; retire à la main les incompatibles avant de lancer.

## Note sur les versions de loader

Chaque loader a sa propre version, indépendante de Minecraft. Quand tu choisis "Forge", tu choisis aussi une version Forge (du genre `47.2.0` pour Minecraft 1.20.1). Pour les mods, la version de loader importe rarement au-delà de "même majeure que ce qu'attend le pack", mais certains exigent un build minimum. La page CurseForge ou Modrinth du mod le précise.
