---
title: "Mémoire Java et Garbage Collection"
description: "Comment Minecraft utilise la RAM, pourquoi en allouer plus n'est pas toujours plus rapide, ce que font réellement le heap, les pauses GC et les Aikar's flags, et quand laisser les valeurs par défaut tranquilles."
faq:
  - question: "Combien de RAM allouer à Minecraft ?"
    answer: "Pour le vanilla : 2-4 Go suffisent. Pour une petite instance moddée (20-40 mods) : 4-6 Go. Pour un gros modpack (100+ mods, style ATM) : 6-8 Go. Au-dessus de 8 Go ça aide rarement, le goulot bascule vers le garbage collector, et un heap énorme veut dire des pauses GC plus longues et un jeu plus saccadé."
  - question: "Plus de RAM rendra-t-il Minecraft plus rapide ?"
    answer: "Jusqu'à un point, ensuite non. Une fois que Minecraft a assez de heap pour tenir le monde et tous les mods sans collecte constante, ajouter de la mémoire ne fait qu'agrandir ce que le GC doit scanner quand il tourne, donc pauses plus longues, gameplay plus saccadé. Le bon réglage c'est 'juste assez', pas 'autant que possible'."
  - question: "C'est quoi les Aikar's flags et faut-il les utiliser ?"
    answer: "Les Aikar's flags sont un jeu d'arguments JVM qui règlent le garbage collector G1 pour favoriser de courtes pauses plutôt que le débit, à l'origine pour serveurs Minecraft. Ils aident sur serveurs et clients moddés lourds. GDLauncher ne les applique pas automatiquement ; tu peux les coller dans Java Arguments dans les Settings de l'instance. Pas magiques, pas toujours plus rapides."
  - question: "Pourquoi Minecraft lague toutes les quelques secondes même avec 16 Go alloués ?"
    answer: "Presque toujours du stutter de pause GC, pas un manque de mémoire. Le GC tourne moins souvent avec plus de heap, mais quand il tourne sur un gros heap, c'est plus long. Paradoxalement, baisse l'allocation, ou passe à un modpack plus petit."
  - question: "Quelle différence entre Xmx et Xms ?"
    answer: "Xmx est la taille max du heap (le plafond). Xms est la taille initiale (point de départ). Le slider RAM de GDLauncher règle Xmx ; Xms est mis à une valeur raisonnable automatiquement. Pour Minecraft, mettre Xmx égal à Xms n'aide pas vraiment, la JVM grossit le heap selon les besoins dans la limite de Xmx."
---

# Mémoire Java et Garbage Collection

## Comment Minecraft utilise la mémoire

Minecraft est un programme Java. Comme tous les programmes Java, il tourne dans une Java Virtual Machine (JVM) qui reçoit une tranche fixe de RAM système. Tout ce que fait Minecraft, chunks chargés, entités, état des mods, textures, vit dans cette tranche.

Quand tu règles **Instance Java Memory** dans les paramètres d'instance de GDLauncher, tu règles `-Xmx`, la taille max de heap que la JVM peut utiliser. Le code Java lui-même (allocations d'objets, structures de données des mods, état du monde) vit dans ce heap. Les textures et buffers OpenGL vivent hors du heap, en mémoire native, et ne sont pas affectés par Xmx.

## Le vrai goulot c'est le garbage collector

Java ne libère pas la mémoire à la main ; il a un **garbage collector** qui scanne périodiquement le heap, trouve les objets que rien ne référence plus, et les récupère. Le Minecraft moderne utilise par défaut le collecteur **G1**.

Le GC tourne en deux modes :

- **Young GC.** Court, fréquent. Scanne une petite "young generation" d'objets récents. Quelques millisecondes en général.
- **Old GC / Mixed GC.** Long, moins fréquent. Scanne le reste du heap. Peut prendre des dizaines de millisecondes voire plus sur un gros heap.

Quand le GC tourne, **Minecraft est en pause**. Plus le heap est gros, plus ces grandes collectes sont longues. C'est pour ça qu'ajouter de la RAM au-delà du besoin réel *empire* le stutter de pauses, pas l'inverse.

C'est la chose la plus contre-intuitive du tuning mémoire Java : **allouer moins peut être plus fluide qu'allouer plus**.

## La bonne quantité de RAM

Repères approximatifs, basés sur ce qui rentre dans le heap actif sans thrash :

| Charge | Xmx recommandé |
|---|---|
| Minecraft vanilla | 2-4 Go |
| Mods légers (20-40, style Sodium) | 4 Go |
| Modpack moyen (80-120 mods) | 4-6 Go |
| Gros modpack (ATM, FTB Continents, 250+ mods) | 6-8 Go |
| Modpacks "kitchen sink" (500+ mods, prégen de chunks profonde) | 8-10 Go |

Au-dessus de 10 Go, c'est rarement utile sauf si la doc du pack le dit explicitement. Certains packs demandent vraiment plus (combos de mods gourmands en mémoire comme Better End plus NetherEx) ; suis la recommandation du pack.

## Comment marche le slider de GDLauncher

Ouvre une instance, clique l'onglet Settings, descends jusqu'à **Instance Java Memory**. Bascule le toggle pour activer l'override par instance, puis tire le slider ; il va de 1 Go jusqu'au total de RAM système (avec un avertissement au-delà de 80 %). Le launcher convertit la valeur en `-Xmx<n>M` et la passe à la JVM.

Le même slider existe au niveau global dans **Settings → Java → Java Memory**, comme défaut pour toute instance qui n'override pas. Mets le global bas pour le casual, ne booste que les gros modpacks.

## Les Aikar's flags

Longue série d'arguments JVM qui tunent G1 pour privilégier les courtes pauses au débit. À l'origine écrits pour serveurs Minecraft, utiles aussi sur clients moddés. Ça ressemble à :

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

GDLauncher ne les applique pas par défaut. Pour les utiliser, colle la chaîne complète dans le champ **Instance Java Arguments** dans l'onglet Settings de l'instance (ou dans **Settings → Java → Java Arguments** pour appliquer globalement). Les effets varient ; le bénéfice le plus régulier est moins de longues pauses sur des heaps de 6 à 10 Go.

Quelques précautions :

- Tunés pour des versions anciennes de Java. Sur Java 17+ les défauts sont déjà bons et les gains d'Aikar plus faibles.
- Supposent des motifs d'allocation de serveur. Sur un desktop avec petit heap, ça peut nuire.
- N'ajoutent pas de mémoire et ne changent pas ce que le jeu utilise ; ils changent juste le comportement du collecteur.

Sans raison précise d'y toucher, laisse le champ Java Arguments tel que GDLauncher l'a réglé.

## Diagnostiquer le stutter

Si Minecraft fait des pauses de plusieurs centaines de millisecondes toutes les quelques secondes :

1. Ouvre l'écran de debug F3. Regarde la ligne "Mem:" (en haut à droite). Si elle saute rapidement entre bas et haut, c'est du churn GC.
2. Baisse Xmx de 1-2 Go et réessaie. Contre-intuitif, mais les heaps plus petits sont GC plus vite.
3. Si un mod précis alloue comme un dingue (certains mods de prégénération ou de rendu le font), ça apparaîtra dans les profilers côté mod (Spark, JmxMC). Le mod a peut-être besoin d'une mise à jour.
4. Si ton CPU est à 100% pendant les pauses, le GC travaille vraiment dur. Baisse Xmx encore ou retire les mods gourmands.

## TL;DR

- Mets Java Memory à *juste assez*, pas autant que tu as.
- Le goulot est le temps de pause GC, pas la taille brute du heap.
- Les Aikar's flags aident sur les gros heaps mais ne sont pas une solution magique.
- 4-6 Go est correct pour presque tout en modded.
