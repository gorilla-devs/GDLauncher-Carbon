---
title: "Anatomia de uma pasta de instância"
description: "O que tem dentro de uma pasta de instância do GDLauncher. Onde ficam mods, mundos, configs, prints e logs, e o que dá pra apagar à mão com segurança."
faq:
  - question: "Onde meus mundos de Minecraft são salvos?"
    answer: "Cada mundo fica em <runtime_path>/instances/<instance>/instance/saves/<nome-mundo>/. A pasta saves/ é criada na primeira vez que você gera ou importa um mundo. Botão direito na instância → Open Folder, depois entre em instance/saves. Pra backup, copie a pasta do mundo inteira."
  - question: "Onde estão os crash reports?"
    answer: "Dentro da instância: instance/crash-reports/. A pasta só existe se o Minecraft realmente travou; se você nunca viu o jogo morrer feio, ela ainda não está lá. Cada report é um arquivo de texto com timestamp (crash-<data>-server.txt, etc.)."
  - question: "Onde vão os JARs de mod?"
    answer: "instance/mods/ dentro da pasta da instância. Soltar um JAR aí à mão funciona, mas escapa do tracking do GDLauncher; prefira a aba Addons salvo motivo claro. Instâncias de modpack bloqueadas desativam o botão Add, mas cópia manual de arquivo passa."
  - question: "Qual a diferença entre os logs do launcher e os do Minecraft?"
    answer: "Dois conjuntos diferentes. Logs de sessão do launcher vivem em <instance>/logs/. Logs do jogo Minecraft (latest.log, crash-reports, tudo que o jogo escreve) ficam um nível abaixo, em <instance>/instance/logs/ e <instance>/instance/crash-reports/."
  - question: "O que são instance.json e packinfo.json?"
    answer: "Arquivos de metadata que o GDLauncher escreve no topo de cada instância. instance.json guarda nome, ícone, mod loader, versão do Minecraft, último momento jogado e tempo total. packinfo.json (só presente em instâncias pareadas com um modpack CurseForge ou Modrinth) rastreia quais arquivos pertencem ao pack pra distinguir mods do pack de mods adicionados na mão. Não apague nenhum deles na mão."
---

# Anatomia de uma pasta de instância

## Onde vive a pasta da instância

Cada instância do GDLauncher é uma subpasta do Runtime Path:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← a pasta da instância
```

`<shortpath>` é uma versão saneada do nome de exibição. Botão direito na instância no GDLauncher → **Open Folder** te leva direto.

## O que tem dentro

A pasta da instância se divide em alguns itens que o GDLauncher rastreia no topo, mais uma subpasta `instance/` que é o diretório real de jogo do Minecraft. Algumas coisas estão sempre lá; muitas são criadas sob demanda, na primeira vez que algo escreve nelas.

```
<shortpath>/
├── instance.json          ← metadata GDLauncher da instância (sempre presente)
├── packinfo.json          ← info de pareamento de modpack (só pra modpacks pareados)
├── icon.png | icon.webp   ← ícone custom (só se você setou um)
├── logs/                  ← logs do launcher GDLauncher por instância
└── instance/              ← diretório de jogo do Minecraft
    ├── mods/              ← JARs de mod
    ├── config/            ← configs de mod
    ├── shaderpacks/       ← shader packs (se você instalou algum)
    ├── options.txt        ← configurações do cliente Minecraft (após o primeiro launch)
    ├── logs/              ← logs de sessão do Minecraft (latest.log etc.)
    ├── saves/             ← mundos (criada quando você gera um)
    ├── screenshots/       ← prints F2 (criada no primeiro F2)
    ├── crash-reports/     ← crash dumps (só quando o jogo travou)
    ├── resourcepacks/     ← resource packs próprios (quando você adiciona um)
    ├── datapacks/         ← data packs globais (por mundo ficam em saves/<mundo>/datapacks/)
    └── (específico do pack) ← kubejs/, defaultconfigs/, packmenu/ etc., só se o modpack traz
```

Não se assuste se algumas dessas faltarem numa instância nova. O launcher e o Minecraft só criam o que precisam, quando precisam. Uma instância nunca jogada não tem `saves/`, nem `screenshots/`, nem `options.txt`. Uma instância vanilla não tem `mods/` nem `config/`.

## O que cada coisa tem

### Arquivos do topo

- **`instance.json`**: metadata do GDLauncher, nome, caminho do ícone, mod loader e versão, versão do Minecraft, quando foi criada, último momento jogado, tempo total. Sempre presente.
- **`packinfo.json`**: manifest de hashes dos arquivos vindos do modpack. Permite ao launcher distinguir mods do pack dos que você adicionou. Só presente em instâncias pareadas com um modpack CurseForge ou Modrinth.
- **`icon.png`** ou **`icon.webp`**: o ícone custom que você subiu. Ausente se está usando o padrão.

### `logs/` (topo)

Logs próprios do GDLauncher por instância. É o que a ação **View Logs** do menu de contexto mostra. Cobrem o lançamento sob a ótica do *launcher* (argumentos Java, downloads de asset, instalação do mod loader, código de saída); úteis quando o jogo nem chega a escrever o próprio log.

### `instance/mods/`

Os arquivos JAR dos mods. O Minecraft carrega tudo aqui no startup (de acordo com as regras do mod loader). O launcher rastreia quais mods pertencem à instância em seu banco de dados (com nome do arquivo e hash do conteúdo como chave), não há arquivos sidecar. JARs jogados à mão também são reconhecidos; o launcher só não tem metadata do CurseForge/Modrinth pra eles.

### `instance/config/`

Uma subpasta ou arquivo por mod. Aqui ficam as configurações dos mods. A maioria escreve `config/<modid>.toml` ou uma pasta `config/<modid>/`. Editar à mão costuma ser seguro; muitos mods releem no restart.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Asset packs. Resource packs pra texturas e sons, shader packs pra renderização (precisam de Iris/OptiFine instalado como mod), data packs pra adicionar receitas/loot/funções. Data packs específicos de um mundo vivem em `saves/<mundo>/datapacks/`. Essas pastas só são criadas quando você tem conteúdo nelas de verdade.

### `instance/saves/`

Uma subpasta por mundo. Dentro: `level.dat` (mestre), `region/` (dados de chunk), `playerdata/` (estado por jogador), `datapacks/` (data packs escopo mundo). Pra backup, copie toda a pasta `<mundo>/`. A `saves/` em si aparece na primeira vez que você gera um mundo.

### `instance/screenshots/`

Tudo que você capturou com F2 in-game. PNGs nomeados por timestamp. Criada na primeira vez que você tira um print.

### `instance/logs/` e `instance/crash-reports/`

A saída de diagnóstico do Minecraft. `logs/latest.log` é sempre o último lançamento (rotacionado pra `logs/<data>-1.log.gz` no próximo). `crash-reports/` guarda crash dumps completos e só aparece uma vez que houve um crash de verdade.

### `instance/options.txt`

Configurações do cliente Minecraft (gráficos, controles, som). Texto puro, key=value. Editável se você insistir.

### Pastas específicas de modpack

Muitos modpacks grandes trazem pastas extras. As mais comuns:

- **`kubejs/`**: scripts KubeJS (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Autores de pack usam pra ajustes em runtime.
- **`defaultconfigs/`**: snapshot de "como as configs deveriam parecer por padrão". O script de boot do pack copia entradas faltantes pra `config/` a cada start.
- **`packmenu/`**: assets de menu principal temáticos do pack (botões custom, fundos, splash text).
- **`defaultsettings/`**: parecido com `defaultconfigs/`, mas pra `options.txt` e teclas.

Só existem se o pack traz. Vanilla e a maioria das instâncias custom não têm nenhuma.

## O que dá pra apagar

| Pasta | Apagar é seguro? | Efeito |
|---|---|---|
| `instance/mods/` (um JAR específico) | Sim | Esse mod sai. Mundos que o usavam podem quebrar. |
| `instance/config/<modid>/` | Sim | Mod reseta pra defaults na próxima execução. |
| Conteúdo de `instance/resourcepacks/`, `instance/shaderpacks/` | Sim | O pack sai. |
| `instance/saves/<mundo>/` | Sim | Mundo apagado pra sempre. Backup antes. |
| `instance/logs/`, `crash-reports/` | Sim | Só libera disco. |
| `instance/screenshots/` | Sim | Tchau prints antigos. |
| `logs/` (logs do launcher) | Sim | Idem. |
| `instance/options.txt` | Sim | Configurações do jogo voltam ao default. |
| `instance.json` | Não | O launcher perde o rastro da instância. |
| `packinfo.json` | Possível (efetivamente desemperelha) | O launcher deixa de tratar a instância como modpack pareado. Mesmo efeito que o botão Unpair na aba Settings da instância, só que sujo. |
| Toda a pasta `instance/` | Não | A instância fica quebrada. Use Delete na UI do launcher. |

## Nota sobre instâncias de modpack bloqueadas

A pasta da instância é só uma pasta normal no disco; o bloqueio que o GDLauncher aplica a instâncias de modpack vive na UI, não no filesystem. Soltar um JAR em `instance/mods/` de uma instância bloqueada funciona mecanicamente, o launcher só não saberá disso via a aba Addons. Pra remover, precisa voltar pelo filesystem. Pra fluxos mais seguros, abra a instância, vá na aba **Settings** e clique **Unlock** na seção Modpack.
