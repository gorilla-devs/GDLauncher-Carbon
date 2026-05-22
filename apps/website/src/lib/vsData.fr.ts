import type { LocaleData } from "./vsData"

const fr: LocaleData = {
  chrome: {
    compareBreadcrumb: "Comparer",
    feature: "Fonctionnalité",
    tryGdl: "Essayer GDLauncher",
    seeAllComparisons: "Voir toutes les comparaisons",
    theVerdict: "Le verdict",
  },
  hub: {
    pageTitle:
      "GDLauncher vs autres launchers Minecraft : comparatifs détaillés",
    pageDescription:
      "Comparatifs détaillés entre GDLauncher et les autres launchers Minecraft populaires : Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "GDLauncher face aux autres",
    intro:
      "Tu hésites sur le launcher Minecraft à utiliser ? Voici comment GDLauncher se compare aux principales alternatives, fonctionnalité par fonctionnalité. On n'est pas neutres, mais on met tout par écrit pour que tu décides toi-même.",
    competitors: {
      prismlauncher: {
        blurb:
          "Fork léger et open source de MultiMC. Comparaison à GDLauncher sur l'ergonomie et le support des modpacks.",
      },
      "curseforge-app": {
        blurb:
          "Le launcher officiel de CurseForge. Comparaison sur l'intégration CurseForge, le support Modrinth et la gestion de serveur intégrée.",
      },
      "modrinth-app": {
        blurb:
          "Le launcher Modrinth seul. Là où GDLauncher te donne Modrinth et CurseForge au même endroit.",
      },
      atlauncher: {
        blurb:
          "Le vétéran des launchers modpack. UI, performances et support des plateformes côte à côte.",
      },
      multimc: {
        blurb:
          "Le launcher léger pour power-users. Là où l'automatisation et les workflows modpack divergent.",
      },
      "ftb-app": {
        blurb:
          "Le launcher officiel de Feed The Beast pour les packs FTB et CurseForge. Là où Modrinth, le Cloud Instance Sharing et la gestion de serveur diffèrent.",
      },
      tlauncher: {
        blurb:
          "Launcher qui contourne l'authentification Mojang. Pourquoi cette approche enfreint l'EULA et ce que tu y perds.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher : comparatif détaillé des fonctionnalités, du support modpack, des performances et de l'UI. Trouve le bon launcher Minecraft.",
      intro:
        "Prism Launcher est le fork open source populaire de MultiMC. GDLauncher est un launcher moderne avec une intégration profonde de CurseForge et Modrinth. Voici comment ils se comparent vraiment sur ce qui compte au quotidien.",
      rows: [
        {
          feature: "Support CurseForge",
          gdl: "Oui",
          competitor: "Partiel (workaround)",
          note: "Quand un auteur de mod a désactivé l'accès via API tierce, Prism te demande de télécharger ce fichier manuellement dans un browser",
        },
        { feature: "Support Modrinth", gdl: "Oui", competitor: "Oui" },
        { feature: "Gestion Java automatique", gdl: "Oui", competitor: "Oui" },
        { feature: "Mises à jour mods auto", gdl: "Oui", competitor: "Non (vérification manuelle uniquement)" },
        { feature: "Mises à jour modpack auto", gdl: "Oui", competitor: "Non (vérification manuelle uniquement)" },
        { feature: "Multi-instance", gdl: "Oui", competitor: "Oui" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Oui (code en un clic, mix CF + MR)",
          competitor: "Non (export manuel, pas de mix CF + MR)",
        },
        { feature: "Gestion de serveur", gdl: "Oui (intégrée)", competitor: "Non" },
        { feature: "UI moderne", gdl: "Oui", competitor: "Non" },
        {
          feature: "Rémunère les auteurs d'addons",
          gdl: "Oui",
          competitor: "Non",
        },
        { feature: "Source sur GitHub", gdl: "Oui", competitor: "Oui" },
        { feature: "Léger (RAM)", gdl: "Non", competitor: "Oui" },
      ],
      verdict:
        "Prism est excellent si tu veux un launcher minimaliste, léger, et que ça ne te dérange pas de faire plus de manip pour les modpacks. GDLauncher s'adresse aux joueurs qui veulent des installs en un clic depuis CurseForge et Modrinth, du Cloud Instance Sharing et de la gestion de serveur intégrée sans quitter l'app. Si tu débutes en Minecraft moddé ou que tu préfères le poli au minimalisme, GDLauncher est le chemin le plus simple.",
      sections: [
        {
          heading: "Workflow modpack",
          paragraphs: [
            "Prism et GDLauncher savent tous les deux parcourir et installer les packs CurseForge depuis le launcher, donc l'expérience au quotidien est similaire. La friction est ailleurs : quand un auteur de mod a désactivé l'accès via API tierce pour son fichier, Prism te demande de cliquer sur chaque lien bloqué et de télécharger ces fichiers à la main dans un browser. Le partenariat CurseForge de GDLauncher récupère ces fichiers directement, donc l'install reste en un clic même quand un pack contient des mods bloqués.",
            "Les packs Modrinth fonctionnent pareil dans les deux launchers, browse depuis l'app, install en un clic.",
          ],
        },
        {
          heading: "UI et découverte",
          paragraphs: [
            "L'UI Qt de Prism est fonctionnelle mais utilitaire ; la vue principale est une liste d'instances. L'UI de GDLauncher est pensée spécifiquement pour trouver et gérer des modpacks, avec un browser intégré, du regroupement d'instances, du drag-and-drop pour réorganiser et des cartes visuelles. C'est subjectif, mais ça vaut un coup d'œil sur les screenshots.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher offre du Cloud Instance Sharing en un clic : colle un code, récupère exactement le même setup. Prism a l'export/import d'instance via fichiers, ça marche, mais c'est moins fluide pour partager avec des amis.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App : comparatif des fonctionnalités, des pubs, du support Modrinth et de la gestion de serveur. La meilleure façon de jouer à Minecraft moddé.",
      intro:
        "La CurseForge App est le launcher officiel pour le contenu CurseForge. GDLauncher s'intègre aussi à CurseForge, et y ajoute Modrinth dans le même browser, le Cloud Instance Sharing entre les deux plateformes et la gestion de serveur intégrée. Voici les détails.",
      rows: [
        {
          feature: "Support CurseForge",
          gdl: "Oui",
          competitor: "Oui (natif, c'est leur app)",
        },
        { feature: "Support Modrinth", gdl: "Oui", competitor: "Non" },
        { feature: "Gestion Java automatique", gdl: "Oui", competitor: "Oui" },
        { feature: "Mises à jour mods auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Mises à jour modpack auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Multi-instance", gdl: "Oui", competitor: "Oui" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Oui (code en un clic, mix CF + MR)",
          competitor: "Oui (CurseForge uniquement)",
        },
        { feature: "Gestion de serveur", gdl: "Oui (intégrée)", competitor: "Non" },
        {
          feature: "Publicités dans l'app",
          gdl: "Oui (bandeau in-app)",
          competitor: "Oui (bandeau in-app)",
        },
        { feature: "Source sur GitHub", gdl: "Oui", competitor: "Non" },
        { feature: "Rémunère les auteurs d'addons", gdl: "Oui", competitor: "Oui" },
      ],
      verdict:
        "Si tu n'installes que du contenu CurseForge, la CurseForge App est le choix officiel. GDLauncher t'offre la même intégration CurseForge avec en plus Modrinth dans le même browser, le Cloud Instance Sharing qui voyage avec des setups mixtes CurseForge + Modrinth, et la gestion de serveur intégrée.",
      sections: [
        {
          heading: "Modrinth dans le même launcher",
          paragraphs: [
            "La CurseForge App est, par conception, CurseForge-only. Modrinth grandit vite, surtout pour les mods Fabric, les mods de performance et les shaders, et beaucoup d'auteurs publient maintenant sur les deux plateformes. Le browser intégré de GDLauncher cherche dans les deux à la fois, tu n'as pas à choisir.",
          ],
        },
        {
          heading: "Gestion de serveur",
          paragraphs: [
            "GDLauncher inclut la gestion de serveur Minecraft, crée un serveur Vanilla, Forge, Fabric, NeoForge ou Quilt et gère-le dans la même UI que tes instances solo. La CurseForge App n'inclut pas de gestion de serveur.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Les deux launchers savent partager un setup avec un pote. La CurseForge App garde tout dans l'écosystème CurseForge, tu peux refiler un modpack CurseForge, mais un setup qui mélange des mods CurseForge et des mods Modrinth ne passe pas intact. Le Cloud Instance Sharing de GDLauncher gère aussi le cas mixte : tu colles un code, le destinataire récupère ton instance exacte avec les fichiers des deux plateformes retéléchargés depuis leurs CDNs d'origine.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App : quel launcher Minecraft est le mieux pour les mods et modpacks ? Comparatif des fonctionnalités, plateformes et écosystèmes.",
      intro:
        "La Modrinth App est le launcher officiel de Modrinth, un excellent choix si tu n'utilises que du contenu Modrinth. GDLauncher s'intègre aussi à Modrinth, ajoute CurseForge, le Cloud Instance Sharing et la gestion de serveur. Voici le face-à-face.",
      rows: [
        {
          feature: "Support CurseForge",
          gdl: "Oui",
          competitor: "Non",
        },
        {
          feature: "Support Modrinth",
          gdl: "Oui",
          competitor: "Oui (natif, c'est leur app)",
        },
        { feature: "Gestion Java automatique", gdl: "Oui", competitor: "Oui" },
        { feature: "Mises à jour mods auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Mises à jour modpack auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Multi-instance", gdl: "Oui", competitor: "Oui" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Oui (code en un clic, mix CF + MR)",
          competitor: "Non (export manuel, Modrinth uniquement)",
        },
        { feature: "Gestion de serveur", gdl: "Oui (intégrée)", competitor: "Oui (Modrinth Hosting)" },
        { feature: "UI moderne", gdl: "Oui", competitor: "Oui" },
        { feature: "Source sur GitHub", gdl: "Oui", competitor: "Oui" },
        { feature: "Rémunère les auteurs d'addons", gdl: "Oui", competitor: "Oui" },
        { feature: "Léger", gdl: "Moyen", competitor: "Moyen" },
      ],
      verdict:
        "La Modrinth App est top si tu vis entièrement dans l'écosystème Modrinth. Mais beaucoup des modpacks les plus populaires (RLCraft, ATM10, DawnCraft, la gamme FTB) restent exclusifs à CurseForge, et même les packs disponibles sur les deux plateformes sortent généralement d'abord sur CurseForge. GDLauncher te donne Modrinth et CurseForge dans un seul browser, plus le Cloud Instance Sharing pour tes potes, plus la gestion de serveur intégrée. Choisis GDLauncher pour l'écosystème plus large ; choisis la Modrinth App pour une expérience focus Modrinth.",
      sections: [
        {
          heading: "L'écart CurseForge",
          paragraphs: [
            "La plus grosse différence est simple : la Modrinth App ne peut pas installer du contenu CurseForge. Pour les mods Modrinth-only, ça n'a pas d'importance. Mais CurseForge héberge toujours la plus grande bibliothèque de modpacks et de nombreux vieux mods Forge en exclu. Le browser de GDLauncher montre les deux plateformes en une recherche, tu prends celle qui a la version dont tu as besoin.",
          ],
        },
        {
          heading: "Les deux écosystèmes sont bons",
          paragraphs: [
            "Modrinth a une bibliothèque plus petite mais un site plus rapide et sans pubs, et de meilleures APIs pour les moddeurs. CurseForge a le catalogue plus profond et les packs historiques. La plupart des mods populaires sont désormais sur les deux. La stratégie de GDLauncher est de supporter les deux nativement plutôt que de te forcer à choisir.",
          ],
        },
        {
          heading: "Gestion de serveur et Cloud Instance Sharing",
          paragraphs: [
            "La gestion de serveur de Modrinth, c'est l'intégration payante Modrinth Hosting : tu provisionnes un serveur via Modrinth et tu le gères depuis l'app. La gestion de serveur de GDLauncher est locale : tu montes un serveur Vanilla / Forge / Fabric / NeoForge / Quilt sur ta propre machine, avec console en direct, gestion des joueurs et les mêmes paramètres d'instance qu'en solo, sans facture d'hébergement.",
            "Le Cloud Instance Sharing, c'est l'autre fonctionnalité GDLauncher que la Modrinth App ne réplique pas. Colle un code, récupère le setup exact avec un mix de contenu CurseForge + Modrinth dans le même partage.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher : comparatif détaillé de l'UI, du support modpack, de la gestion de serveur et de l'expérience développeur. Lequel est le meilleur launcher Minecraft ?",
      intro:
        "ATLauncher est un launcher modpack basé Java existant depuis longtemps, avec son propre écosystème de packs ATLauncher. GDLauncher est l'alternative plus récente en Rust + Solid, avec une UI moderne et des installs en un clic depuis CurseForge / Modrinth. Voici la comparaison.",
      rows: [
        {
          feature: "Support CurseForge",
          gdl: "Oui",
          competitor: "Partiel (workaround)",
          note: "Quand un auteur de mod a désactivé l'accès via API tierce, ATLauncher te demande de télécharger ce fichier manuellement dans un browser",
        },
        { feature: "Support Modrinth", gdl: "Oui", competitor: "Oui" },
        { feature: "Gestion Java automatique", gdl: "Oui", competitor: "Oui" },
        { feature: "Mises à jour mods auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Mises à jour modpack auto", gdl: "Oui", competitor: "Oui (avec confirmation)" },
        { feature: "Multi-instance", gdl: "Oui", competitor: "Oui" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Oui (code en un clic, mix CF + MR)",
          competitor: "Non (export manuel, pas de mix CF + MR)",
        },
        { feature: "Gestion de serveur", gdl: "Oui (intégrée)", competitor: "Non" },
        {
          feature: "UI moderne",
          gdl: "Oui",
          competitor: "Partiel (Java Swing avec FlatLaf)",
        },
        { feature: "Rémunère les auteurs d'addons", gdl: "Oui", competitor: "Non" },
        { feature: "Source sur GitHub", gdl: "Oui", competitor: "Oui" },
        {
          feature: "Publication de modpacks custom",
          gdl: "Oui (via Cloud Instance Sharing, code en un clic)",
          competitor: "Oui (packs ATLauncher)",
        },
      ],
      verdict:
        "ATLauncher est un bon choix si tu veux spécifiquement la liste de packs curated d'ATLauncher ou que tu as déjà l'habitude de son workflow. Les forces de GDLauncher sont une UI plus moderne, une intégration CurseForge plus profonde, le Cloud Instance Sharing et la gestion de serveur intégrée. Pour la plupart des joueurs Minecraft moddé en 2026, l'expérience GDLauncher est plus proche de ce qu'on attend d'une app moderne.",
      sections: [
        {
          heading: "Le saut de génération UI",
          paragraphs: [
            "ATLauncher utilise Java Swing avec le look-and-feel moderne FlatLaf par-dessus. C'est un vrai progrès par rapport au Swing classique, mais ça reste en retrait des launchers natifs modernes sur la densité, les animations et le ressenti propre à chaque plateforme. GDLauncher est construit avec Solid et utilise un design system maison basé sur UnoCSS, avec du drag-and-drop, des animations et du regroupement qui sonnent natifs.",
          ],
        },
        {
          heading: "Intégration CurseForge",
          paragraphs: [
            "ATLauncher et GDLauncher savent tous les deux parcourir et installer des packs CurseForge depuis l'app, donc au quotidien c'est très proche. Les frictions vivent sur les bords : quand un auteur de mod a désactivé l'accès API tiers pour son fichier, ATLauncher te demande de cliquer sur chaque lien bloqué et de télécharger ces fichiers manuellement dans un browser. Le partenariat CurseForge de GDLauncher va chercher ces fichiers en direct, donc les installs restent en un clic même quand les packs contiennent des mods bloqués.",
          ],
        },
        {
          heading: "Packs ATLauncher vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher héberge son propre écosystème de packs. GDLauncher ne se bat pas sur ce terrain, à la place, le Cloud Instance Sharing permet à n'importe qui de partager son setup exact (mods, configs, réglages) avec un seul code. Philosophies différentes ; prends ce qui correspond à ta façon de jouer avec tes amis.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC : comparatif détaillé des fonctionnalités, de l'automatisation, de la gestion des modpacks et de l'UI moderne. Trouve le bon launcher Minecraft.",
      intro:
        "MultiMC a été le pionnier du lancement de Minecraft en multi-instance, mais sa dernière release officielle reste la 0.6.14 sortie en décembre 2021 et l'essentiel du développement actif s'est déplacé vers ses forks (Prism Launcher en tête). GDLauncher est un launcher moderne et opinionated avec une automatisation poussée. Voici la comparaison pratique.",
      rows: [
        {
          feature: "Support CurseForge",
          gdl: "Oui",
          competitor: "Non",
        },
        { feature: "Support Modrinth", gdl: "Oui", competitor: "Oui" },
        { feature: "Gestion Java automatique", gdl: "Oui", competitor: "Non" },
        { feature: "Mises à jour mods auto", gdl: "Oui", competitor: "Non" },
        { feature: "Mises à jour modpack auto", gdl: "Oui", competitor: "Non" },
        {
          feature: "Multi-instance",
          gdl: "Oui",
          competitor: "Oui (sa spécialité)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Oui (code en un clic, mix CF + MR)",
          competitor: "Non (export manuel, pas de mix CF + MR)",
        },
        { feature: "Gestion de serveur", gdl: "Oui (intégrée)", competitor: "Non" },
        { feature: "UI moderne", gdl: "Oui", competitor: "Non" },
        { feature: "Rémunère les auteurs d'addons", gdl: "Oui", competitor: "Non" },
        { feature: "Source sur GitHub", gdl: "Oui", competitor: "Oui" },
        { feature: "Léger", gdl: "Non", competitor: "Oui (très)" },
      ],
      verdict:
        "MultiMC est un excellent choix si tu veux un launcher minuscule, ultra flexible, et que tu es à l'aise pour faire ton propre setup Java, ta gestion de mods et tes updates. GDLauncher est pour les joueurs qui préfèrent voir ces choses gérées automatiquement : Java auto, updates auto, installs en un clic, Cloud Instance Sharing et gestion de serveur, sans sacrifier le workflow multi-instance que MultiMC a inauguré.",
      sections: [
        {
          heading: "Automatisation vs contrôle",
          paragraphs: [
            "Le design de MultiMC, c'est \"ne rien faire que l'utilisateur n'a pas demandé.\" Ça veut dire que tu règles le chemin Java, tu choisis la version, tu gères les mods, tu les mets à jour. Les power-users adorent. Les nouveaux joueurs s'en vont.",
            "GDLauncher prend l'approche inverse : détecter ce dont chaque instance a besoin, l'installer, le tenir à jour, mais exposer tous les mêmes leviers dans les réglages d'instance si tu veux surcharger quoi que ce soit. Les defaults marchent ; les contrôles sont toujours là.",
          ],
        },
        {
          heading: "Gestion des modpacks",
          paragraphs: [
            "MultiMC a un browser Modrinth intégré, mais pas d'intégration CurseForge. Pour jouer à un pack CurseForge, il faut l'importer manuellement en zip ou utiliser des outils tiers pour récupérer le manifest. Le browser de GDLauncher affiche CurseForge et Modrinth côte à côte, avec installation en un clic sur les deux.",
          ],
        },
        {
          heading: "L'héritage",
          paragraphs: [
            "MultiMC n'a pas livré de nouvelle release depuis décembre 2021 ; l'énergie du projet s'est concrètement reportée sur Prism Launcher et les autres forks. Si tu utilises MultiMC depuis des années et que tu veux une UI plus moderne sans perdre le workflow, Prism est le chemin d'upgrade naturel ; GDLauncher est le saut plus important (plus d'automatisation, moins d'étapes manuelles). Essaie les deux et prends le modèle qui colle à ton vrai usage d'un launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Partager un setup avec un pote dans MultiMC, ça veut dire exporter l'instance en zip et lui refiler le fichier. Ça marche, mais c'est un fichier que tu dois héberger quelque part, et le destinataire doit l'importer pareil. Le Cloud Instance Sharing de GDLauncher remplace ça par un code court : tu le colles, le launcher tire le snapshot depuis le service GDL, et les mods se retéléchargent depuis leurs CDNs d'origine. Un code, du contenu mixte CurseForge + Modrinth dans le même partage, pas de zip à se passer.",
          ],
        },
      ],
    },
  },
}

export default fr
