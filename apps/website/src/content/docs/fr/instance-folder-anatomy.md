---
title: "Anatomie d'un dossier d'instance"
description: "Ce qu'il y a dans un dossier d'instance GDLauncher. Où vivent les mods, mondes, configs, screenshots et logs, et ce qu'on peut supprimer à la main sans risque."
faq:
  - question: "Où sont sauvegardés mes mondes Minecraft ?"
    answer: "Chaque monde est dans <runtime_path>/instances/<instance>/instance/saves/<nom-monde>/. Le dossier saves/ est créé la première fois que tu génères ou importes un monde. Clic droit sur l'instance → Open Folder, puis dossier instance/saves. Pour un backup, copie le dossier du monde."
  - question: "Où sont les crash reports ?"
    answer: "Dans l'instance : instance/crash-reports/. Le dossier n'existe que si Minecraft a vraiment crashé ; si tu n'as jamais vu le jeu mourir salement, il n'est pas encore là. Chaque report est un fichier texte horodaté (crash-<date>-server.txt etc.)."
  - question: "Où vont les JARs de mods ?"
    answer: "instance/mods/ dans le dossier d'instance. Y déposer un JAR à la main fonctionne, mais contourne le tracking de GDLauncher, préfère l'onglet Addons sauf raison particulière. Les instances modpack verrouillées bloquent le bouton Add du tableau Addons, mais une copie manuelle de fichier passe."
  - question: "Quelle différence entre les logs du launcher et ceux de Minecraft ?"
    answer: "Deux jeux distincts. Les logs de session du launcher sont à <instance>/logs/. Les logs de jeu Minecraft (latest.log, crash-reports, tout ce que le jeu écrit) sont un niveau plus bas, à <instance>/instance/logs/ et <instance>/instance/crash-reports/."
  - question: "C'est quoi instance.json et packinfo.json ?"
    answer: "Des fichiers de métadonnées que GDLauncher écrit au niveau racine de chaque instance. instance.json contient le nom, l'icône, le modloader, la version Minecraft, le dernier moment joué et le temps de jeu total. packinfo.json (seulement présent pour les instances appairées à un modpack CurseForge ou Modrinth) trace quels fichiers appartiennent au pack pour distinguer mods gérés par le pack et mods ajoutés à la main. Ne les supprime pas à la main."
---

# Anatomie d'un dossier d'instance

## Où vit le dossier d'instance

Chaque instance GDLauncher est un sous-dossier du Runtime Path :

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← le dossier d'instance
```

`<shortpath>` est une version assainie du nom d'affichage. Clic droit sur l'instance dans GDLauncher → **Open Folder** t'y emmène directement.

## Ce qu'il y a dedans

Le dossier d'instance se divise en quelques éléments que GDLauncher trace au niveau racine, plus un sous-dossier `instance/` qui est le vrai répertoire de jeu Minecraft. Certains éléments sont toujours présents ; beaucoup sont créés à la demande, la première fois que quelque chose y écrit.

```
<shortpath>/
├── instance.json          ← métadonnées GDLauncher de l'instance (toujours présent)
├── packinfo.json          ← info de pairing modpack (seulement pour les modpacks appairés)
├── icon.png | icon.webp   ← icône custom (seulement si tu en as mis une)
├── logs/                  ← logs du launcher GDLauncher par instance
└── instance/              ← dossier de jeu de Minecraft
    ├── mods/              ← JARs de mods
    ├── config/            ← configs de mods
    ├── shaderpacks/       ← shader packs (si tu en as installé)
    ├── options.txt        ← réglages du client Minecraft (après le premier lancement)
    ├── logs/              ← logs de session Minecraft (latest.log etc.)
    ├── saves/             ← mondes (créé quand tu en génères un)
    ├── screenshots/       ← screenshots F2 (créé au premier F2)
    ├── crash-reports/     ← crash dumps (uniquement quand le jeu a crashé)
    ├── resourcepacks/     ← resource packs perso (quand tu en ajoutes un)
    ├── datapacks/         ← data packs globaux (par monde sous saves/<monde>/datapacks/)
    └── (spécifique au pack) ← kubejs/, defaultconfigs/, packmenu/, etc. uniquement si le modpack les apporte
```

Ne sois pas surpris si certains de ces dossiers manquent dans une instance fraîche. Le launcher et Minecraft ne créent que ce dont ils ont besoin, quand ils en ont besoin. Une instance jamais jouée n'a pas de `saves/`, pas de `screenshots/`, pas d'`options.txt`. Une instance vanilla n'a pas de `mods/` ni de `config/`.

## Ce que chaque chose contient

### Fichiers racine

- **`instance.json`** : métadonnées GDLauncher, nom, chemin d'icône, modloader et version, version Minecraft, date de création, dernier jeu, temps de jeu total. Toujours présent.
- **`packinfo.json`** : manifest de hashes des fichiers venant du modpack. Permet au launcher de distinguer les mods gérés par le pack de ceux que tu as ajoutés. Seulement présent pour les instances appairées à un modpack CurseForge ou Modrinth.
- **`icon.png`** ou **`icon.webp`** : l'icône custom que tu as uploadée. Absente si tu utilises celle par défaut.

### `logs/` (racine)

Les logs propres à GDLauncher pour cette instance. C'est ce que montre **View Logs** du menu contextuel. Ils couvrent le lancement depuis le point de vue du *launcher* (arguments Java, téléchargements d'assets, install du mod loader, code de sortie), utile quand le jeu ne va jamais assez loin pour écrire son propre log.

### `instance/mods/`

Les fichiers JAR des mods. Minecraft charge tout ce qui y est au démarrage (selon les règles du mod loader). Le launcher trace dans sa base de données quels mods appartiennent à une instance (clé par nom de fichier et hash du contenu), pas de fichiers sidecar. Les JARs déposés à la main sont également repérés, le launcher n'a juste pas de métadonnées CurseForge/Modrinth pour eux.

### `instance/config/`

Un sous-dossier ou fichier par mod. Où persistent les réglages des mods. La plupart des mods écrivent `config/<modid>.toml` ou un dossier `config/<modid>/`. L'édition manuelle est en général sûre ; la plupart des mods relisent les changements au redémarrage.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Asset packs. Resource packs pour textures et sons, shader packs pour le rendu (nécessitent Iris/OptiFine installé comme mod), data packs pour recettes/loot/functions. Les data packs propres à un monde vivent sous `saves/<monde>/datapacks/`. Ces dossiers ne sont créés que lorsque tu as réellement du contenu pour eux.

### `instance/saves/`

Un sous-dossier par monde. À l'intérieur : `level.dat` (fichier maître), `region/` (données chunks), `playerdata/` (état par joueur), `datapacks/` (data packs scope monde). Pour backuper, copie tout le dossier `<monde>/`. `saves/` lui-même apparaît la première fois que tu génères un monde.

### `instance/screenshots/`

Tout ce que tu as capturé avec F2 en jeu. PNGs nommés par timestamp. Créé au premier screenshot.

### `instance/logs/` et `instance/crash-reports/`

La sortie diagnostique de Minecraft. `logs/latest.log` est toujours le lancement le plus récent (roté en `logs/<date>-1.log.gz` au lancement suivant). `crash-reports/` contient les dumps complets quand le jeu meurt salement, et n'apparaît qu'une fois qu'il y a eu un vrai crash.

### `instance/options.txt`

Réglages du client Minecraft (graphismes, contrôles, sons). Texte brut, key=value. Éditable si tu y tiens.

### Dossiers spécifiques aux modpacks

Beaucoup de gros modpacks apportent des dossiers supplémentaires. Les plus courants :

- **`kubejs/`** : scripts KubeJS (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Les auteurs de pack s'en servent pour les ajustements runtime.
- **`defaultconfigs/`** : snapshot de "ce à quoi les configs devraient ressembler par défaut". Le script de lancement du pack copie les entrées manquantes dans `config/` à chaque démarrage.
- **`packmenu/`** : assets de menu principal aux couleurs du pack (boutons custom, fonds, splash text).
- **`defaultsettings/`** : similaire à `defaultconfigs/`, mais pour `options.txt` et les keybinds.

Existent uniquement si le pack les apporte. Vanilla et la plupart des instances custom n'en ont aucun.

## Ce qu'on peut supprimer

| Dossier | Suppression sûre ? | Effet |
|---|---|---|
| `instance/mods/` (un JAR spécifique) | Oui | Le mod est parti. Les mondes qui l'utilisent peuvent casser. |
| `instance/config/<modid>/` | Oui | Le mod revient aux défauts au prochain lancement. |
| Contenus de `instance/resourcepacks/`, `instance/shaderpacks/` | Oui | Le pack est parti. |
| `instance/saves/<monde>/` | Oui | Monde supprimé définitivement. Backup avant. |
| `instance/logs/`, `crash-reports/` | Oui | Libère juste de la place. |
| `instance/screenshots/` | Oui | Adieu vieux screenshots. |
| `logs/` (logs launcher) | Oui | Idem. |
| `instance/options.txt` | Oui | Réglages du jeu remis aux défauts. |
| `instance.json` | Ne pas faire | Le launcher perd la trace de l'instance. |
| `packinfo.json` | Possible (équivaut à dépairer) | Le launcher arrête de traiter l'instance comme un modpack appairé. Même effet que le bouton Unpair dans l'onglet Settings de l'instance, en plus salissant. |
| Tout le dossier `instance/` | Ne pas faire | L'instance devient corrompue. Utilise Delete dans l'UI. |

## Note sur les instances modpack verrouillées

Le dossier d'instance est juste un dossier ordinaire sur le disque ; le verrou que GDLauncher applique aux instances modpack est appliqué dans l'UI, pas dans le système de fichiers. Déposer un JAR dans `instance/mods/` d'une instance verrouillée fonctionne mécaniquement, le launcher n'en saura juste rien via l'onglet Addons. Pour le retirer il faudra repasser par le système de fichiers. Pour un workflow plus propre, ouvre l'instance, va dans l'onglet **Settings** et clique **Unlock** dans la section Modpack.
