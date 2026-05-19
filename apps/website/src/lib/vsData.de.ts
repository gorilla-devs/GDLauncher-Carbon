import type { LocaleData } from "./vsData"

const de: LocaleData = {
  chrome: {
    compareBreadcrumb: "Vergleich",
    feature: "Funktion",
    tryGdl: "GDLauncher ausprobieren",
    seeAllComparisons: "Alle Vergleiche ansehen",
    theVerdict: "Das Fazit",
  },
  hub: {
    pageTitle:
      "GDLauncher vs andere Minecraft-Launcher: Vergleiche im Detail",
    pageDescription:
      "Ausführliche Vergleiche zwischen GDLauncher und anderen beliebten Minecraft-Launchern: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "So schneidet GDLauncher ab",
    intro:
      "Welcher Minecraft-Launcher passt zu dir? Hier siehst du, wie GDLauncher Funktion für Funktion gegen die wichtigsten Alternativen abschneidet. Wir sind nicht neutral, aber wir schreiben die Vergleiche aus, damit du selbst entscheiden kannst.",
    competitors: {
      prismlauncher: {
        blurb:
          "Leichtgewichtiger, quelloffener MultiMC-Fork. Vergleich zu GDLauncher bei Bedienung und Modpack-Support.",
      },
      "curseforge-app": {
        blurb:
          "Der offizielle CurseForge-Launcher. Vergleich bei CurseForge-Integration, Modrinth-Support und integrierter Server-Verwaltung.",
      },
      "modrinth-app": {
        blurb:
          "Der Modrinth-only Launcher. Wo GDLauncher dir Modrinth und CurseForge an einem Ort liefert.",
      },
      atlauncher: {
        blurb:
          "Der Veteran unter den Modpack-Launchern. UI, Performance und Plattform-Support direkt nebeneinander.",
      },
      multimc: {
        blurb:
          "Der schlanke Power-User-Launcher. Wo sich Automatik und Modpack-Workflows unterscheiden.",
      },
      "ftb-app": {
        blurb:
          "Der hauseigene Launcher des Feed-The-Beast-Teams für FTB- und CurseForge-Packs. Wo sich Modrinth-Support, Cloud Instance Sharing und Server-Verwaltung unterscheiden.",
      },
      tlauncher: {
        blurb:
          "Launcher, der die Mojang-Authentifizierung überspringt. Warum dieser Weg gegen den EULA verstößt und was du dafür aufgibst.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher: detaillierter Vergleich von Funktionen, Modpack-Support, Performance und UI. Finde den passenden Minecraft-Launcher.",
      intro:
        "Prism Launcher ist der beliebte Open-Source-Fork von MultiMC. GDLauncher ist ein moderner Launcher mit tiefer CurseForge- und Modrinth-Integration. Hier siehst du, wo sich beide im Alltag wirklich unterscheiden.",
      rows: [
        {
          feature: "CurseForge-Support",
          gdl: "Ja",
          competitor: "Teilweise (Workaround)",
          note: "Wenn ein Mod-Autor den Zugriff über Drittanbieter-APIs deaktiviert hat, fordert Prism dich auf, die Datei manuell im Browser herunterzuladen",
        },
        { feature: "Modrinth-Support", gdl: "Ja", competitor: "Ja" },
        { feature: "Java automatisch verwalten", gdl: "Ja", competitor: "Ja" },
        { feature: "Automatische Mod-Updates", gdl: "Ja", competitor: "Nein (nur manuelle Prüfung)" },
        {
          feature: "Automatische Modpack-Updates",
          gdl: "Ja",
          competitor: "Nein (nur manuelle Prüfung)",
        },
        { feature: "Mehrere Instanzen", gdl: "Ja", competitor: "Ja" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Ja (Ein-Klick-Code, gemischte CF + MR Setups)",
          competitor: "Nein (manueller Export, kein gemischtes CF + MR)",
        },
        { feature: "Server-Verwaltung", gdl: "Ja (integriert)", competitor: "Nein" },
        { feature: "Modernes UI", gdl: "Ja", competitor: "Nein" },
        {
          feature: "Bezahlung für Addon-Autoren",
          gdl: "Ja",
          competitor: "Nein",
        },
        { feature: "Quellcode auf GitHub", gdl: "Ja", competitor: "Ja" },
        { feature: "Schlank (RAM)", gdl: "Nein", competitor: "Ja" },
      ],
      verdict:
        "Prism ist top, wenn du einen reduzierten, schlanken Launcher willst und Modpacks zur Not selbst zusammenstellst. GDLauncher ist für Spielende, die Ein-Klick-Installs aus CurseForge und Modrinth, Cloud Instance Sharing und integrierte Server-Verwaltung ohne App-Wechsel wollen. Wenn du neu bei modded Minecraft bist oder Wert auf Politur statt Minimalismus legst, ist GDLauncher der einfachere Weg.",
      sections: [
        {
          heading: "Modpack-Workflow",
          paragraphs: [
            "Prism und GDLauncher können beide CurseForge-Packs direkt aus dem Launcher heraus durchsuchen und installieren, das alltägliche Erlebnis ist also ähnlich. Der Unterschied steckt in den Randfällen: Wenn ein Mod-Autor den Zugriff über Drittanbieter-APIs für seine Datei abgeschaltet hat, fordert Prism dich auf, jede blockierte Datei einzeln im Browser herunterzuladen. Dank der CurseForge-Partnerschaft holt GDLauncher diese Dateien direkt, sodass die Installation auch bei Packs mit blockierten Mods Ein-Klick bleibt.",
            "Modrinth-Packs funktionieren in beiden Launchern gleich, aus dem App-Browser heraus mit einem Klick installierbar.",
          ],
        },
        {
          heading: "UI und Discovery",
          paragraphs: [
            "Prisms Qt-basiertes UI ist funktional, aber nüchtern; die Hauptansicht ist eine Liste von Instanzen. Das UI von GDLauncher ist speziell fürs Finden und Verwalten von Modpacks gebaut, mit eingebautem Browser, Instance-Gruppen, Drag&Drop-Sortierung und visuellen Karten. Geschmackssache, aber Screenshots vergleichen lohnt sich.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher hat Cloud Instance Sharing per Ein-Klick: Code einfügen, das exakt gleiche Setup ist da. Prism setzt auf Datei-basierten Export/Import von Instanzen, das funktioniert, ist aber für die Weitergabe an Freunde weniger reibungslos.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App: Vergleich von Funktionen, Werbung, Modrinth-Support und Server-Verwaltung. Der bessere Weg, modded Minecraft zu spielen.",
      intro:
        "Die CurseForge App ist der offizielle Launcher für CurseForge-Inhalte. GDLauncher integriert ebenfalls CurseForge, ergänzt um Modrinth im selben Browser, Cloud Instance Sharing über beide Plattformen und integrierte Server-Verwaltung. Hier ist der Überblick.",
      rows: [
        {
          feature: "CurseForge-Support",
          gdl: "Ja",
          competitor: "Ja (nativ, ist ja deren App)",
        },
        { feature: "Modrinth-Support", gdl: "Ja", competitor: "Nein" },
        { feature: "Java automatisch verwalten", gdl: "Ja", competitor: "Ja" },
        { feature: "Automatische Mod-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Automatische Modpack-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Mehrere Instanzen", gdl: "Ja", competitor: "Ja" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Ja (Ein-Klick-Code, gemischte CF + MR Setups)",
          competitor: "Ja (nur CurseForge)",
        },
        { feature: "Server-Verwaltung", gdl: "Ja (integriert)", competitor: "Nein" },
        {
          feature: "Werbefinanziert in der App",
          gdl: "Ja (In-App-Banner)",
          competitor: "Ja (In-App-Banner)",
        },
        { feature: "Quellcode auf GitHub", gdl: "Ja", competitor: "Nein" },
        { feature: "Bezahlung für Addon-Autoren", gdl: "Ja", competitor: "Ja" },
      ],
      verdict:
        "Wenn du nur CurseForge-Inhalte installierst, ist die CurseForge App die offizielle Wahl. GDLauncher liefert dieselbe CurseForge-Integration plus Modrinth im selben Browser, Cloud Instance Sharing für gemischte CurseForge-und-Modrinth-Setups und integrierte Server-Verwaltung.",
      sections: [
        {
          heading: "Modrinth im selben Launcher",
          paragraphs: [
            "Die CurseForge App ist per Design CurseForge-only. Modrinth wächst schnell, besonders bei Fabric-Mods, Performance-Mods und Shadern, viele Autoren veröffentlichen mittlerweile auf beiden Plattformen. Der eingebaute Browser von GDLauncher durchsucht beide gleichzeitig, du musst dich nicht entscheiden.",
          ],
        },
        {
          heading: "Server-Verwaltung",
          paragraphs: [
            "GDLauncher bringt eine integrierte Minecraft-Server-Verwaltung mit, erstelle einen Vanilla-, Forge-, Fabric-, NeoForge- oder Quilt-Server und verwalte ihn im selben UI wie deine Singleplayer-Instanzen. Die CurseForge App enthält keine Server-Verwaltung.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Beide Launcher können ein Setup mit Freunden teilen. Die CurseForge App bleibt komplett im CurseForge-Ökosystem, du kannst ein CurseForge-Modpack weitergeben, aber ein Setup, das CurseForge-Mods mit Modrinth-Mods mischt, übersteht die Reise nicht intakt. GDLauncher's Cloud Instance Sharing nimmt auch den gemischten Fall: einen Code einfügen, der Empfänger bekommt deine exakte Instanz mit Dateien beider Plattformen, frisch von den ursprünglichen CDNs.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: welcher Minecraft-Launcher ist besser für Mods und Modpacks? Vergleich von Funktionen, Plattformen und Ökosystem-Support.",
      intro:
        "Die Modrinth App ist der offizielle Modrinth-Launcher und eine gute Wahl, wenn du nur Modrinth-Inhalte nutzt. GDLauncher integriert ebenfalls Modrinth und kombiniert es mit CurseForge, Cloud Instance Sharing und Server-Verwaltung. Hier ist der direkte Vergleich.",
      rows: [
        {
          feature: "CurseForge-Support",
          gdl: "Ja",
          competitor: "Nein",
        },
        {
          feature: "Modrinth-Support",
          gdl: "Ja",
          competitor: "Ja (nativ, ist ja deren App)",
        },
        { feature: "Java automatisch verwalten", gdl: "Ja", competitor: "Ja" },
        { feature: "Automatische Mod-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Automatische Modpack-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Mehrere Instanzen", gdl: "Ja", competitor: "Ja" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Ja (Ein-Klick-Code, gemischte CF + MR Setups)",
          competitor: "Nein (manueller Export, nur Modrinth)",
        },
        { feature: "Server-Verwaltung", gdl: "Ja (integriert)", competitor: "Ja (Modrinth Hosting)" },
        { feature: "Modernes UI", gdl: "Ja", competitor: "Ja" },
        { feature: "Quellcode auf GitHub", gdl: "Ja", competitor: "Ja" },
        { feature: "Bezahlung für Addon-Autoren", gdl: "Ja", competitor: "Ja" },
        { feature: "Schlank", gdl: "Mittel", competitor: "Mittel" },
      ],
      verdict:
        "Die Modrinth App ist fantastisch, wenn du komplett im Modrinth-Ökosystem lebst. Aber viele der beliebtesten Modpacks (RLCraft, ATM10, DawnCraft, die FTB-Reihe) sind nach wie vor CurseForge-only, und selbst plattformübergreifende Packs erscheinen meist zuerst auf CurseForge. GDLauncher gibt dir Modrinth plus CurseForge in einem Browser, dazu Cloud Instance Sharing für Freunde und integrierte Server-Verwaltung. Wähle GDLauncher für das breitere Ökosystem; wähle die Modrinth App, wenn du einen fokussierten, Modrinth-only Workflow willst.",
      sections: [
        {
          heading: "Die CurseForge-Lücke",
          paragraphs: [
            "Der größte Unterschied ist simpel: Die Modrinth App kann keine CurseForge-Inhalte installieren. Für reine Modrinth-Mods ist das egal. Aber CurseForge hostet weiterhin die größere Modpack-Bibliothek und viele ältere Forge-Mods exklusiv. Der Browser von GDLauncher zeigt beide Plattformen in einer Suche, du kannst nehmen, was die richtige Version hat.",
          ],
        },
        {
          heading: "Beide Ökosysteme sind stark",
          paragraphs: [
            "Modrinth hat eine kleinere Bibliothek, dafür eine schnellere, werbefreie Seite und bessere APIs für Modder. CurseForge hat den tieferen Katalog und die historischen Packs. Die meisten populären Mods stehen mittlerweile auf beiden. GDLauncher unterstützt beide nativ, statt dich zur Wahl zu zwingen.",
          ],
        },
        {
          heading: "Server-Verwaltung und Cloud Instance Sharing",
          paragraphs: [
            "Modrinths Server-Verwaltung ist die kostenpflichtige Modrinth-Hosting-Integration: Du provisionierst einen Server über Modrinth und verwaltest ihn aus der App. GDLaunchers Server-Verwaltung läuft lokal: Vanilla- / Forge- / Fabric- / NeoForge- / Quilt-Server auf deiner eigenen Maschine starten, mit Live-Konsole, Spieler-Management und denselben Instanz-Einstellungen wie im Singleplayer, ohne Hosting-Kosten.",
            "Cloud Instance Sharing ist das zweite GDLauncher-Feature, das die Modrinth App nicht ersetzt. Code einfügen, exaktes Setup mit gemischten CurseForge- und Modrinth-Inhalten in einer einzigen Share-URL.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher: detaillierter Vergleich zu UI, Modpack-Support, Server-Verwaltung und Entwicklerkomfort. Welcher Minecraft-Launcher ist besser?",
      intro:
        "ATLauncher ist ein langjähriger Java-basierter Modpack-Launcher mit eigenem ATLauncher-Pack-Ökosystem. GDLauncher ist die neuere Rust + Solid-Alternative mit modernem UI und Ein-Klick-Installs aus CurseForge / Modrinth. So vergleichen sich beide.",
      rows: [
        {
          feature: "CurseForge-Support",
          gdl: "Ja",
          competitor: "Teilweise (Workaround)",
          note: "Wenn ein Mod-Autor den Zugriff über Drittanbieter-APIs deaktiviert hat, fordert ATLauncher dich auf, die Datei manuell im Browser herunterzuladen",
        },
        { feature: "Modrinth-Support", gdl: "Ja", competitor: "Ja" },
        { feature: "Java automatisch verwalten", gdl: "Ja", competitor: "Ja" },
        { feature: "Automatische Mod-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Automatische Modpack-Updates", gdl: "Ja", competitor: "Ja (mit Bestätigung)" },
        { feature: "Mehrere Instanzen", gdl: "Ja", competitor: "Ja" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Ja (Ein-Klick-Code, gemischte CF + MR Setups)",
          competitor: "Nein (manueller Export, kein gemischtes CF + MR)",
        },
        { feature: "Server-Verwaltung", gdl: "Ja (integriert)", competitor: "Nein" },
        {
          feature: "Modernes UI",
          gdl: "Ja",
          competitor: "Teilweise (Java Swing mit FlatLaf)",
        },
        { feature: "Bezahlung für Addon-Autoren", gdl: "Ja", competitor: "Nein" },
        { feature: "Quellcode auf GitHub", gdl: "Ja", competitor: "Ja" },
        {
          feature: "Eigene Modpacks veröffentlichen",
          gdl: "Ja (über Cloud Instance Sharing, Ein-Klick-Code)",
          competitor: "Ja (ATLauncher-Packs)",
        },
      ],
      verdict:
        "ATLauncher ist solide, wenn du speziell die kuratierte ATLauncher-Pack-Liste willst oder dich an dessen Workflow gewöhnt hast. GDLauncher punktet mit modernem UI, tieferer CurseForge-Integration, Cloud Instance Sharing und integrierter Server-Verwaltung. Für die meisten modded-Minecraft-Spielenden 2026 fühlt sich GDLauncher näher an dem an, was man von einer modernen App erwartet.",
      sections: [
        {
          heading: "UI-Generationssprung",
          paragraphs: [
            "ATLauncher setzt auf Java Swing mit dem modernen FlatLaf-Look-and-Feel obendrauf. Das ist ein echter Schritt nach vorn gegenüber klassischem Swing, hängt aber bei Informationsdichte, Animationen und Plattform-Gefühl trotzdem hinter nativen modernen Launchern zurück. GDLauncher ist mit Solid gebaut und nutzt ein eigenes UnoCSS-Design-System mit nativ wirkendem Drag&Drop, Animationen und Gruppierung.",
          ],
        },
        {
          heading: "CurseForge-Integration",
          paragraphs: [
            "ATLauncher und GDLauncher können beide CurseForge-Packs aus dem Launcher heraus durchsuchen und installieren, der Alltag fühlt sich also ähnlich an. Die Reibung sitzt an den Rändern: Wenn ein Mod-Autor den Drittanbieter-API-Zugriff für seine Datei deaktiviert hat, schickt dich ATLauncher zu jedem blockierten Link in den Browser, um die Datei manuell herunterzuladen. GDLauncher holt diese Dateien dank seiner CurseForge-Partnerschaft direkt, sodass Installs auch bei Packs mit blockierten Mods Ein-Klick bleiben.",
          ],
        },
        {
          heading: "ATLauncher-Packs vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher betreibt ein eigenes Pack-Ökosystem. GDLauncher konkurriert da nicht, sondern lässt mit Cloud Instance Sharing jeden sein genaues Setup (Mods, Configs, Settings) mit einem einzigen Code teilen. Andere Philosophie, wähle, was zu dir und deinen Freunden passt.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC: detaillierter Vergleich zu Funktionen, Automatik, Modpack-Handling und modernem UI. Finde den richtigen Minecraft-Launcher.",
      intro:
        "MultiMC hat das Mehrinstanz-Konzept fürs Minecraft-Starten geprägt, allerdings war 0.6.14 im Dezember 2021 das letzte offizielle Release, und die aktive Entwicklung ist weitgehend in die Forks abgewandert (allen voran Prism Launcher). GDLauncher ist ein moderner, meinungsstarker Launcher mit tiefer Automatik. Hier der praktische Vergleich.",
      rows: [
        {
          feature: "CurseForge-Support",
          gdl: "Ja",
          competitor: "Nein",
        },
        { feature: "Modrinth-Support", gdl: "Ja", competitor: "Ja" },
        { feature: "Java automatisch verwalten", gdl: "Ja", competitor: "Nein" },
        { feature: "Automatische Mod-Updates", gdl: "Ja", competitor: "Nein" },
        {
          feature: "Automatische Modpack-Updates",
          gdl: "Ja",
          competitor: "Nein",
        },
        {
          feature: "Mehrere Instanzen",
          gdl: "Ja",
          competitor: "Ja (Spezialität)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Ja (Ein-Klick-Code, gemischte CF + MR Setups)",
          competitor: "Nein (manueller Export, kein gemischtes CF + MR)",
        },
        { feature: "Server-Verwaltung", gdl: "Ja (integriert)", competitor: "Nein" },
        { feature: "Modernes UI", gdl: "Ja", competitor: "Nein" },
        { feature: "Bezahlung für Addon-Autoren", gdl: "Ja", competitor: "Nein" },
        { feature: "Quellcode auf GitHub", gdl: "Ja", competitor: "Ja" },
        { feature: "Schlank", gdl: "Nein", competitor: "Ja (sehr)" },
      ],
      verdict:
        "MultiMC ist top, wenn du einen winzigen, sehr flexiblen Launcher willst und gerne dein Java-Setup, deine Mods und deine Updates selbst machst. GDLauncher ist für Spielende, die das lieber automatisch erledigt sehen: auto Java, auto Updates, Ein-Klick-Installs, Cloud Instance Sharing und Server-Verwaltung, ohne den Mehrinstanz-Workflow zu verlieren, den MultiMC etabliert hat.",
      sections: [
        {
          heading: "Automatik vs Kontrolle",
          paragraphs: [
            "Das Designprinzip von MultiMC lautet: \"Tue nichts, worum dich der Nutzer nicht gebeten hat.\" Heißt: Du setzt den Java-Pfad, du wählst die Version, du verwaltest die Mods, du updatest sie. Power-User lieben das, neue Spielende winken ab.",
            "GDLauncher geht den umgekehrten Weg: erkennen, was jede Instanz braucht, installieren, aktuell halten, und gleichzeitig alle Stellschrauben in den Instance-Settings exponieren, falls du etwas überschreiben willst. Die Defaults funktionieren; die Kontrollen sind weiterhin da.",
          ],
        },
        {
          heading: "Modpack-Handling",
          paragraphs: [
            "MultiMC hat einen eingebauten Modrinth-Browser, aber keine CurseForge-Integration. Um CurseForge-Packs zu spielen, müsstest du sie manuell als Zip-Datei importieren oder Drittanbieter-Tools zum Abrufen des Manifests nutzen. GDLauncher zeigt CurseForge und Modrinth nebeneinander im Browser, mit Ein-Klick-Installs auf beiden.",
          ],
        },
        {
          heading: "Das Erbe",
          paragraphs: [
            "MultiMC hat seit Dezember 2021 kein neues Release mehr veröffentlicht; die Energie des Projekts ist faktisch in Prism Launcher und andere Forks geflossen. Wenn du seit Jahren MultiMC nutzt und ein moderneres UI ohne Workflow-Verlust willst, ist Prism der natürliche Upgrade-Pfad; GDLauncher ist der größere Sprung (mehr Automatik, weniger manuelle Schritte). Probier beide aus und nimm das Modell, das zu deinem realen Launcher-Verhalten passt.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Ein Setup mit Freunden teilen heißt in MultiMC: die Instanz als Zip exportieren und die Datei rüberreichen. Das funktioniert, aber es ist eine Datei, die du irgendwo hosten musst, und der Empfänger muss sie genauso importieren. GDLauncher's Cloud Instance Sharing ersetzt das durch einen kurzen Code: einfügen, der Launcher zieht den Snapshot vom GDL-Dienst, und Mods werden frisch von den ursprünglichen CDNs geladen. Ein Code, gemischte CurseForge + Modrinth Inhalte im selben Share, keine Zip-Datei zum Weiterreichen.",
          ],
        },
      ],
    },
  },
}

export default de
