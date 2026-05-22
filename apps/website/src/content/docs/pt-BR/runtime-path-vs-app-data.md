---
title: "Runtime Path vs App Data Path"
description: "O GDLauncher usa dois caminhos distintos para guardar dados. O que cada um contém, por que estão separados e qual normalmente vale a pena mover."
faq:
  - question: "Qual a diferença entre App Data Path e Runtime Path?"
    answer: "O App Data Path é o diretório padrão por usuário onde o Electron coloca caches e o marcador runtime_path_override. O Runtime Path é onde o core do GDLauncher guarda o pesado: instâncias, assets e bibliotecas do Minecraft, instalações de Java, banco de dados do launcher e logs globais. Por padrão o Runtime Path fica dentro do App Data Path, mas é o Runtime Path que foi pensado para ser movido."
  - question: "Onde fica o App Data Path no meu sistema?"
    answer: "Windows: C:\\Users\\<você>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<você>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, ou ~/.local/share/gdlauncher_carbon se XDG não estiver definida."
  - question: "Devo mover o Runtime Path ou o App Data Path?"
    answer: "O Runtime Path. É ele que cresce a cada instância nova. O App Data Path se mantém pequeno e amarrado ao perfil do SO. O GDLauncher expõe a mudança em Settings → Runtime Path; o App Data Path é gerenciado pelo Electron e não tem opção pela UI."
  - question: "Para que serve o arquivo runtime_path_override?"
    answer: "Quando você muda o Runtime Path, o GDLauncher escreve um arquivo de texto chamado runtime_path_override dentro do App Data Path. O conteúdo é o novo Runtime Path; a cada boot o launcher lê esse arquivo pra saber onde estão seus dados. Se sumir, o launcher volta pro Runtime Path padrão."
  - question: "Posso compartilhar o Runtime Path entre dois computadores?"
    answer: "Não. O banco do launcher contém estado por máquina (caminhos, tokens, Javas) e não foi pensado para uso simultâneo. Se quiser as mesmas instâncias em outro PC, copie manualmente ou use Cloud Instance Share."
---

# Runtime Path vs App Data Path

## Dois caminhos, dois papéis

O GDLauncher separa seus arquivos em dois lugares: um **App Data Path** pras coisas pequenas do lado do Electron, e um **Runtime Path** pro pesado (instâncias, assets, Java, banco de dados). O Runtime Path é o que de vez em quando você quer mover; o App Data Path, normalmente nunca toca.

### App Data Path

É o diretório app padrão por usuário do SO, pra onde o `userData` do Electron aponta. O GDLauncher usa pra:

- O marcador `runtime_path_override`, um arquivo de texto de uma linha que avisa o launcher onde o Runtime Path realmente fica
- O Runtime Path padrão, num subdiretório `data/`, se você não moveu
- Os caches Chromium do próprio Electron (Network/, GPUCache/, Cookies etc.)
- Os logs do processo principal do Electron

Local padrão por SO:

- **Windows:** `C:\Users\<você>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<você>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, ou `~/.local/share/gdlauncher_carbon` se não estiver definida

Sem o subdiretório `data/`, é geralmente pequeno. O GDLauncher não tem ajuste pra mover esse diretório; as convenções do SO e o Electron esperam ele ali.

### Runtime Path (Core Module)

Onde o core em Rust do GDLauncher coloca todo o resto:

- Suas instâncias (em `instances/`)
- Assets compartilhados do Minecraft (texturas, sons que a Mojang entrega)
- Bibliotecas compartilhadas (JARs da Mojang e dos mod loaders)
- Javas baixados pelo GDLauncher
- O banco do launcher, `gdl_conf.db`
- Logs globais em `__gdl_logs__/`

Por padrão fica em `<App Data Path>/data/`, dentro do App Data. Você pode apontar para qualquer lugar via **Settings → Runtime Path**. É o caminho que cresce, alguns modpacks grandes já passam de 50 GB.

## Quando mover o Runtime Path

Dois motivos comuns:

1. **Seu SSD está enchendo.** Mover instâncias para um HDD maior ou um SSD secundário.
2. **Quer backups separados do perfil do SO.** Colocar em um drive que você faz backup à parte é tranquilo; só não sincronize ativamente enquanto joga, launcher e ferramenta de sync vão brigar pelos arquivos.

Para uso normal, não precisa mover. O local padrão atende bem.

## Como é a migração

Abra **Settings → Runtime Path**. Digite o novo destino, ou use o ícone de pasta pra procurar. O botão aplicar (ícone de seta em círculo à direita da linha) acende assim que o caminho difere do atual e é válido. Clicar abre um modal de confirmação mostrando o caminho antigo e o novo.

Se a pasta destino estiver vazia (ou ainda não existir), confirmar dispara uma migração completa: um overlay mostra scan, depois cópia arquivo por arquivo, depois remoção arquivo por arquivo da origem. Não feche a app nem desligue a máquina enquanto isso roda. No fim o launcher reinicia sozinho.

Se o destino já contém um Runtime Path do GDLauncher (você moveu antes e quer que uma instalação nova aponte pra esses dados), o modal avisa em amarelo que a pasta não está vazia. Confirmar faz um "switch only": o marcador é reescrito apontando pros dados existentes, nada é copiado, e o launcher reinicia. Os dados no local antigo viram órfãos, dá pra apagar manualmente.

Se uma migração falhar no meio, o overlay fica vermelho e mostra o erro. O launcher faz rollback: arquivos criados no novo caminho são removidos e o marcador continua apontando pro caminho antigo, então dá pra tentar de novo sem perder nada. As duas causas comuns são permissão de escrita ausente no drive destino e falta de espaço livre.

### O marcador runtime_path_override

Ao mudar o Runtime Path, o GDLauncher escreve um arquivo de texto chamado `runtime_path_override` dentro do **App Data Path** (não dentro do Runtime Path). O conteúdo é o novo Runtime Path em texto puro. A cada boot a app lê esse arquivo pra saber onde estão seus dados.

Se apagar o marcador, o GDLauncher cai no Runtime Path padrão (`<App Data>/data/`). Os dados em si não somem, continuam onde você moveu, mas o launcher não vê até você ir em **Settings → Runtime Path** e apontar pra essa pasta de novo. Como a pasta já tem dados do GDLauncher, o launcher trata como "switch only" e só atualiza o marcador sem copiar nada.

## Por que não compartilhar o banco

O arquivo `gdl_conf.db` no Runtime Path contém tokens de conta, refresh tokens da Microsoft, estado da conta GDL e metadados de instância. Local por máquina e com credenciais sensíveis: **não compartilhe com ninguém.** Dois computadores com a mesma DB vão brigar pelos refresh tokens e os dois ficam deslogados.

Para as mesmas instâncias em outro PC, copie manualmente as pastas em `instances/`, ou use a funcionalidade Cloud Instance Share, feita para isso.
