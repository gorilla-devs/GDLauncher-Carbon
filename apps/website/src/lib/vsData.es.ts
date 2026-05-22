import type { LocaleData } from "./vsData"

const es: LocaleData = {
  chrome: {
    compareBreadcrumb: "Comparar",
    feature: "Función",
    tryGdl: "Prueba GDLauncher",
    seeAllComparisons: "Ver todas las comparativas",
    theVerdict: "El veredicto",
  },
  hub: {
    pageTitle:
      "GDLauncher vs otros launchers de Minecraft: comparativas en detalle",
    pageDescription:
      "Comparativas detalladas entre GDLauncher y otros launchers de Minecraft populares: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "Cómo se compara GDLauncher",
    intro:
      "¿Eligiendo un launcher de Minecraft? Aquí tienes cómo GDLauncher se mide con las principales alternativas, función por función. Somos parte interesada, pero ponemos las comparativas por escrito para que decidas tú.",
    competitors: {
      prismlauncher: {
        blurb:
          "Fork ligero y open source de MultiMC. Comparativa con GDLauncher en usabilidad y soporte de modpacks.",
      },
      "curseforge-app": {
        blurb:
          "El launcher oficial de CurseForge. Comparativa de integración con CurseForge, soporte de Modrinth y gestión de servidor integrada.",
      },
      "modrinth-app": {
        blurb:
          "El launcher solo Modrinth. Donde GDLauncher te da Modrinth y CurseForge en el mismo sitio.",
      },
      atlauncher: {
        blurb:
          "El veterano de los launchers de modpacks. UI, rendimiento y soporte de plataformas lado a lado.",
      },
      multimc: {
        blurb:
          "El launcher ligero para power users. Donde la automatización y los flujos de modpack divergen.",
      },
      "ftb-app": {
        blurb:
          "El launcher oficial de Feed The Beast para packs FTB y CurseForge. Donde Modrinth, Cloud Instance Sharing y la gestión de servidor cambian.",
      },
      tlauncher: {
        blurb:
          "Launcher que se salta la autenticación de Mojang. Por qué ese enfoque va contra el EULA y qué pierdes al usarlo.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher: comparativa detallada de funciones, soporte de modpacks, rendimiento y UI. Encuentra el launcher de Minecraft adecuado.",
      intro:
        "Prism Launcher es el popular fork open source de MultiMC. GDLauncher es un launcher moderno con una integración profunda con CurseForge y Modrinth. Aquí ves cómo se comparan de verdad en lo que importa en el día a día.",
      rows: [
        {
          feature: "Soporte CurseForge",
          gdl: "Sí",
          competitor: "Parcial (workaround)",
          note: "Cuando un autor de mod ha desactivado el acceso vía API de terceros, Prism te pide bajar ese archivo manualmente desde el browser",
        },
        { feature: "Soporte Modrinth", gdl: "Sí", competitor: "Sí" },
        { feature: "Gestión automática de Java", gdl: "Sí", competitor: "Sí" },
        { feature: "Auto-actualización de mods", gdl: "Sí", competitor: "No (solo comprobación manual)" },
        {
          feature: "Auto-actualización de modpacks",
          gdl: "Sí",
          competitor: "No (solo comprobación manual)",
        },
        { feature: "Multi-instancia", gdl: "Sí", competitor: "Sí" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sí (código en un clic, mezcla CF + MR)",
          competitor: "No (exportación manual, sin mezcla CF + MR)",
        },
        { feature: "Gestión de servidor", gdl: "Sí (integrada)", competitor: "No" },
        { feature: "UI moderna", gdl: "Sí", competitor: "No" },
        {
          feature: "Paga a autores de addons",
          gdl: "Sí",
          competitor: "No",
        },
        { feature: "Código en GitHub", gdl: "Sí", competitor: "Sí" },
        { feature: "Ligero (RAM)", gdl: "No", competitor: "Sí" },
      ],
      verdict:
        "Prism es excelente si quieres un launcher minimalista y ligero y no te molesta hacer más trabajo manual con los modpacks. GDLauncher es para jugadores que quieren instalaciones en un clic desde CurseForge y Modrinth, Cloud Instance Sharing y gestión de servidor integrada sin salir de la app. Si eres nuevo en Minecraft modeado o valoras el acabado más que el minimalismo, GDLauncher es el camino más fácil.",
      sections: [
        {
          heading: "Flujo de modpacks",
          paragraphs: [
            "Prism y GDLauncher pueden buscar e instalar packs CurseForge desde dentro del launcher, así que la experiencia diaria es parecida. La fricción aparece en los casos límite: cuando un autor de mod ha desactivado el acceso vía API de terceros para su archivo, Prism te pide hacer clic en cada enlace bloqueado y descargar esos archivos a mano en un browser. La asociación de GDLauncher con CurseForge recupera esos archivos directamente, así que la instalación sigue siendo de un clic incluso cuando un pack incluye mods bloqueados.",
            "Los packs Modrinth funcionan igual en ambos launchers, navega desde la app e instala en un clic.",
          ],
        },
        {
          heading: "UI y descubrimiento",
          paragraphs: [
            "La UI Qt de Prism es funcional pero utilitaria; la vista principal es una lista de instancias. La UI de GDLauncher está pensada específicamente para encontrar y gestionar modpacks, con browser integrado, agrupación de instancias, drag-and-drop para reordenar y tarjetas visuales. Es subjetivo, pero vale la pena comparar capturas.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher tiene Cloud Instance Sharing en un clic: pegas un código, obtienes exactamente el mismo setup. Prism tiene exportación/importación de instancia por archivos, funciona, pero no es tan fluido para compartir con amigos.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App: comparativa de funciones, publicidad, soporte Modrinth y gestión de servidor. La mejor forma de jugar Minecraft modeado.",
      intro:
        "La CurseForge App es el launcher oficial para contenido de CurseForge. GDLauncher también se integra con CurseForge y añade Modrinth en el mismo browser, Cloud Instance Sharing entre ambas plataformas y gestión de servidor integrada. Aquí está el desglose.",
      rows: [
        {
          feature: "Soporte CurseForge",
          gdl: "Sí",
          competitor: "Sí (nativo, es su app)",
        },
        { feature: "Soporte Modrinth", gdl: "Sí", competitor: "No" },
        { feature: "Gestión automática de Java", gdl: "Sí", competitor: "Sí" },
        { feature: "Auto-actualización de mods", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Auto-actualización de modpacks", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Multi-instancia", gdl: "Sí", competitor: "Sí" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sí (código en un clic, mezcla CF + MR)",
          competitor: "Sí (solo CurseForge)",
        },
        { feature: "Gestión de servidor", gdl: "Sí (integrada)", competitor: "No" },
        {
          feature: "Publicidad en la app",
          gdl: "Sí (banner in-app)",
          competitor: "Sí (banner in-app)",
        },
        { feature: "Código en GitHub", gdl: "Sí", competitor: "No" },
        { feature: "Paga a autores de addons", gdl: "Sí", competitor: "Sí" },
      ],
      verdict:
        "Si solo instalas contenido de CurseForge, la CurseForge App es la opción oficial. GDLauncher te da la misma integración con CurseForge, además de Modrinth en el mismo browser, Cloud Instance Sharing que viaja con setups mixtos de CurseForge + Modrinth, y gestión de servidor integrada.",
      sections: [
        {
          heading: "Modrinth en el mismo launcher",
          paragraphs: [
            "La CurseForge App es, por diseño, solo CurseForge. Modrinth ha crecido rápido, sobre todo para mods de Fabric, mods de rendimiento y shaders, y muchos autores publican ahora en ambas plataformas. El browser integrado de GDLauncher busca en las dos a la vez, así no tienes que elegir.",
          ],
        },
        {
          heading: "Gestión de servidor",
          paragraphs: [
            "GDLauncher incluye gestión de servidor Minecraft, crea un servidor Vanilla, Forge, Fabric, NeoForge o Quilt y gestiónalo desde la misma UI que tus instancias de un jugador. La CurseForge App no incluye gestión de servidor.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Ambos launchers pueden compartir un setup con un amigo. La CurseForge App mantiene todo dentro del ecosistema CurseForge, puedes pasar un modpack de CurseForge, pero un setup que mezcla mods de CurseForge con mods de Modrinth no viaja intacto. El Cloud Instance Sharing de GDLauncher acepta el caso mixto: pegas un código y el receptor recibe tu instancia exacta con archivos de ambas plataformas redescargados desde sus CDNs originales.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: ¿qué launcher de Minecraft es mejor para mods y modpacks? Comparativa de funciones, plataformas y soporte de ecosistemas.",
      intro:
        "La Modrinth App es el launcher oficial de Modrinth y una gran opción si solo usas contenido Modrinth. GDLauncher también se integra con Modrinth y le suma CurseForge, Cloud Instance Sharing y gestión de servidor. Aquí los tienes lado a lado.",
      rows: [
        {
          feature: "Soporte CurseForge",
          gdl: "Sí",
          competitor: "No",
        },
        {
          feature: "Soporte Modrinth",
          gdl: "Sí",
          competitor: "Sí (nativo, es su app)",
        },
        { feature: "Gestión automática de Java", gdl: "Sí", competitor: "Sí" },
        { feature: "Auto-actualización de mods", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Auto-actualización de modpacks", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Multi-instancia", gdl: "Sí", competitor: "Sí" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sí (código en un clic, mezcla CF + MR)",
          competitor: "No (exportación manual, solo Modrinth)",
        },
        { feature: "Gestión de servidor", gdl: "Sí (integrada)", competitor: "Sí (Modrinth Hosting)" },
        { feature: "UI moderna", gdl: "Sí", competitor: "Sí" },
        { feature: "Código en GitHub", gdl: "Sí", competitor: "Sí" },
        { feature: "Paga a autores de addons", gdl: "Sí", competitor: "Sí" },
        { feature: "Ligero", gdl: "Medio", competitor: "Medio" },
      ],
      verdict:
        "La Modrinth App es fantástica si vives totalmente en el ecosistema Modrinth. Pero muchos de los modpacks más populares (RLCraft, ATM10, DawnCraft, la línea de FTB) siguen siendo exclusivos de CurseForge, e incluso los packs multiplataforma suelen salir antes en CurseForge. GDLauncher te da Modrinth y CurseForge en un solo browser, Cloud Instance Sharing con amigos y gestión de servidor integrada. Elige GDLauncher si quieres el ecosistema más amplio; elige Modrinth App si quieres una experiencia focalizada solo en Modrinth.",
      sections: [
        {
          heading: "La brecha de CurseForge",
          paragraphs: [
            "La mayor diferencia es directa: la Modrinth App no puede instalar contenido de CurseForge. Para mods solo de Modrinth da igual. Pero CurseForge sigue albergando la biblioteca de modpacks más grande y muchos mods Forge antiguos en exclusiva. El browser de GDLauncher muestra ambas plataformas en una única búsqueda, así eliges donde esté la versión que necesitas.",
          ],
        },
        {
          heading: "Los dos ecosistemas están bien",
          paragraphs: [
            "Modrinth tiene una biblioteca más pequeña pero un sitio más rápido y sin publicidad, y mejores APIs para modders. CurseForge tiene el catálogo más profundo y los packs históricos. La mayoría de los mods populares están ya en ambos. La estrategia de GDLauncher es soportar los dos nativamente en lugar de forzarte a elegir.",
          ],
        },
        {
          heading: "Gestión de servidor y Cloud Instance Sharing",
          paragraphs: [
            "La gestión de servidor de Modrinth es la integración de pago Modrinth Hosting: provisionas un servidor desde Modrinth y lo gestionas desde la app. La gestión de servidor de GDLauncher es local: levantas un servidor Vanilla / Forge / Fabric / NeoForge / Quilt en tu propia máquina, con consola en vivo, gestión de jugadores y los mismos ajustes de instancia que usas en single player, sin factura de hosting.",
            "Cloud Instance Sharing es la otra función de GDLauncher que la Modrinth App no replica. Pega un código, obtén el setup exacto con contenido mixto de CurseForge + Modrinth en una sola compartición.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher: comparativa detallada de UI, soporte de modpacks, gestión de servidor y experiencia de desarrollo. ¿Cuál es el mejor launcher de Minecraft?",
      intro:
        "ATLauncher es un launcher de modpacks basado en Java con muchos años de recorrido y su propio ecosistema de packs ATLauncher. GDLauncher es la alternativa más nueva en Rust + Solid con UI moderna e instalaciones en un clic desde CurseForge / Modrinth. Veamos cómo se comparan.",
      rows: [
        {
          feature: "Soporte CurseForge",
          gdl: "Sí",
          competitor: "Parcial (workaround)",
          note: "Cuando un autor de mod ha desactivado el acceso vía API de terceros, ATLauncher te pide bajar ese archivo manualmente desde el browser",
        },
        { feature: "Soporte Modrinth", gdl: "Sí", competitor: "Sí" },
        { feature: "Gestión automática de Java", gdl: "Sí", competitor: "Sí" },
        { feature: "Auto-actualización de mods", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Auto-actualización de modpacks", gdl: "Sí", competitor: "Sí (con confirmación)" },
        { feature: "Multi-instancia", gdl: "Sí", competitor: "Sí" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sí (código en un clic, mezcla CF + MR)",
          competitor: "No (exportación manual, sin mezcla CF + MR)",
        },
        { feature: "Gestión de servidor", gdl: "Sí (integrada)", competitor: "No" },
        {
          feature: "UI moderna",
          gdl: "Sí",
          competitor: "Parcial (Java Swing con FlatLaf)",
        },
        { feature: "Paga a autores de addons", gdl: "Sí", competitor: "No" },
        { feature: "Código en GitHub", gdl: "Sí", competitor: "Sí" },
        {
          feature: "Publicación de modpacks propios",
          gdl: "Sí (vía Cloud Instance Sharing, código en un clic)",
          competitor: "Sí (packs ATLauncher)",
        },
      ],
      verdict:
        "ATLauncher es una opción sólida si quieres específicamente la lista de packs curados de ATLauncher o ya estás cómodo con su flujo. Los puntos fuertes de GDLauncher son una UI más moderna, una integración con CurseForge más profunda, Cloud Instance Sharing y gestión de servidor integrada. Para la mayoría de jugadores de Minecraft modeado en 2026, la experiencia de GDLauncher está más cerca de lo que se espera de una app moderna.",
      sections: [
        {
          heading: "Salto generacional en UI",
          paragraphs: [
            "ATLauncher usa Java Swing con el look-and-feel moderno de FlatLaf montado encima. Es un salto real respecto al Swing clásico, pero sigue por detrás de los launchers nativos modernos en densidad, movimiento y sensación de plataforma. GDLauncher está construido con Solid y usa un sistema de diseño propio basado en UnoCSS con drag-and-drop, animaciones y agrupación que se sienten nativos.",
          ],
        },
        {
          heading: "Integración CurseForge",
          paragraphs: [
            "ATLauncher y GDLauncher buscan e instalan packs de CurseForge desde el propio launcher, así que el día a día se parece. La fricción aparece en los bordes: cuando un autor de mod ha desactivado el acceso vía API de terceros para su archivo, ATLauncher te hace pinchar cada enlace bloqueado y bajar esos archivos manualmente desde el browser. La asociación de GDLauncher con CurseForge se baja esos archivos en directo, así que las instalaciones siguen siendo de un clic incluso cuando los packs incluyen mods bloqueados.",
          ],
        },
        {
          heading: "Packs ATLauncher vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher aloja su propio ecosistema de packs. GDLauncher no compite ahí, en cambio, Cloud Instance Sharing deja a cualquiera compartir su setup exacto (mods, configs, ajustes) con un único código. Filosofías distintas; elige lo que encaje con cómo jugáis tú y tus amigos.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC: comparativa detallada de funciones, automatización, gestión de modpacks y UI moderna. Encuentra el launcher de Minecraft adecuado.",
      intro:
        "MultiMC fue el pionero de lanzar Minecraft en multi-instancia, aunque su última versión oficial fue la 0.6.14 en diciembre de 2021 y la mayor parte del desarrollo activo se ha trasladado a sus forks (Prism Launcher el principal entre ellos). GDLauncher es un launcher moderno y con opinión, con automatización profunda. Aquí la comparativa práctica.",
      rows: [
        {
          feature: "Soporte CurseForge",
          gdl: "Sí",
          competitor: "No",
        },
        { feature: "Soporte Modrinth", gdl: "Sí", competitor: "Sí" },
        { feature: "Gestión automática de Java", gdl: "Sí", competitor: "No" },
        { feature: "Auto-actualización de mods", gdl: "Sí", competitor: "No" },
        {
          feature: "Auto-actualización de modpacks",
          gdl: "Sí",
          competitor: "No",
        },
        {
          feature: "Multi-instancia",
          gdl: "Sí",
          competitor: "Sí (es su especialidad)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sí (código en un clic, mezcla CF + MR)",
          competitor: "No (exportación manual, sin mezcla CF + MR)",
        },
        { feature: "Gestión de servidor", gdl: "Sí (integrada)", competitor: "No" },
        { feature: "UI moderna", gdl: "Sí", competitor: "No" },
        { feature: "Paga a autores de addons", gdl: "Sí", competitor: "No" },
        { feature: "Código en GitHub", gdl: "Sí", competitor: "Sí" },
        { feature: "Ligero", gdl: "No", competitor: "Sí (mucho)" },
      ],
      verdict:
        "MultiMC es una gran opción si quieres un launcher diminuto, ultra flexible, y estás a gusto haciendo tú mismo el setup de Java, la gestión de mods y las actualizaciones. GDLauncher es para jugadores que prefieren que esas cosas se gestionen automáticamente: Java auto, updates auto, instalaciones en un clic, Cloud Instance Sharing y gestión de servidor, sin sacrificar el flujo multi-instancia que MultiMC inauguró.",
      sections: [
        {
          heading: "Automatización vs control",
          paragraphs: [
            "El diseño de MultiMC es \"no hacer nada que el usuario no haya pedido.\" Eso significa que pones tú la ruta de Java, eliges la versión, gestionas los mods, los actualizas tú. A los power users les encanta. Los jugadores nuevos rebotan.",
            "GDLauncher toma el enfoque opuesto: detectar qué necesita cada instancia, instalarlo, mantenerlo al día, pero exponer los mismos mandos en los ajustes de instancia si quieres sobrescribir algo. Los defaults funcionan; los controles siguen ahí.",
          ],
        },
        {
          heading: "Manejo de modpacks",
          paragraphs: [
            "MultiMC tiene un navegador de Modrinth integrado, pero ninguna integración con CurseForge. Para jugar a packs de CurseForge tendrías que importarlos manualmente como zip o usar herramientas de terceros para sacar el manifest. El navegador de GDLauncher muestra CurseForge y Modrinth uno al lado del otro, con instalación de un clic en ambos.",
          ],
        },
        {
          heading: "El legado",
          paragraphs: [
            "MultiMC no ha lanzado una versión nueva desde diciembre de 2021; la energía del proyecto se ha trasladado de hecho a Prism Launcher y otros forks. Si llevas años usando MultiMC y quieres una UI más moderna sin perder el flujo, Prism es el camino natural de upgrade; GDLauncher es el salto más grande (más automatización, menos pasos manuales). Prueba ambos y quédate con el modelo que encaje con cómo usas de verdad un launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Compartir un setup con un amigo en MultiMC significa exportar la instancia a un zip y pasarle el archivo. Funciona, pero es un archivo que tienes que hostear en algún sitio, y el receptor tiene que importarlo de la misma forma. El Cloud Instance Sharing de GDLauncher sustituye eso por un código corto: lo pegas, el launcher tira del snapshot desde el servicio GDL, y los mods se redescargan desde sus CDNs originales. Un código, contenido mixto de CurseForge + Modrinth en el mismo share, sin zip que pasar.",
          ],
        },
      ],
    },
  },
}

export default es
