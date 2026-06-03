import type { LocaleData } from "./vsData"

const it: LocaleData = {
  chrome: {
    compareBreadcrumb: "Confronta",
    feature: "Funzionalità",
    tryGdl: "Prova GDLauncher",
    seeAllComparisons: "Vedi tutti i confronti",
    theVerdict: "Il verdetto",
  },
  hub: {
    pageTitle:
      "GDLauncher vs altri launcher Minecraft: confronti dettagliati",
    pageDescription:
      "Confronti dettagliati tra GDLauncher e gli altri launcher Minecraft più diffusi: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "Come si posiziona GDLauncher",
    intro:
      "Stai scegliendo un launcher per Minecraft? Ecco come GDLauncher si confronta con le principali alternative, funzione per funzione. Siamo di parte, ma mettiamo i confronti per iscritto così decidi da solo.",
    competitors: {
      prismlauncher: {
        blurb:
          "Fork leggero e open source di MultiMC. Confronto con GDLauncher su usabilità e gestione modpack.",
      },
      "curseforge-app": {
        blurb:
          "Il launcher ufficiale di CurseForge. Confronto su integrazione CurseForge, supporto Modrinth e gestione server integrata.",
      },
      "modrinth-app": {
        blurb:
          "Il launcher solo Modrinth. Dove GDLauncher ti dà Modrinth e CurseForge nello stesso posto.",
      },
      atlauncher: {
        blurb:
          "Il veterano dei launcher modpack. UI, prestazioni e supporto piattaforme a confronto.",
      },
      multimc: {
        blurb:
          "Il launcher leggero per power-user. Dove automazione e workflow modpack divergono.",
      },
      "ftb-app": {
        blurb:
          "Il launcher ufficiale Feed The Beast per i pack FTB e CurseForge. Dove Modrinth, Cloud Instance Sharing e gestione server fanno la differenza.",
      },
      tlauncher: {
        blurb:
          "Launcher che salta l'autenticazione Mojang. Perché viola l'EULA e cosa perdi a usarlo.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher: confronto dettagliato di funzioni, supporto ai modpack, prestazioni e UI. Trova il launcher Minecraft giusto.",
      intro:
        "Prism Launcher è il fork open source di MultiMC più popolare. GDLauncher è un launcher moderno con integrazione profonda di CurseForge e Modrinth. Ecco come si confrontano davvero sulle cose che contano nell'uso quotidiano.",
      rows: [
        {
          feature: "Supporto CurseForge",
          gdl: "Sì",
          competitor: "No",
          note: "Quando un autore di mod disattiva l'accesso via API di terze parti, Prism ti chiede di scaricare quel file a mano in un browser",
        },
        { feature: "Supporto Modrinth", gdl: "Sì", competitor: "Sì" },
        { feature: "Gestione Java automatica", gdl: "Sì", competitor: "Sì" },
        { feature: "Aggiornamento mod automatico", gdl: "Sì", competitor: "No (solo controllo manuale)" },
        { feature: "Aggiornamento modpack automatico", gdl: "Sì", competitor: "No (solo controllo manuale)" },
        { feature: "Multi-istanza", gdl: "Sì", competitor: "Sì" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sì (codice in un clic, mix CF + MR)",
          competitor: "No (export manuale, niente mix CF + MR)",
        },
        { feature: "Gestione server", gdl: "Sì (integrata)", competitor: "No" },
        { feature: "UI moderna", gdl: "Sì", competitor: "No" },
        {
          feature: "Paga gli autori di addon",
          gdl: "Sì",
          competitor: "No",
        },
        { feature: "Source su GitHub", gdl: "Sì", competitor: "Sì" },
        { feature: "Leggero (RAM)", gdl: "No", competitor: "Sì" },
      ],
      verdict:
        "Prism è ottimo se vuoi un launcher essenziale e leggero e non ti dispiace fare più lavoro a mano per i modpack. GDLauncher è per i giocatori che vogliono installazioni in un clic da CurseForge e Modrinth, Cloud Instance Sharing e gestione server integrata senza uscire dall'app. Se sei nuovo al Minecraft moddato o preferisci la rifinitura al minimalismo, GDLauncher è la strada più semplice.",
      sections: [
        {
          heading: "Workflow modpack",
          paragraphs: [
            "Prism e GDLauncher sanno sfogliare e installare pack CurseForge da dentro il launcher, quindi l'esperienza quotidiana è simile. L'attrito sta ai bordi: quando un autore di mod ha disattivato l'accesso via API di terze parti per il suo file, Prism ti chiede di cliccare su ogni link bloccato e scaricare quei file a mano in un browser. La partnership di GDLauncher con CurseForge recupera questi file direttamente, quindi le installazioni restano in un clic anche quando un pack include mod bloccate.",
            "I pack Modrinth funzionano allo stesso modo in entrambi i launcher: sfoglia dall'app, installa in un clic.",
          ],
        },
        {
          heading: "UI e discovery",
          paragraphs: [
            "L'UI in Qt di Prism è funzionale ma essenziale: la vista principale è una lista di istanze. L'UI di GDLauncher è pensata appositamente per trovare e gestire modpack, con browser integrato, raggruppamento istanze, drag-and-drop per riordinare e card visuali. Soggettivo, ma vale la pena dare un'occhiata agli screenshot.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher ha Cloud Instance Sharing in un clic: incolla un codice, ottieni la stessa identica configurazione. Prism ha export/import via file, che funziona ma non è altrettanto fluido per condividere con gli amici.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App: confronto di funzioni, pubblicità, supporto Modrinth e gestione server. Trova il modo migliore per giocare Minecraft moddato.",
      intro:
        "La CurseForge App è il launcher ufficiale per i contenuti CurseForge. GDLauncher si integra anche con CurseForge e aggiunge Modrinth nello stesso browser, Cloud Instance Sharing tra le due piattaforme e gestione server integrata. Ecco i dettagli.",
      rows: [
        {
          feature: "Supporto CurseForge",
          gdl: "Sì",
          competitor: "Sì (nativo, è la loro app)",
        },
        { feature: "Supporto Modrinth", gdl: "Sì", competitor: "No" },
        { feature: "Gestione Java automatica", gdl: "Sì", competitor: "Sì" },
        { feature: "Aggiornamento mod automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Aggiornamento modpack automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Multi-istanza", gdl: "Sì", competitor: "Sì" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sì (codice in un clic, mix CF + MR)",
          competitor: "Sì (solo CurseForge)",
        },
        { feature: "Gestione server", gdl: "Sì (integrata)", competitor: "No" },
        {
          feature: "Pubblicità in-app",
          gdl: "Sì (banner in-app)",
          competitor: "Sì (banner in-app)",
        },
        { feature: "Source su GitHub", gdl: "Sì", competitor: "No" },
        { feature: "Paga gli autori di addon", gdl: "Sì", competitor: "Sì" },
      ],
      verdict:
        "Se installi solo contenuti CurseForge, la CurseForge App è la scelta ufficiale. GDLauncher offre la stessa integrazione CurseForge, in più Modrinth nello stesso browser, Cloud Instance Sharing che viaggia con setup misti CurseForge + Modrinth e gestione server integrata.",
      sections: [
        {
          heading: "Modrinth nello stesso launcher",
          paragraphs: [
            "La CurseForge App è, per scelta progettuale, solo CurseForge. Modrinth sta crescendo rapidamente, specialmente per mod Fabric, mod di performance e shader, e molti autori pubblicano su entrambe le piattaforme. Il browser integrato di GDLauncher cerca su entrambe contemporaneamente: non devi scegliere.",
          ],
        },
        {
          heading: "Gestione server",
          paragraphs: [
            "GDLauncher include la gestione dei server Minecraft: crea un server Vanilla, Forge, Fabric, NeoForge o Quilt e gestiscilo nella stessa UI delle tue istanze singleplayer. La CurseForge App non include gestione dei server.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Entrambi i launcher sanno condividere un setup con un amico. La CurseForge App tiene tutto dentro l'ecosistema CurseForge: puoi passare un modpack CurseForge, ma un setup che mescola mod CurseForge e mod Modrinth non viaggia intatto. Cloud Instance Sharing di GDLauncher accetta anche il caso misto: incolli un codice, il destinatario riceve la tua istanza esatta con i file di entrambe le piattaforme riscaricati dai loro CDN originali.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: qual è il miglior launcher Minecraft per mod e modpack? Confronto di funzioni, piattaforme ed ecosistemi.",
      intro:
        "La Modrinth App è il launcher ufficiale di Modrinth, ottima scelta se usi solo contenuti Modrinth. GDLauncher si integra anche con Modrinth, aggiunge CurseForge, Cloud Instance Sharing e gestione server. Ecco il confronto.",
      rows: [
        {
          feature: "Supporto CurseForge",
          gdl: "Sì",
          competitor: "No",
        },
        {
          feature: "Supporto Modrinth",
          gdl: "Sì",
          competitor: "Sì (nativo, è la loro app)",
        },
        { feature: "Gestione Java automatica", gdl: "Sì", competitor: "Sì" },
        { feature: "Aggiornamento mod automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Aggiornamento modpack automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Multi-istanza", gdl: "Sì", competitor: "Sì" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sì (codice in un clic, mix CF + MR)",
          competitor: "No (export manuale, solo Modrinth)",
        },
        { feature: "Gestione server", gdl: "Sì (integrata)", competitor: "Sì (Modrinth Hosting)" },
        { feature: "UI moderna", gdl: "Sì", competitor: "Sì" },
        { feature: "Source su GitHub", gdl: "Sì", competitor: "Sì" },
        { feature: "Paga gli autori di addon", gdl: "Sì", competitor: "Sì" },
        { feature: "Leggero", gdl: "Medio", competitor: "Medio" },
      ],
      verdict:
        "La Modrinth App va benissimo se vivi interamente nell'ecosistema Modrinth. Ma molti dei modpack più popolari (RLCraft, ATM10, DawnCraft, la linea FTB) sono ancora esclusivi CurseForge, e anche i pack cross-platform sono di solito CurseForge-first. GDLauncher ti dà Modrinth e CurseForge in un unico browser, più la Cloud Instance Sharing per gli amici e la gestione server integrata. Scegli GDLauncher per l'ecosistema più ampio; scegli la Modrinth App per un'esperienza tutta Modrinth.",
      sections: [
        {
          heading: "Il gap CurseForge",
          paragraphs: [
            "La differenza più grande è semplice: la Modrinth App non può installare contenuti CurseForge. Per le mod solo Modrinth non importa. Ma CurseForge ospita ancora la più grande libreria di modpack e tante mod Forge più datate in esclusiva. Il browser di GDLauncher mostra entrambe le piattaforme in un'unica ricerca: scegli quella che ha la versione che ti serve.",
          ],
        },
        {
          heading: "Entrambi gli ecosistemi vanno bene",
          paragraphs: [
            "Modrinth ha una libreria più piccola ma un sito più veloce e senza pubblicità, e API migliori per i mod author. CurseForge ha il catalogo più profondo e i pack storici. La maggior parte delle mod popolari oggi è su entrambi. La scelta di GDLauncher è supportare entrambi nativamente invece di costringerti a scegliere.",
          ],
        },
        {
          heading: "Gestione server e Cloud Instance Sharing",
          paragraphs: [
            "La gestione server di Modrinth è l'integrazione a pagamento Modrinth Hosting: fai provisioning di un server via Modrinth e lo gestisci dall'app. La gestione server di GDLauncher è locale: avvii un server Vanilla / Forge / Fabric / NeoForge / Quilt sulla tua macchina con console live, gestione giocatori e gli stessi parametri d'istanza che usi in singleplayer, senza bolletta di hosting.",
            "Cloud Instance Sharing è l'altra funzione GDLauncher che la Modrinth App non replica. Incolla un codice e ottieni la configurazione esatta, con un mix di contenuti CurseForge + Modrinth nello stesso share.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher: confronto dettagliato di UI, supporto ai modpack, gestione server ed esperienza sviluppatore. Qual è il miglior launcher Minecraft?",
      intro:
        "ATLauncher è un launcher modpack basato su Java che esiste da molti anni, con il suo ecosistema di pack ATLauncher. GDLauncher è l'alternativa più recente in Rust + Solid, con UI moderna e installazioni in un clic da CurseForge e Modrinth. Ecco il confronto.",
      rows: [
        {
          feature: "Supporto CurseForge",
          gdl: "Sì",
          competitor: "Parziale (workaround)",
          note: "Quando un autore di mod disattiva l'accesso via API di terze parti, ATLauncher ti chiede di scaricare quel file a mano in un browser",
        },
        { feature: "Supporto Modrinth", gdl: "Sì", competitor: "Sì" },
        { feature: "Gestione Java automatica", gdl: "Sì", competitor: "Sì" },
        { feature: "Aggiornamento mod automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Aggiornamento modpack automatico", gdl: "Sì", competitor: "Sì (con conferma)" },
        { feature: "Multi-istanza", gdl: "Sì", competitor: "Sì" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sì (codice in un clic, mix CF + MR)",
          competitor: "No (export manuale, niente mix CF + MR)",
        },
        { feature: "Gestione server", gdl: "Sì (integrata)", competitor: "No" },
        {
          feature: "UI moderna",
          gdl: "Sì",
          competitor: "Parziale (Java Swing con FlatLaf)",
        },
        { feature: "Paga gli autori di addon", gdl: "Sì", competitor: "No" },
        { feature: "Source su GitHub", gdl: "Sì", competitor: "Sì" },
        {
          feature: "Pubblicare modpack custom",
          gdl: "Sì (via Cloud Instance Sharing, codice in un clic)",
          competitor: "Sì (pack ATLauncher)",
        },
      ],
      verdict:
        "ATLauncher è una buona scelta se vuoi nello specifico la lista curata di pack di ATLauncher o sei già abituato al suo workflow. I punti di forza di GDLauncher sono UI più moderna, integrazione CurseForge più profonda, Cloud Instance Sharing e gestione server integrata. Per la maggior parte dei giocatori Minecraft moddato nel 2026, l'esperienza GDLauncher è più vicina a quello che ci si aspetta da un'app moderna.",
      sections: [
        {
          heading: "Il salto di generazione UI",
          paragraphs: [
            "ATLauncher usa Java Swing con il look moderno FlatLaf sovrapposto. È un passo avanti rispetto al classico Swing, ma resta indietro rispetto ai launcher nativi moderni su densità, animazioni e sensazione di piattaforma. GDLauncher è costruito con Solid e usa un design system in casa basato su UnoCSS, con drag-and-drop, animazioni e raggruppamenti che hanno la stessa fluidità di un'app nativa.",
          ],
        },
        {
          heading: "Integrazione CurseForge",
          paragraphs: [
            "ATLauncher e GDLauncher sanno entrambi sfogliare e installare pack CurseForge dall'app, quindi il quotidiano è molto simile. L'attrito vive ai bordi: quando un autore di mod ha disattivato l'accesso via API di terze parti per il suo file, ATLauncher ti chiede di cliccare su ogni link bloccato e scaricare quei file a mano in un browser. La partnership di GDLauncher con CurseForge recupera questi file direttamente, quindi le installazioni restano in un clic anche quando un pack include mod bloccate.",
          ],
        },
        {
          heading: "Pack ATLauncher vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher ospita il suo ecosistema di pack. GDLauncher non gioca su quel terreno: al suo posto, la Cloud Instance Sharing permette a chiunque di condividere la propria configurazione esatta (mod, config, impostazioni) con un singolo codice. Filosofie diverse: scegli quella che corrisponde al tuo modo di giocare con gli amici.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC: confronto dettagliato di funzioni, automazione, gestione modpack e UI moderna. Trova il launcher Minecraft giusto.",
      intro:
        "MultiMC ha aperto la strada al lancio di Minecraft in multi-istanza, anche se la sua ultima release ufficiale è stata la 0.6.14 a dicembre 2021 e gran parte dello sviluppo attivo si è spostato sui fork (Prism Launcher in testa). GDLauncher è un launcher moderno e con scelte forti, con automazione pesante. Ecco il confronto pratico.",
      rows: [
        {
          feature: "Supporto CurseForge",
          gdl: "Sì",
          competitor: "No",
        },
        { feature: "Supporto Modrinth", gdl: "Sì", competitor: "Sì" },
        { feature: "Gestione Java automatica", gdl: "Sì", competitor: "No" },
        { feature: "Aggiornamento mod automatico", gdl: "Sì", competitor: "No" },
        { feature: "Aggiornamento modpack automatico", gdl: "Sì", competitor: "No" },
        {
          feature: "Multi-istanza",
          gdl: "Sì",
          competitor: "Sì (la sua specialità)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sì (codice in un clic, mix CF + MR)",
          competitor: "No (export manuale, niente mix CF + MR)",
        },
        { feature: "Gestione server", gdl: "Sì (integrata)", competitor: "No" },
        { feature: "UI moderna", gdl: "Sì", competitor: "No" },
        { feature: "Paga gli autori di addon", gdl: "Sì", competitor: "No" },
        { feature: "Source su GitHub", gdl: "Sì", competitor: "Sì" },
        { feature: "Leggero", gdl: "No", competitor: "Sì (molto)" },
      ],
      verdict:
        "MultiMC è una grande scelta se vuoi un launcher minuscolo e iper-flessibile e ti va di fare da solo il setup di Java, la gestione delle mod e gli aggiornamenti. GDLauncher è per i giocatori che preferiscono avere queste cose gestite automaticamente: Java automatico, aggiornamenti automatici, installazione in un clic, Cloud Instance Sharing e gestione server, senza sacrificare il workflow multi-istanza che MultiMC ha inaugurato.",
      sections: [
        {
          heading: "Automazione vs controllo",
          paragraphs: [
            "Il design di MultiMC è \"non fare nulla che l'utente non abbia chiesto.\" Vuol dire che imposti tu il path di Java, scegli tu la versione, gestisci tu le mod, le aggiorni tu. I power user lo adorano. I nuovi rimbalzano.",
            "GDLauncher prende l'approccio opposto: rileva quello che serve a ogni istanza, lo installa, lo tiene aggiornato, ma espone tutti gli stessi controlli nelle impostazioni d'istanza se vuoi sovrascrivere qualcosa. I default funzionano; i controlli ci sono ancora.",
          ],
        },
        {
          heading: "Gestione dei modpack",
          paragraphs: [
            "MultiMC ha un browser Modrinth integrato, ma nessuna integrazione CurseForge. Per giocare a pack CurseForge devi importarli manualmente come file zip o usare strumenti di terze parti per recuperare il manifest. Il browser di GDLauncher mostra CurseForge e Modrinth fianco a fianco, con installazione in un clic su entrambi.",
          ],
        },
        {
          heading: "L'eredità",
          paragraphs: [
            "MultiMC non rilascia una nuova versione da dicembre 2021; l'energia del progetto si è di fatto spostata su Prism Launcher e altri fork. Se usi MultiMC da anni e vuoi una UI più moderna senza perdere il workflow, Prism è il percorso di upgrade naturale; GDLauncher è il salto più grande (più automazione, meno passi manuali). Provali entrambi e prendi il modello che si adatta a come usi davvero un launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Condividere un setup con un amico in MultiMC vuol dire esportare l'istanza in uno zip e passare il file. Funziona, ma è un file che devi ospitare da qualche parte, e il destinatario deve importarlo nello stesso modo. Cloud Instance Sharing di GDLauncher sostituisce tutto questo con un codice corto: lo incolli, il launcher tira lo snapshot dal servizio GDL, e le mod vengono riscaricate dai loro CDN originali. Un codice, contenuti misti CurseForge + Modrinth nello stesso share, niente zip da passare.",
          ],
        },
      ],
    },
  },
}

export default it
