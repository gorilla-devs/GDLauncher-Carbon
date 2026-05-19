---
title: "Format du manifest d'un modpack"
description: "Ce qu'il y a dans les trois archives de modpacks que GDLauncher lit et écrit : CurseForge .zip, Modrinth .mrpack et le .gdlpack de GDLauncher (basé sur les hashes). Référence champ par champ, comparatif côte à côte et comment chaque format résout les mods à l'installation."
faq:
  - question: "C'est quoi un manifest de modpack ?"
    answer: "Un fichier JSON à l'intérieur d'une archive de modpack qui dit au launcher ce que contient le pack : version Minecraft, mod loader, liste des mods et quels fichiers livrés (overrides) extraire dans l'instance. GDLauncher lit le manifest en tout premier quand il installe un pack."
  - question: "Quelle différence entre les formats CurseForge, Modrinth et GDLauncher ?"
    answer: "Les trois sont des zips contenant un manifest JSON. CurseForge utilise manifest.json avec des project/file IDs CurseForge. Modrinth utilise modrinth.index.json avec des URLs de téléchargement directes plus des hashes. GDLauncher utilise gdlpack.json avec uniquement des hashes de contenu (sha512 + sha1 + murmur2) et résout chaque fichier contre la plateforme qui le possède réellement. Ils ne sont pas interchangeables ; le launcher dispatche selon le JSON trouvé à la racine de l'archive."
  - question: "C'est quoi un .gdlpack et pourquoi ça existe ?"
    answer: "Le format de modpack propre à GDLauncher. Chaque mod est identifié par des hashes de contenu, donc un même pack fonctionne que les mods vivent sur CurseForge, Modrinth ou les deux, sans URL qui rouille. Il a aussi un support de première classe pour les fonctionnalités optionnelles (bundles de mods + configs qu'on peut sauter) et les icônes embarquées. Recommandé quand les deux côtés utilisent GDLauncher ; CurseForge .zip reste le choix le plus sûr pour la distribution cross-launcher."
  - question: "Où je trouve le manifest dans une instance installée ?"
    answer: "GDLauncher ne garde pas l'archive d'origine après l'installation. Le contenu du pack se retrouve décompressé dans le dossier de l'instance. Le manifest avec lequel GDLauncher travaille est suivi dans la base de données du launcher contre l'instance ; il n'est pas consultable comme un fichier. Pour inspecter un manifest avant d'installer, ouvre le .zip / .mrpack / .gdlpack avec n'importe quel outil d'archive et lis le fichier JSON à l'intérieur directement."
  - question: "Je peux éditer un manifest ?"
    answer: "Tu peux en éditer un dans une archive si tu construis ton propre pack, mais tu ne peux pas éditer le manifest d'une instance installée via l'UI du launcher. Pour changer les mods d'une instance, utilise l'onglet Addons (ou déverrouille d'abord une instance verrouillée). Pour changer la version d'un pack apparié, ouvre l'instance, va dans l'onglet Settings et clique sur Change Modpack Version."
  - question: "C'est quoi un 'override' dans le jargon des packs ?"
    answer: "Des fichiers que le pack livre pré-empaquetés, pas des mods. Configs par défaut, scripts, resource packs, parfois un monde de départ. Ils sont extraits par-dessus les mods dans le dossier de l'instance quand le pack s'installe. CurseForge les liste dans le champ overrides ; Modrinth marque les fichiers du pack avec env.client=required ; GDLPack utilise un dossier overrides (configurable via le manifest)."
---

# Format du manifest d'un modpack

## Pourquoi c'est utile

Normalement tu n'as pas besoin de savoir ce qu'il y a dans une archive de modpack. Mais quand quelque chose se passe mal pendant l'installation, tu vois des messages comme « manifest », « invalid manifest format », « manifest missing field » ou « manifest version unsupported », et tu veux savoir à quoi ils font référence. Cette page est aussi utile si tu construis un pack à partager ou si tu cherches pourquoi le même pack s'installe proprement dans un launcher et casse dans un autre.

GDLauncher lit et écrit trois formats :

- **CurseForge** (`.zip`, avec `manifest.json` dedans).
- **Modrinth** (`.mrpack`, avec `modrinth.index.json` dedans).
- **GDLauncher** (`.gdlpack`, avec `gdlpack.json` dedans).

Les trois sont des archives ZIP standards sous le capot ; l'extension indique juste quel schéma de manifest attendre.

## Ce qu'il y a dans une archive de modpack

La forme générale est la même pour les trois formats :

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← manifest spécifique au format
├── overrides/            ← configs, scripts, monde de départ optionnel
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (CurseForge uniquement, optionnel, liste de mods lisible)
```

Le manifest est le seul fichier dont le launcher a strictement besoin pour savoir installer le pack. Le dossier overrides est du contenu à décompresser dans l'instance résultante.

## CurseForge : `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

Champs clés :

- `minecraft.version` : la version de Minecraft visée par le pack.
- `minecraft.modLoaders` : quel loader et quelle version (format : `<loader>-<version>`).
- `files` : chaque mod est référencé par `projectID` et `fileID` CurseForge. GDLauncher les résout contre l'API CurseForge pour télécharger.
- `overrides` : nom du dossier dans le zip dont le contenu est copié dans l'instance une fois les téléchargements de mods terminés.

Si le `projectID` ou `fileID` d'un fichier n'existe plus sur CurseForge (l'auteur l'a retiré), l'installation échoue avec une erreur « file not found » pour ce mod précis.

## Modrinth : `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

Principales différences avec CurseForge :

- `dependencies` déclare les versions Minecraft et loader directement (pas d'objet imbriqué).
- Chaque entrée `files` inclut l'**URL de téléchargement exacte** et le **hash**, donc les installations ne dépendent pas d'un appel API de métadonnées.
- Chaque fichier déclare s'il est requis sur client, serveur ou les deux via `env`.

Ce format est plus simple et plus facile à installer hors-ligne (les URLs sont embarquées). Il est aussi plus strict : un mismatch de hash entraîne un refus dur d'installation, pas un avertissement.

## GDLauncher : `gdlpack.json`

Le format de pack propre à GDLauncher est ce que GDLauncher écrit quand tu choisis **GDLauncher .gdlpack** dans **Export Instance**. La propriété définissante : chaque mod est identifié uniquement par ses hashes de contenu, pas par un ID spécifique à une plateforme ou une URL. GDLauncher résout chaque hash contre CurseForge et Modrinth au moment de l'installation et télécharge depuis la plateforme qui possède le fichier.

### Pourquoi des hashes plutôt que des IDs ou URLs

- **Cross-plateforme depuis une source.** Un mod qui vit à la fois sur CurseForge et Modrinth a des IDs différents de chaque côté. Un `.gdlpack` s'en fiche ; une liste de hashes fonctionne pour les deux.
- **Pas de pourrissement d'URL.** Les URLs du CDN Modrinth sont adressées par contenu et stables, mais embarquer des URLs fige une seule origine de téléchargement. Les hashes permettent au launcher de basculer quand une plateforme est down.
- **Vérification par construction.** Chaque octet écrit dans ton dossier `mods/` est vérifié contre le hash du manifest. Aucun moyen de substituer silencieusement un autre JAR.

### Structure d'archive

```
mypack.gdlpack/
├── gdlpack.json          ← le manifest
├── .gdl/
│   └── icon.png          ← icône du pack embarquée (optionnel)
└── overrides/            ← fichiers livrés (configs, scripts, mods non résolvables)
    ├── config/
    └── ...
```

Le manifest est à la racine. Le dossier `.gdl/` contient les métadonnées que le launcher embarque (actuellement juste l'icône). Les overrides fonctionnent comme dans CurseForge : le contenu est extrait dans l'instance résultante après l'étape de résolution des mods.

### Manifest minimal

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

Seuls `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries` et `overrides` sont requis pour le chargement.

### Manifest complet, annoté

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### Champs de premier niveau

| Champ | Type | Requis | Signification |
|---|---|---|---|
| `formatVersion` | integer | oui | Version de schéma du manifest. Actuellement toujours `1`. |
| `name` | string | oui | Nom d'affichage du pack. Sert de nom d'instance par défaut à l'import. |
| `version` | string | non | Version du pack (semver recommandé, ex. `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | non | Tagline d'une ligne. |
| `author` | string | non | Auteur ou équipe du pack. |
| `createdAt` | timestamp RFC 3339 | oui | Date d'export de l'archive. |
| `icon` | string | non | Chemin dans l'archive vers le fichier d'icône (ex. `.gdl/icon.png`). |
| `dependencies` | object | oui | Exigences Minecraft et modloader (voir ci-dessous). |
| `entries` | array | oui | Fichiers plateforme et fonctionnalités optionnelles (voir ci-dessous). Un tableau vide est valide pour un pack purement overrides. |
| `overrides` | string | oui (défaut `"overrides"`) | Nom du dossier dans l'archive à extraire dans l'instance. |
| `serverOverrides` | string | non | Dossier d'overrides serveur uniquement. |
| `clientOverrides` | string | non | Dossier d'overrides client uniquement. |
| `source` | object | non | Si le pack est dérivé d'un pack CurseForge ou Modrinth, pointe vers l'original (voir ci-dessous). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft` : version Minecraft requise (ex. `1.20.1`, `1.21.4`, `25w20a` pour les snapshots).
- `modloaders` : zéro ou plus d'entrées de loader. Chaque entrée a :
  - `type` : un parmi `forge`, `neoforge`, `fabric`, `quilt` (en minuscules).
  - `version` : version exacte du loader contre laquelle le pack a été construit.
  - `primary` : marque le loader recommandé quand plusieurs sont listés (ex. un pack compatible Fabric et Quilt).

Un pack vanilla a un tableau `modloaders` vide.

#### `entries`

Deux types d'entrées, distingués par le champ `type` :

**`platform`**, un mod requis résolu via hash :

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

Les trois hashes doivent être présents. Chacun est utilisé différemment à la résolution :

| Hash | Utilisé pour |
|---|---|
| `sha512` | Résolution Modrinth (endpoint `version_file` de Modrinth), vérification d'intégrité principale du fichier téléchargé. |
| `sha1` | Résolution Modrinth (fallback), utilisé par Minecraft lui-même pour vérifier le fichier au lancement. |
| `murmur2` | Résolution CurseForge (endpoint `fingerprints` de CurseForge). |

À l'installation, le launcher essaie d'abord Modrinth (lookup sha512), bascule sur CurseForge (empreinte murmur2), et fait échouer l'entrée si aucune des deux plateformes n'a le fichier. Les fichiers résolus se téléchargent toujours depuis la plateforme qui a répondu.

**`optional`**, un groupe sautable de mods et/ou chemins d'overrides que l'utilisateur peut inclure ou exclure :

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description` : montré à l'utilisateur dans l'aperçu d'import pour qu'il décide d'inclure ou non cette fonctionnalité.
- `platforms` : zéro ou plus d'entrées hash téléchargées seulement si la fonctionnalité est incluse.
- `overridePaths` : zéro ou plus de chemins (fichiers ou dossiers) dans le dossier `overrides/` extraits seulement si la fonctionnalité est incluse. Les chemins sont relatifs à `overrides/`.

Une fonctionnalité peut n'avoir que des fichiers plateforme, que des overrides, ou les deux. Sert à grouper du contenu optionnel apparenté : un mod shader plus sa config, un preset de difficulté hardcore qui est juste des configs, un resource pack optionnel.

#### `source`

Quand un pack a été exporté depuis une instance de modpack CurseForge ou Modrinth existante, GDLauncher enregistre l'origine :

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

ou

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

C'est informatif ; le launcher ne s'en sert pas pour télécharger (le tableau `entries` couvre déjà ça). Ça existe pour que les dérivés de packs publics restent attribuables à l'original.

### Exports avec ou sans bundle

À l'export d'une instance en `.gdlpack`, le toggle **Bundle Addons** dans l'assistant d'export décide de ce qui va où :

- **Bundle off (manifest seul).** Chaque mod que GDLauncher peut résoudre via CurseForge ou Modrinth devient une entrée `platform` dans `entries`. Le JAR réel n'est *pas* inclus dans l'archive. Le launcher du destinataire le retélécharge depuis la plateforme à l'import. Les mods que GDLauncher ne peut pas résoudre (ex. des JARs déposés à la main) sont bundlés dans `overrides/mods/` en fallback. Résultat : archive petite, le destinataire a besoin d'internet à l'installation.
- **Bundle on (autonome).** Aucune entrée `platform` n'est émise ; chaque mod est copié dans `overrides/mods/`. Résultat : archive plus grande (souvent des centaines de Mo à plusieurs Go), installable hors-ligne une fois les assets Minecraft en cache.

Les resource packs et shader packs se comportent pareil : les résolvables deviennent des entrées `platform` avec bundle off, tout est bundlé directement avec bundle on.

## Côte à côte : les trois formats

| Propriété | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Nom du manifest | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Identification des mods | Project + File ID CurseForge | URL de téléchargement + hash | Hashes de contenu uniquement |
| Résout depuis | API CurseForge | URL embarquée dans le manifest | CurseForge **ou** Modrinth, celle qui répond |
| Installation hors-ligne | Non (CDN requis) | Oui (les URLs peuvent encore nécessiter le CDN) | Oui quand `Bundle Addons` est activé |
| Fonctionnalités optionnelles | Flag `required: false` par mod | `env` par fichier (client/server) | Entrée `optional` dédiée avec plusieurs fichiers |
| Dossier overrides | `overrides/` | chemin par fichier | `overrides/` (configurable) |
| Fichiers serveur uniquement | Pack séparé | `env.server` | Dossier `serverOverrides` |
| Icône | Externe | Externe | Embarquée dans `.gdl/icon.<ext>` |
| Suivi de source | Aucun | Aucun | Champ `source` |
| Portabilité | La plus large (la plupart des launchers lisent) | Launchers compatibles Modrinth | GDLauncher |

## Overrides, en détail

Quand le manifest référence un dossier overrides ou des fichiers de pack marqués client-required, le launcher les extrait dans l'instance après l'étape de résolution des mods. C'est comme ça que les packs livrent :

- Configs de mods par défaut (pour que le pack se joue pareil out-of-the-box pour tout le monde).
- Scripts KubeJS ou CraftTweaker.
- Un monde de départ (occasionnellement).
- Resource packs que le pack attend activés.
- `options.txt` avec des réglages de jeu adaptés au pack.

Les overrides gagnent sur les défauts auto-générés mais perdent face à tout ce que le joueur change plus tard manuellement. GDLPack supporte en plus les dossiers `serverOverrides` et `clientOverrides` pour des fichiers qui ne doivent atterrir que d'un côté, utile pour les packs censés livrer un bundle client et serveur assortis.

## Quand les manifests vieillissent

Un manifest de pack est un instantané de ce que l'auteur a publié en dernier. Si un mod référencé par le manifest est retiré de la plateforme source après que le manifest a été construit, l'installation de cette version de pack échoue jusqu'à ce que l'auteur en publie une nouvelle. C'est ce qui se passe quand une vieille version de pack s'installe proprement mais une plus récente casse : la plus récente référence quelque chose qui n'est plus disponible.

Le fix est du côté de l'auteur ; le launcher ne peut que faire ce que le manifest dit. GDLPack a un avantage partiel ici parce qu'il n'est pas lié à une seule plateforme : si un mod est retiré de CurseForge mais reste sur Modrinth (ou l'inverse), le même tableau `entries` résout encore.

## Lire un manifest soi-même

Pas besoin de spécial. Renomme `mypack.mrpack` ou `mypack.gdlpack` en `mypack.zip`, ouvre-le avec n'importe quel outil d'archive, et regarde le fichier JSON à l'intérieur (`modrinth.index.json` ou `gdlpack.json`). Pareil pour les `.zip` CurseForge et `manifest.json`. Les trois sont du JSON pur, lisible dans n'importe quel éditeur de texte.

## Construire un pack

Si tu construis un pack à partager, le chemin le plus simple est de configurer l'instance dans GDLauncher, puis d'utiliser **Export Instance** (clic droit → Export Instance). L'assistant d'export te laisse choisir le format cible : CurseForge `.zip`, Modrinth `.mrpack` ou le `.gdlpack` propre à GDLauncher. Choisis `.gdlpack` si le destinataire utilise aussi GDLauncher et que tu veux des fonctionnalités optionnelles ou des icônes embarquées ; choisis `.zip` pour la compatibilité cross-launcher la plus large.
