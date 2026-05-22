---
title: "Dépannage"
description: "Résous les problèmes courants de lancement de GDLauncher et Minecraft. Chemin des données, chemin de runtime, emplacement des logs et solutions éprouvées."
faq:
  - question: "Où GDLauncher stocke-t-il ses données ?"
    answer: "Sur Windows : C:\\Users\\<toi>\\AppData\\Roaming\\gdlauncher_carbon. Sur macOS : /Users/<toi>/Library/Application Support/gdlauncher_carbon. Sur Linux : $XDG_DATA_HOME/gdlauncher_carbon (ou ~/.local/share/gdlauncher_carbon si XDG n'est pas défini)."
  - question: "Où sont les logs de GDLauncher ?"
    answer: "GDLauncher écrit deux logs au niveau de l'app dans des fichiers séparés : main.log (Electron) dans le dossier App Data, et des fichiers <timestamp>.log dans le dossier __gdl_logs__ du chemin de runtime (Rust core ; les 10 plus récents sont gardés). Quand tu signales un problème, envoie les deux. Les chemins exacts sont dans le guide Share App Logs."
  - question: "GDLauncher ne s'ouvre pas. Que faire ?"
    answer: "Commence par vérifier les logs dans le dossier de données pour repérer une erreur. Causes fréquentes : runtime corrompu, antivirus qui bloque l'exécutable, ou mise à jour partiellement appliquée. Une réinstallation propre de GDLauncher avec restauration des instances règle généralement les deux."
  - question: "Pourquoi mon modpack plante-t-il au lancement ?"
    answer: "La plupart des plantages au lancement viennent d'une incompatibilité entre la version de Minecraft, le mod loader et les mods. Consulte le fichier le plus récent dans __gdl_logs__ pour repérer l'erreur. Si un mod précis est nommé, c'est généralement le coupable, désactive-le dans l'onglet Addons et relance. En cas d'OutOfMemoryError, augmente la RAM dans les paramètres de l'instance."
  - question: "Comment déplacer GDLauncher vers un autre disque ou dossier ?"
    answer: "Ouvre Paramètres → Général → Chemin du runtime. Modifie-le pour le nouvel emplacement et GDLauncher migrera automatiquement tes instances et téléchargements. La migration s'exécute une seule fois au prochain lancement."
  - question: "Puis-je utiliser GDLauncher hors ligne ?"
    answer: "Tu peux jouer hors ligne à des instances déjà installées. L'authentification nécessite tout de même une connexion en ligne au moins une fois (compte Microsoft), et le téléchargement de nouveaux mods ou modpacks requiert une connexion Internet."
---

## Chemin des données de l'application

C'est le chemin où GDLauncher stocke les données d'Electron, ainsi que par défaut le chemin de runtime du Core Module.

### Windows

`C:\Users\\{{Ton nom d'utilisateur}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Ton nom d'utilisateur}}/Library/Application Support/gdlauncher_carbon`

### Linux

- si la variable d'environnement `$XDG_DATA_HOME` est définie : `$XDG_DATA_HOME/gdlauncher_carbon`
- sinon : `{{homedir}}/.local/share/gdlauncher_carbon`

[Plus de détails sur homedir](https://nodejs.org/api/os.html#oshomedir)

## Chemin de runtime du Core Module

C'est le chemin où le Core Module stocke toutes ses données, y compris les instances, les assets et les bibliothèques.
Il se trouve généralement dans le même répertoire que le chemin des données, dans le sous-dossier `data`, à moins que tu ne définisses un autre emplacement.

### Base de données de l'application

La base de données se trouve dans le chemin de runtime du Core Module ; c'est un fichier SQLite nommé `gdl_conf.db`.

**N'ENVOIE CE FICHIER À PERSONNE, IL CONTIENT DES DONNÉES SENSIBLES.**

### Logs de l'application

GDLauncher écrit deux logs au niveau de l'app dans des fichiers séparés. Pour le support, **envoie toujours les deux**, les deux moitiés du launcher se transmettent du travail en permanence, et la cause d'une panne d'un côté apparaît souvent dans le log de l'autre côté.

- **`main.log`** dans l'App Data Path : le log du processus principal Electron. Couvre la création de fenêtre, IPC, l'auto-update, les dialogues natifs et les crashs durs du shell desktop.
- **`__gdl_logs__/<timestamp>.log`** dans le Core Module Runtime Path : le log du Rust core. Couvre l'authentification, les téléchargements d'assets, les installs de mod loaders, les lancements d'instance, les changements de settings. Les 10 plus récents sont gardés.

Chemins par OS et captures dans le guide [Share App Logs](/guides/share-app-logs).

**LES LOGS PEUVENT CONTENIR DES DONNÉES SENSIBLES. PRUDENCE QUAND TU LES PARTAGES.**

### Modifier le chemin de runtime

Si tu changes le chemin de runtime, l'app déplace automatiquement toutes tes instances et fichiers de configuration vers le nouvel emplacement.

Si le dossier cible est déjà utilisé, l'app se contente de mettre à jour la configuration du chemin de runtime ; aucun fichier n'est déplacé ou copié.

#### Erreur de migration

Si la migration échoue, l'app affiche un message d'erreur.

La première chose à faire est d'essayer de comprendre ce que dit le message.
Si tous les fichiers ont été copiés correctement, l'erreur est probablement survenue lors de la suppression des anciens fichiers. Tu peux fermer l'app et supprimer manuellement les anciens fichiers.

Veille à NE PAS SUPPRIMER le fichier nommé `runtime_path_override` dans l'ancien chemin de runtime, il sert à l'app pour détecter qu'un changement de chemin a eu lieu.

En cas de doute, rejoins notre [serveur Discord](https://discord.gdlauncher.com) pour demander de l'aide.
