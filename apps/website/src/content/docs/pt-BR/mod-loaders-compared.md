---
title: "Mod loaders comparados: Forge, NeoForge, Fabric, Quilt"
description: "O GDLauncher suporta quatro mod loaders do Minecraft. O que cada um é, em que diferem, e qual escolher para um mod ou modpack."
faq:
  - question: "Qual mod loader usar no Minecraft?"
    answer: "O que o mod ou modpack que você quer exigir, na prática isso quase sempre decide. Se for escolher do zero: Fabric pra mods de performance/QoL nas versões mais novas, NeoForge pra grandes mods de conteúdo modernos, Forge pra modpacks antigos e a maior biblioteca histórica."
  - question: "Mods Forge funcionam no Fabric?"
    answer: "Não. Forge e Fabric não são intercambiáveis. Um mod feito pra um não carrega no outro. Muitos mods populares têm builds separadas Forge e Fabric; veja a página do mod pra saber loaders e versões suportadas."
  - question: "NeoForge é substituto do Forge?"
    answer: "Na prática sim pra versões novas de Minecraft. NeoForge começou em 2023 como fork do Forge com a mesma API; os dois divergiram desde então, então um mod atual costuma publicar uma build NeoForge separada em vez de rodar nos dois. A partir do 1.20.4, muitos mods Forge agora são buildados pra NeoForge. Em 1.20.1 e anteriores, Forge continua sendo o padrão."
  - question: "Mods do Fabric rodam no Quilt?"
    answer: "A maioria sim. Quilt é um fork do Fabric e executa mods Fabric direto. Alguns mods só de Quilt usam APIs do Quilt e não rodam no Fabric. Se sua lista de mods é toda Fabric, qualquer um dos dois loaders dá o mesmo resultado."
  - question: "Dá pra usar dois mod loaders ao mesmo tempo?"
    answer: "Na mesma instância não. Cada instância pega exatamente um loader. Pra usar os dois, crie duas instâncias. O sistema de instâncias do GDLauncher foi feito pra isso: uma Forge, uma Fabric, troca com um clique."
---

# Mod loaders comparados: Forge, NeoForge, Fabric, Quilt

## Os quatro mod loaders que o GDLauncher suporta

O GDLauncher instala e roda qualquer um dos quatro grandes mod loaders do Minecraft Java Edition, além de vanilla (sem loader). Ao criar uma instância custom você escolhe um. Em modpack, o loader é o que o manifest do pack indicar.

### Forge

O mod loader original, começou em 2011. Forge tem a maior biblioteca histórica de mods, especialmente os pesados em conteúdo (árvores de tech, sistemas de magia, mundos novos, como Tinkers' Construct, Twilight Forest, Create em versões antigas). É também o alvo da maioria dos modpacks antigos.

Forge atualiza mais lento que Fabric. Novas versões Minecraft costumam ter release Forge semanas ou meses depois.

### NeoForge

Um fork de 2023 do Forge, criado após uma rachadura na comunidade Forge. NeoForge mantém o estilo de API do Forge (mods são em geral source-compatíveis) mas lança mais rápido, e boa parte do desenvolvimento de mods Forge migrou pra ele.

A partir do Minecraft 1.20.4, NeoForge é o mais ativo dos dois. Muitos mods grandes hoje publicam builds NeoForge em paridade com Forge ou no lugar do Forge.

### Fabric

Filosofia diferente: pequeno, rápido, modular. Fabric sai praticamente no dia que uma nova versão de Minecraft é lançada, às vezes em horas. O ecossistema mod tende a performance (Sodium, Lithium, FerriteCore), QoL (Mod Menu, Iris) e mods de conteúdo moderno de alta qualidade.

Se performance é prioridade ou você joga numa versão Minecraft bleeding-edge, Fabric é o loader.

### Quilt

Fork de 2022 do Fabric com modelo de governança diferente e algumas APIs extras. Quilt executa mods Fabric direto, então a diferença prática é pequena: use Quilt se um mod específico exigir, senão Fabric dá no mesmo.

Quilt tem um ecossistema dedicado menor que o Fabric mas é quase totalmente compatível com conteúdo Fabric.

## Matriz de compatibilidade

| Mod feito para | Roda no Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Sim | Às vezes (o NeoForge inicial conseguia rodar mods Forge sem mexer porque era um fork recém-criado; as APIs divergiram desde então, e a maior parte dos mods Forge atuais precisa de uma build NeoForge) | Não | Não |
| NeoForge | Não | Sim | Não | Não |
| Fabric | Não | Não | Sim | Sim |
| Quilt | Não | Não | Mods de Quilt-API: não; resto: sim | Sim |

Não existe ponte cross-loader em produção. Os JARs que você põe em `mods/` precisam casar com o loader da instância.

## Escolher pra uma instância nova

Normalmente os mods ou o modpack escolhem por você:

- **Instalando um modpack do CurseForge ou Modrinth?** O GDLauncher lê o manifest e instala o loader indicado. Sem escolha.
- **Montando uma instância custom em torno de um mod só?** Veja a página do mod. Se disser "Fabric 1.21.x", crie uma instância Fabric 1.21.x.
- **Montando uma custom em torno de uma lista de mods?** Pra cada mod, veja quais loaders ele suporta, pegue a interseção. A maioria dos mods de performance é Fabric-only; os grandes de conteúdo, Forge/NeoForge.

Sem restrição, recomendação: **Fabric** pra setup focado em performance/visual, **NeoForge** pra survival modded de conteúdo pesado.

## Trocar o loader numa instância existente

O GDLauncher permite trocar o mod loader de uma instância depois de criada, ver [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). Resumo: botão direito na instância → Edit → escolha outro loader. A pasta mods não é limpa, então JARs do loader antigo ficam; remova à mão os incompatíveis antes de lançar.

## Nota sobre versões de loader

Cada loader tem o próprio versionamento, independente do Minecraft. Quando você escolhe "Forge", também escolhe uma versão Forge (tipo `47.2.0` pro Minecraft 1.20.1). Pra mods, a versão do loader raramente importa além de "mesma major que o pack espera", mas alguns exigem um build mínimo. A página CurseForge ou Modrinth do mod diz.
