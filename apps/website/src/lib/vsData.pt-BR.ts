import type { LocaleData } from "./vsData"

const ptBR: LocaleData = {
  chrome: {
    compareBreadcrumb: "Comparar",
    feature: "Recurso",
    tryGdl: "Experimente o GDLauncher",
    seeAllComparisons: "Ver todas as comparações",
    theVerdict: "O veredito",
  },
  hub: {
    pageTitle:
      "GDLauncher vs outros launchers de Minecraft: comparações detalhadas",
    pageDescription:
      "Comparações detalhadas entre o GDLauncher e outros launchers de Minecraft populares: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "Como o GDLauncher se compara",
    intro:
      "Escolhendo um launcher de Minecraft? Aqui está como o GDLauncher se mede com as principais alternativas, recurso por recurso. A gente é parte interessada, mas deixa as comparações por escrito para você decidir.",
    competitors: {
      prismlauncher: {
        blurb:
          "Fork leve e open source do MultiMC. Comparação com o GDLauncher em usabilidade e suporte a modpacks.",
      },
      "curseforge-app": {
        blurb:
          "O launcher oficial do CurseForge. Comparando integração com CurseForge, suporte a Modrinth e gerenciamento de servidor embutido.",
      },
      "modrinth-app": {
        blurb:
          "Launcher só de Modrinth. Onde o GDLauncher entrega Modrinth e CurseForge no mesmo lugar.",
      },
      atlauncher: {
        blurb:
          "Veterano dos launchers de modpacks. UI, performance e suporte de plataforma lado a lado.",
      },
      multimc: {
        blurb:
          "Launcher leve para power users. Onde a automação e os fluxos de modpack divergem.",
      },
      "ftb-app": {
        blurb:
          "O launcher oficial do Feed The Beast para packs FTB e CurseForge. Onde Modrinth, Cloud Instance Sharing e o gerenciamento de servidor diferem.",
      },
      tlauncher: {
        blurb:
          "Launcher que pula a autenticação Mojang. Por que essa abordagem vai contra o EULA e o que você perde usando.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher: comparação detalhada de recursos, suporte a modpacks, performance e UI. Encontre o launcher de Minecraft certo para você.",
      intro:
        "O Prism Launcher é o popular fork open source do MultiMC. O GDLauncher é um launcher moderno com integração profunda com CurseForge e Modrinth. Aqui está como eles realmente se comparam no que importa no dia a dia.",
      rows: [
        {
          feature: "Suporte CurseForge",
          gdl: "Sim",
          competitor: "Parcial (workaround)",
          note: "Quando um autor de mod desativa o acesso via API de terceiros, o Prism pede que você baixe aquele arquivo manualmente no browser",
        },
        { feature: "Suporte Modrinth", gdl: "Sim", competitor: "Sim" },
        { feature: "Gestão automática de Java", gdl: "Sim", competitor: "Sim" },
        { feature: "Auto-update de mods", gdl: "Sim", competitor: "Não (apenas verificação manual)" },
        { feature: "Auto-update de modpacks", gdl: "Sim", competitor: "Não (apenas verificação manual)" },
        { feature: "Multi-instância", gdl: "Sim", competitor: "Sim" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sim (código de um clique, mix CF + MR)",
          competitor: "Não (export manual, sem mix CF + MR)",
        },
        {
          feature: "Gerenciamento de servidor",
          gdl: "Sim (embutido)",
          competitor: "Não",
        },
        { feature: "UI moderna", gdl: "Sim", competitor: "Não" },
        {
          feature: "Paga autores de addons",
          gdl: "Sim",
          competitor: "Não",
        },
        { feature: "Código no GitHub", gdl: "Sim", competitor: "Sim" },
        { feature: "Leve (RAM)", gdl: "Não", competitor: "Sim" },
      ],
      verdict:
        "Prism é excelente se você quer um launcher cru e leve e não se incomoda em ter mais trabalho manual com modpacks. O GDLauncher é para jogadores que querem instalação em um clique a partir do CurseForge e do Modrinth, Cloud Instance Sharing e gerenciamento de servidor embutido sem sair da app. Se você é novo no Minecraft modado ou valoriza acabamento acima do minimalismo, o GDLauncher é o caminho mais fácil.",
      sections: [
        {
          heading: "Fluxo de modpack",
          paragraphs: [
            "O Prism e o GDLauncher conseguem navegar e instalar packs CurseForge de dentro do launcher, então a experiência do dia a dia é parecida. O atrito aparece nas pontas: quando um autor de mod desativa o acesso via API de terceiros para o arquivo dele, o Prism pede que você clique em cada link bloqueado e baixe esses arquivos manualmente no browser. A parceria do GDLauncher com a CurseForge pega esses arquivos direto, então a instalação fica em um clique mesmo quando um pack tem mods bloqueados.",
            "Packs Modrinth funcionam igual nos dois launchers, navega de dentro do app e instala em um clique.",
          ],
        },
        {
          heading: "UI e descoberta",
          paragraphs: [
            "A UI baseada em Qt do Prism é funcional, mas utilitária; a visão principal é uma lista de instâncias. A UI do GDLauncher é feita especificamente para achar e gerenciar modpacks, com browser embutido, agrupamento de instâncias, drag-and-drop para reordenar e cards visuais. É subjetivo, mas vale comparar screenshots.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "O GDLauncher tem Cloud Instance Sharing em um clique: cola um código, e o mesmo setup aparece pronto. O Prism tem export/import de instância via arquivo, funciona, mas não é tão fluido para compartilhar com amigos.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App: comparação de recursos, anúncios, suporte ao Modrinth e gerenciamento de servidor. A melhor forma de jogar Minecraft modado.",
      intro:
        "O CurseForge App é o launcher oficial para conteúdo CurseForge. O GDLauncher também integra com o CurseForge e acrescenta Modrinth no mesmo browser, Cloud Instance Sharing entre as duas plataformas e gerenciamento de servidor embutido. Aqui vai o panorama.",
      rows: [
        {
          feature: "Suporte CurseForge",
          gdl: "Sim",
          competitor: "Sim (nativo, é o app deles)",
        },
        { feature: "Suporte Modrinth", gdl: "Sim", competitor: "Não" },
        { feature: "Gestão automática de Java", gdl: "Sim", competitor: "Sim" },
        { feature: "Auto-update de mods", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Auto-update de modpacks", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Multi-instância", gdl: "Sim", competitor: "Sim" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sim (código de um clique, mix CF + MR)",
          competitor: "Sim (só CurseForge)",
        },
        {
          feature: "Gerenciamento de servidor",
          gdl: "Sim (embutido)",
          competitor: "Não",
        },
        {
          feature: "Anúncios no app",
          gdl: "Sim (banner no app)",
          competitor: "Sim (banner no app)",
        },
        { feature: "Código no GitHub", gdl: "Sim", competitor: "Não" },
        { feature: "Paga autores de addons", gdl: "Sim", competitor: "Sim" },
      ],
      verdict:
        "Se você só instala conteúdo CurseForge, o CurseForge App é a escolha oficial. O GDLauncher entrega a mesma integração CurseForge, além de Modrinth no mesmo browser, Cloud Instance Sharing que viaja com setups mistos CurseForge + Modrinth, e gerenciamento de servidor embutido.",
      sections: [
        {
          heading: "Modrinth no mesmo launcher",
          paragraphs: [
            "O CurseForge App é, por design, só CurseForge. O Modrinth cresceu rápido, especialmente em mods Fabric, mods de performance e shaders, e muitos autores publicam agora nas duas plataformas. O browser embutido do GDLauncher busca nas duas ao mesmo tempo, então você não precisa escolher.",
          ],
        },
        {
          heading: "Gerenciamento de servidor",
          paragraphs: [
            "O GDLauncher inclui gerenciamento de servidor Minecraft embutido, crie um servidor Vanilla, Forge, Fabric, NeoForge ou Quilt e administre na mesma UI das suas instâncias singleplayer. O CurseForge App não inclui gerenciamento de servidor.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Os dois launchers conseguem compartilhar um setup com um amigo. O CurseForge App mantém tudo dentro do ecossistema CurseForge, dá pra passar um modpack do CurseForge, mas um setup que mistura mods do CurseForge com mods do Modrinth não viaja intacto. O Cloud Instance Sharing do GDLauncher aceita o caso misto: você cola um código e o destinatário recebe sua instância exata, com arquivos das duas plataformas rebaixados dos CDNs originais.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: qual launcher de Minecraft é melhor para mods e modpacks? Comparação de recursos, plataformas e suporte de ecossistema.",
      intro:
        "O Modrinth App é o launcher oficial do Modrinth e uma ótima escolha se você só usa conteúdo do Modrinth. O GDLauncher também integra com o Modrinth e ainda acrescenta o CurseForge, Cloud Instance Sharing e gerenciamento de servidor. Aqui o lado a lado.",
      rows: [
        {
          feature: "Suporte CurseForge",
          gdl: "Sim",
          competitor: "Não",
        },
        {
          feature: "Suporte Modrinth",
          gdl: "Sim",
          competitor: "Sim (nativo, é o app deles)",
        },
        { feature: "Gestão automática de Java", gdl: "Sim", competitor: "Sim" },
        { feature: "Auto-update de mods", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Auto-update de modpacks", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Multi-instância", gdl: "Sim", competitor: "Sim" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sim (código de um clique, mix CF + MR)",
          competitor: "Não (export manual, só Modrinth)",
        },
        {
          feature: "Gerenciamento de servidor",
          gdl: "Sim (embutido)",
          competitor: "Sim (Modrinth Hosting)",
        },
        { feature: "UI moderna", gdl: "Sim", competitor: "Sim" },
        { feature: "Código no GitHub", gdl: "Sim", competitor: "Sim" },
        { feature: "Paga autores de addons", gdl: "Sim", competitor: "Sim" },
        { feature: "Leve", gdl: "Médio", competitor: "Médio" },
      ],
      verdict:
        "O Modrinth App é fantástico se você vive inteiramente no ecossistema Modrinth. Mas muitos dos modpacks mais populares (RLCraft, ATM10, DawnCraft, a linha FTB) continuam exclusivos do CurseForge, e mesmo packs multiplataforma costumam sair primeiro no CurseForge. O GDLauncher dá Modrinth e CurseForge em um único browser, Cloud Instance Sharing para amigos e gerenciamento de servidor embutido. Escolha GDLauncher se quer o ecossistema mais amplo; escolha Modrinth App se quer uma experiência focada só no Modrinth.",
      sections: [
        {
          heading: "A lacuna do CurseForge",
          paragraphs: [
            "A maior diferença é direta: o Modrinth App não instala conteúdo do CurseForge. Para mods só de Modrinth, não importa. Mas o CurseForge continua hospedando a maior biblioteca de modpacks e muitos mods Forge antigos em exclusivo. O browser do GDLauncher mostra as duas plataformas em uma única busca, então você pega a que tem a versão que precisa.",
          ],
        },
        {
          heading: "Os dois ecossistemas são bons",
          paragraphs: [
            "O Modrinth tem uma biblioteca menor, mas um site mais rápido e sem anúncios, e APIs melhores para moders. O CurseForge tem o catálogo mais profundo e os packs históricos. A maioria dos mods populares está agora nos dois. A estratégia do GDLauncher é suportar os dois nativamente em vez de te forçar a escolher.",
          ],
        },
        {
          heading: "Gerenciamento de servidor e Cloud Instance Sharing",
          paragraphs: [
            "O gerenciamento de servidor do Modrinth é a integração paga Modrinth Hosting: você provisiona um servidor pela Modrinth e gerencia ele pela app. O gerenciamento de servidor do GDLauncher é local: você sobe um servidor Vanilla / Forge / Fabric / NeoForge / Quilt na sua própria máquina, com console ao vivo, gestão de jogadores e as mesmas configurações de instância que usa no singleplayer, sem conta de hosting.",
            "Cloud Instance Sharing é o outro recurso do GDLauncher que o Modrinth App não replica. Cola um código, recebe o setup exato com conteúdo misto CurseForge + Modrinth em um único compartilhamento.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher: comparação detalhada de UI, suporte a modpacks, gerenciamento de servidor e experiência de desenvolvedor. Qual é o melhor launcher de Minecraft?",
      intro:
        "O ATLauncher é um launcher de modpacks veterano baseado em Java, com seu próprio ecossistema de packs ATLauncher. O GDLauncher é a alternativa mais recente em Rust + Solid, com UI moderna e instalação em um clique a partir do CurseForge / Modrinth. Vamos comparar.",
      rows: [
        {
          feature: "Suporte CurseForge",
          gdl: "Sim",
          competitor: "Parcial (workaround)",
          note: "Quando um autor de mod desativa o acesso via API de terceiros, o ATLauncher pede que você baixe aquele arquivo manualmente no browser",
        },
        { feature: "Suporte Modrinth", gdl: "Sim", competitor: "Sim" },
        { feature: "Gestão automática de Java", gdl: "Sim", competitor: "Sim" },
        { feature: "Auto-update de mods", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Auto-update de modpacks", gdl: "Sim", competitor: "Sim (com confirmação)" },
        { feature: "Multi-instância", gdl: "Sim", competitor: "Sim" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sim (código de um clique, mix CF + MR)",
          competitor: "Não (export manual, sem mix CF + MR)",
        },
        { feature: "Gerenciamento de servidor", gdl: "Sim (embutido)", competitor: "Não" },
        {
          feature: "UI moderna",
          gdl: "Sim",
          competitor: "Parcial (Java Swing com FlatLaf)",
        },
        { feature: "Paga autores de addons", gdl: "Sim", competitor: "Não" },
        { feature: "Código no GitHub", gdl: "Sim", competitor: "Sim" },
        {
          feature: "Publicação de modpacks próprios",
          gdl: "Sim (via Cloud Instance Sharing, código de um clique)",
          competitor: "Sim (packs ATLauncher)",
        },
      ],
      verdict:
        "O ATLauncher é uma escolha sólida se você quer especificamente a lista curada de packs do ATLauncher ou já está acostumado ao fluxo dele. Os pontos fortes do GDLauncher são uma UI mais moderna, integração CurseForge mais profunda, Cloud Instance Sharing e gerenciamento de servidor embutido. Para a maioria dos jogadores de Minecraft modado em 2026, a experiência do GDLauncher é mais próxima do que se espera de um app moderno.",
      sections: [
        {
          heading: "Salto de geração de UI",
          paragraphs: [
            "O ATLauncher usa Java Swing com o visual moderno do FlatLaf por cima. É um avanço real em relação ao Swing clássico, mas ainda fica atrás dos launchers nativos modernos em densidade, animações e cara de plataforma. O GDLauncher é feito em Solid e usa um design system próprio em UnoCSS com drag-and-drop, animações e agrupamento que parecem nativos.",
          ],
        },
        {
          heading: "Integração com o CurseForge",
          paragraphs: [
            "O ATLauncher e o GDLauncher procuram e instalam packs do CurseForge de dentro do launcher, então o dia a dia é parecido. O atrito está nas bordas: quando um autor de mod desativa o acesso via API de terceiros para o arquivo, o ATLauncher te manda clicar em cada link bloqueado e baixar esses arquivos manualmente no browser. A parceria do GDLauncher com o CurseForge baixa esses arquivos direto, então a instalação continua em um clique mesmo quando o pack inclui mods bloqueados.",
          ],
        },
        {
          heading: "Packs ATLauncher vs Cloud Instance Sharing",
          paragraphs: [
            "O ATLauncher hospeda o próprio ecossistema de packs. O GDLauncher não compete nesse terreno, em vez disso, Cloud Instance Sharing deixa qualquer um compartilhar o setup exato (mods, configs, ajustes) com um único código. Filosofias diferentes; escolha o que combina com o jeito que você e seus amigos jogam.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC: comparação detalhada de recursos, automação, manuseio de modpacks e UI moderna. Encontre o launcher de Minecraft certo.",
      intro:
        "O MultiMC foi o pioneiro do launcher de Minecraft multi-instância, embora o último release oficial tenha sido o 0.6.14 em dezembro de 2021 e a maior parte do desenvolvimento ativo tenha migrado para os forks (com destaque para o Prism Launcher). O GDLauncher é um launcher moderno e com opinião, com forte automação. Aqui vai a comparação prática.",
      rows: [
        {
          feature: "Suporte CurseForge",
          gdl: "Sim",
          competitor: "Não",
        },
        { feature: "Suporte Modrinth", gdl: "Sim", competitor: "Sim" },
        { feature: "Gestão automática de Java", gdl: "Sim", competitor: "Não" },
        { feature: "Auto-update de mods", gdl: "Sim", competitor: "Não" },
        { feature: "Auto-update de modpacks", gdl: "Sim", competitor: "Não" },
        {
          feature: "Multi-instância",
          gdl: "Sim",
          competitor: "Sim (é a especialidade)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Sim (código de um clique, mix CF + MR)",
          competitor: "Não (export manual, sem mix CF + MR)",
        },
        {
          feature: "Gerenciamento de servidor",
          gdl: "Sim (embutido)",
          competitor: "Não",
        },
        { feature: "UI moderna", gdl: "Sim", competitor: "Não" },
        { feature: "Paga autores de addons", gdl: "Sim", competitor: "Não" },
        { feature: "Código no GitHub", gdl: "Sim", competitor: "Sim" },
        { feature: "Leve", gdl: "Não", competitor: "Sim (muito)" },
      ],
      verdict:
        "O MultiMC é uma ótima escolha se você quer um launcher minúsculo e super flexível e se sente à vontade fazendo seu próprio setup de Java, gerenciando mods e cuidando das atualizações. O GDLauncher é para jogadores que preferem essas coisas resolvidas automaticamente, Java auto, updates auto, instalação em um clique, Cloud Instance Sharing e gerenciamento de servidor, sem abrir mão do fluxo multi-instância que o MultiMC inaugurou.",
      sections: [
        {
          heading: "Automação vs controle",
          paragraphs: [
            "O design do MultiMC é \"não fazer nada que o usuário não pediu.\" Ou seja, você define o caminho do Java, escolhe a versão, gerencia os mods, atualiza tudo na mão. Power users amam isso. Jogadores novos desistem.",
            "O GDLauncher adota a abordagem oposta: detectar o que cada instância precisa, instalar, manter atualizado, mas expor todos os mesmos ajustes nas configurações de instância se você quiser sobrescrever algo. Os defaults funcionam; os controles continuam lá.",
          ],
        },
        {
          heading: "Manuseio de modpacks",
          paragraphs: [
            "O MultiMC tem um browser de Modrinth integrado, mas nenhuma integração com CurseForge. Para jogar packs do CurseForge, você precisaria importá-los manualmente como zip ou usar ferramentas de terceiros para puxar o manifest. O browser do GDLauncher mostra CurseForge e Modrinth lado a lado, com instalação em um clique nos dois.",
          ],
        },
        {
          heading: "O legado",
          paragraphs: [
            "O MultiMC não lança uma versão nova desde dezembro de 2021; a energia do projeto migrou de fato para o Prism Launcher e outros forks. Se você usa MultiMC há anos e quer uma UI mais moderna sem perder o fluxo, o Prism é o caminho natural de upgrade; o GDLauncher é o salto maior (mais automação, menos passos manuais). Experimente os dois e fique com o modelo que combina com como você realmente usa um launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Compartilhar um setup com um amigo no MultiMC significa exportar a instância pra um zip e passar o arquivo. Funciona, mas é um arquivo que você precisa hospedar em algum lugar, e o destinatário tem que importar do mesmo jeito. O Cloud Instance Sharing do GDLauncher substitui isso por um código curto: você cola, o launcher puxa o snapshot do serviço GDL, e os mods são rebaixados dos CDNs originais. Um código, conteúdo misto de CurseForge + Modrinth no mesmo share, sem zip pra passar pra ninguém.",
          ],
        },
      ],
    },
  },
}

export default ptBR
