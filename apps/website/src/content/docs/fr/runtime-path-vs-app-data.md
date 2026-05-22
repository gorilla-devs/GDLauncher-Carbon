---
title: "Runtime Path vs App Data Path"
description: "GDLauncher utilise deux chemins distincts pour ses données. Ce qu'ils contiennent, pourquoi ils sont séparés, et lequel tu veux généralement déplacer."
faq:
  - question: "Quelle est la différence entre l'App Data Path et le Runtime Path ?"
    answer: "L'App Data Path est le répertoire standard par utilisateur, là où Electron met ses caches et le marqueur runtime_path_override. Le Runtime Path est l'endroit où le core de GDLauncher range tout ce qui est volumineux : instances, assets et bibliothèques Minecraft, installations Java, base de données du launcher, et logs globaux. Par défaut, le Runtime Path est à l'intérieur de l'App Data Path, mais c'est le Runtime Path qui est fait pour être déplacé."
  - question: "Où est l'App Data Path sur mon OS ?"
    answer: "Windows : C:\\Users\\<toi>\\AppData\\Roaming\\gdlauncher_carbon. macOS : /Users/<toi>/Library/Application Support/gdlauncher_carbon. Linux : $XDG_DATA_HOME/gdlauncher_carbon, ou ~/.local/share/gdlauncher_carbon si XDG n'est pas défini."
  - question: "Faut-il déplacer le Runtime Path ou l'App Data Path ?"
    answer: "Le Runtime Path. C'est celui qui grossit à chaque instance installée. L'App Data Path reste petit et lié au profil utilisateur de l'OS. GDLauncher expose le déplacement dans Settings → Runtime Path ; l'App Data Path est géré par Electron et non déplaçable depuis l'UI."
  - question: "À quoi sert le fichier runtime_path_override ?"
    answer: "Après un changement de Runtime Path, GDLauncher écrit un petit fichier texte runtime_path_override à l'intérieur de l'App Data Path. Il contient le nouveau Runtime Path ; à chaque démarrage le launcher le lit pour savoir où vivent tes données. S'il manque, le launcher retombe sur le Runtime Path par défaut."
  - question: "Puis-je partager le Runtime Path entre deux ordinateurs ?"
    answer: "Non. La base de données du launcher stocke un état machine-local (chemins, tokens, Java) et n'est pas conçue pour un usage simultané. Pour les mêmes instances sur deux PC, copie-les à la main ou utilise Cloud Instance Share."
---

# Runtime Path vs App Data Path

## Deux chemins, deux rôles

GDLauncher répartit ses fichiers à deux endroits : un **App Data Path** pour les petites choses côté Electron, et un **Runtime Path** pour le gros (instances, assets, Java, base de données). Le Runtime Path est celui qu'on a parfois envie de déplacer ; l'App Data Path, on ne le touche presque jamais.

### App Data Path

C'est le dossier app standard par utilisateur de l'OS, là où pointe `userData` d'Electron. GDLauncher s'en sert pour :

- Le marqueur `runtime_path_override`, un fichier texte d'une ligne qui dit au launcher où le Runtime Path se trouve réellement
- Le Runtime Path par défaut, dans un sous-dossier `data/`, si tu ne l'as pas déplacé
- Les caches Chromium d'Electron (Network/, GPUCache/, Cookies, etc.)
- Les logs du processus principal Electron

Emplacement OS standard :

- **Windows :** `C:\Users\<toi>\AppData\Roaming\gdlauncher_carbon`
- **macOS :** `/Users/<toi>/Library/Application Support/gdlauncher_carbon`
- **Linux :** `$XDG_DATA_HOME/gdlauncher_carbon`, ou `~/.local/share/gdlauncher_carbon` si non défini

Sans le sous-dossier `data/`, c'est généralement petit. GDLauncher n'expose aucun réglage pour déplacer ce répertoire ; les conventions de l'OS et Electron s'attendent à le trouver là.

### Runtime Path (Core Module)

L'endroit où le core Rust de GDLauncher pose tout le reste :

- Tes instances (sous `instances/`)
- Les assets partagés de Minecraft (textures, sons fournis par Mojang)
- Les bibliothèques partagées (JARs de Mojang et des mod loaders)
- Les Java téléchargés par GDLauncher
- La base de données du launcher, `gdl_conf.db`
- Les logs globaux dans `__gdl_logs__/`

Par défaut à `<App Data Path>/data/`, donc à l'intérieur du dossier App Data. Tu peux le déplacer où tu veux via **Settings → Runtime Path**. C'est le chemin qui grossit ; quelques gros modpacks suffisent à dépasser les 50 Go.

## Quand déplacer le Runtime Path

Deux raisons classiques :

1. **Ton SSD est plein.** Déplacer les instances vers un HDD ou un second SSD plus grand.
2. **Tu veux des backups séparés du profil OS.** Le poser sur un disque que tu sauvegardes indépendamment est très bien ; juste ne pas le synchroniser activement pendant que tu joues, le launcher et l'outil de sync se battront pour les handles.

Pour un usage normal, pas besoin de déplacer. L'emplacement par défaut convient.

## Comment se passe le déplacement

Ouvre **Settings → Runtime Path**. Tape le nouvel emplacement, ou utilise l'icône de dossier pour parcourir. Le bouton d'application (l'icône en forme de flèche circulaire à droite de la ligne) s'allume dès que le chemin diffère du courant et qu'il est valide. Le clic ouvre un modal de confirmation qui montre l'ancien et le nouveau chemin.

Si le dossier cible est vide (ou n'existe pas encore), confirmer lance une migration complète : un overlay affiche le scan, puis la copie fichier par fichier, puis la suppression fichier par fichier de la source. Ne ferme ni l'app ni la machine tant que ça tourne. À la fin le launcher redémarre tout seul.

Si la cible contient déjà un Runtime Path GDLauncher (tu l'as déjà déplacé et tu veux qu'une nouvelle installation pointe sur les mêmes données), le modal te prévient en jaune que le dossier n'est pas vide. Confirmer fait un "switch only" : le marqueur est réécrit pour pointer vers les données existantes, aucun fichier n'est copié, et le launcher redémarre. Les données à l'ancien emplacement deviennent orphelines, tu peux les supprimer à la main.

Si une migration échoue en cours de route, l'overlay devient rouge et affiche l'erreur. Le launcher fait un rollback : les fichiers créés dans le nouveau chemin sont supprimés et le marqueur reste pointé sur l'ancien chemin, tu peux donc réessayer sans perdre de données. Les deux causes courantes sont les permissions d'écriture manquantes sur le disque cible et le manque d'espace libre.

### Le marqueur runtime_path_override

Quand tu changes le Runtime Path, GDLauncher écrit un petit fichier texte nommé `runtime_path_override` à l'intérieur de l'**App Data Path** (et non du Runtime Path). Son contenu est le nouveau Runtime Path, en texte brut. À chaque démarrage l'app lit le fichier pour savoir où vivent tes données.

Si tu supprimes le marqueur, GDLauncher retombe sur son Runtime Path par défaut (`<App Data>/data/`). Tes données ne sont pas perdues, elles restent là où tu les as déplacées, mais le launcher ne les voit pas tant que tu ne vas pas dans **Settings → Runtime Path** repointer vers ce dossier. Comme le dossier contient déjà des données GDLauncher, le launcher considère que c'est un "switch only" et met juste le marqueur à jour sans rien copier.

## Pourquoi ne pas partager la base

Le fichier `gdl_conf.db` dans le Runtime Path contient les tokens de comptes, refresh tokens Microsoft, état du compte GDL, et métadonnées des instances. Machine-local et sensible : **ne le partage avec personne.** Deux PC sur la même base se battront pour rafraîchir les tokens et se déconnecteront tous les deux.

Pour les mêmes instances sur un second PC, copie manuellement les dossiers d'instance depuis `instances/`, ou utilise la fonctionnalité Cloud Instance Share, conçue exactement pour ça.
