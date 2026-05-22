---
title: "Compatibilidade do formato de save"
description: "Por que atualizar um mundo do Minecraft pra uma versão nova costuma ser sem volta, como o formato muda de verdade, e o jeito certo de fazer backup antes."
faq:
  - question: "Posso abrir um mundo do Minecraft 1.21 no 1.20?"
    answer: "Não com segurança. O Minecraft só migra mundos pra frente, nunca pra trás. Um mundo aberto no 1.21 tem level.dat e region files reescritos no formato novo; versões antigas se recusam a carregar ou crasham. Se você precisa dos dois, faça uma cópia do mundo antes de lançar a versão mais nova."
  - question: "O GDLauncher avisa antes de atualizar um mundo?"
    answer: "O launcher em si não, o aviso é do lado do Minecraft. Ao abrir um mundo salvo em uma versão mais antiga, o Minecraft mostra um diálogo 'Esse mundo foi salvo em uma versão diferente' antes de carregar. É o momento de voltar atrás e copiar a pasta do mundo pra outro lugar."
  - question: "O que é reescrito ao atualizar um mundo?"
    answer: "level.dat (metadados do mundo), os region files em region/ (dados de chunk), playerdata/ (estado por jogador) e qualquer data pack escopo mundo. O campo Data Version em level.dat sobe pra bater com a nova versão do Minecraft; é esse campo que versões mais novas/antigas leem pra decidir se podem abrir o mundo."
  - question: "Downgrade é impossível?"
    answer: "No sentido estrito, sim. Não existe caminho oficial pra downgrade. Algumas ferramentas comunitárias dizem reverter o Data Version mas não reescrevem chunks de verdade, então o mundo fica parcialmente corrompido (biomas, blocos, entidades novas que a versão antiga não entende). Trate atualizações como mão única."
  - question: "Como fazer backup de um mundo antes de atualizar?"
    answer: "Botão direito na instância no GDLauncher → Open Folder. Entre em instance/saves e copie a pasta do mundo (mesmo nome que aparece na lista) pra fora da pasta da instância. Guarde a cópia até ter certeza de que a versão atualizada funciona bem."
---

# Compatibilidade do formato de save

## Por que formatos de save mudam

O formato de arquivo dos mundos do Minecraft não é fixo. Cada update grande revisa a estrutura no disco. Blocos novos = IDs novas. Entidades novas = formas NBT novas. Biomas novos = registry de bioma novo. Por trás, cada mundo tem um número chamado **Data Version** em `level.dat`, e o Minecraft usa esse número pra decidir o que fazer ao abrir.

Se o Data Version do seu mundo é mais antigo que o da versão atual do Minecraft, ele executa um pass único de **DataFixer** que reescreve o mundo no formato novo. Chunks, entidades, estados de blocos, dados de jogador, tudo convertido pro novo schema. O Data Version em `level.dat` é atualizado pro valor novo.

Essa conversão é **destrutiva e mão única**. Uma vez que os chunks foram reescritos, a versão antiga do Minecraft não consegue mais ler.

## O que "mão única" significa na prática

Imagina um mundo 1.20.1. Você abre no 1.21. O Minecraft mostra o aviso "versão diferente", você clica "Converter" (ou carrega assim mesmo), o jogo inicia. Por trás:

- `level.dat` é reescrito pra que o campo `DataVersion` bata com o 1.21 em vez do 1.20.1.
- Todo region file em `region/` que é carregado (no mínimo tudo dentro da distância de visão) é reescrito chunk por chunk.
- Blocos novos do 1.21 como Crafter ou Trial Spawner agora podem existir no mundo; não existem no registry de blocos do 1.20.1.
- Entidades e tile entities existentes do 1.20.1 são migradas pros schemas do 1.21.

Se agora você tentar abrir a mesma pasta no 1.20.1:

- O Minecraft compara `DataVersion` com o dele e se recusa a carregar (ou crasha carregando certos chunks).
- Mesmo burlando o check de versão, blocos exclusivos do 1.21 apareceriam como blocos faltando/erro no cliente mais antigo.

Daí: **atualizar um mundo pra uma versão mais nova é permanente**. O único rollback seguro é restaurar de um backup feito *antes* da atualização.

## Mundos moddados pioram

O DataFixer do Minecraft vanilla pelo menos é exaustivo e bem testado. Saves moddados acrescentam uma camada de risco:

- Mods removidos deixam erros de **bloco faltando** e **entidade faltando**. O mundo carrega, mas cubos que eram blocos de mod viram placeholders "?".
- Mods substituídos (versão antiga → nova) às vezes mudam IDs de bloco ou chaves NBT de entidade. A migração depende do autor do mod e nem sempre é tranquila.
- Saltos grandes de versão Minecraft dentro de um modpack (Forge 1.20.1 → 1.21.x, por exemplo) costumam coincidir com a maioria dos mods migrando pra APIs totalmente novas. Mundos que rodavam na versão antiga podem ter comportamento indefinido na nova.

Em instâncias moddadas, trate qualquer salto de versão como potencial evento de corrupção e faça backup antes.

## Backupando um mundo direito

O backup mais simples é uma cópia de pasta. No GDLauncher:

1. Botão direito na instância → **Open Folder**.
2. Abra `instance/saves/`.
3. Copie a pasta com o nome do seu mundo (mesmo nome da lista) pra algum lugar fora da instância. Outro drive, uma pasta `~/Documents/mc-backups/`, qualquer canto que não vai ser sobrescrito.

Essa cópia é um snapshot do mundo no momento que copiou. Guarde até ter certeza que a nova versão tá funcionando.

Pra backups contínuos, ferramentas de terceiros como FTBBackups (um mod) tiram snapshots in-game em intervalos. Escrevem em `backups/` dentro da instância e são restauráveis por snapshot.

## O que avisos de "versão snapshot" significam

Se você abrir por acidente um mundo salvo em snapshot do Minecraft (build de desenvolvimento, tipo `24w11a`), o jogo oficial mostra um aviso adicional porque as Data Versions de snapshots às vezes estão à frente de qualquer versão lançada. Um mundo de snapshot pode não abrir no próximo estável se a snapshot fez mudanças de formato que foram revertidas antes do release. Caminho seguro: não jogar mundos importantes em snapshots, ou aceitar que o mundo fica preso à snapshot.

## TL;DR

- Atualizações de mundo são mão única; backup antes de abrir numa versão mais nova.
- Mundos moddados são mais frágeis; qualquer salto de versão é potencial evento de corrupção.
- Em updates de modpack que sobem a versão do Minecraft, copie a pasta saves toda primeiro, depois atualize.
