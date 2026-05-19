---
title: "Formato do manifest de um modpack"
description: "O que tem dentro dos três arquivos de modpack que o GDLauncher lê e escreve: CurseForge .zip, Modrinth .mrpack e o próprio .gdlpack do GDLauncher baseado em hashes. Referência campo a campo, comparativo lado a lado e como cada formato resolve mods na instalação."
faq:
  - question: "O que é um manifest de modpack?"
    answer: "Um arquivo JSON dentro do arquivo de modpack que diz pro launcher o que tem no pack: versão do Minecraft, mod loader, lista de mods e quais arquivos embalados (overrides) extrair na instância. O GDLauncher lê o manifest antes de qualquer coisa quando instala um pack."
  - question: "Qual é a diferença entre os formatos CurseForge, Modrinth e GDLauncher?"
    answer: "Os três são zips com um manifest JSON. CurseForge usa manifest.json com IDs de projeto/arquivo do CurseForge. Modrinth usa modrinth.index.json com URLs diretas de download mais hashes. GDLauncher usa gdlpack.json só com hashes de conteúdo (sha512 + sha1 + murmur2) e resolve cada arquivo contra a plataforma que efetivamente o tem. Não são intercambiáveis; o launcher decide pelo JSON que encontra na raiz do arquivo."
  - question: "O que é um .gdlpack e por que ele existe?"
    answer: "O formato de modpack próprio do GDLauncher. Cada mod é identificado por hashes de conteúdo, então um único pack funciona quer os mods vivam no CurseForge, no Modrinth ou em ambos, sem URLs envelhecendo. Também tem suporte de primeira classe a recursos opcionais (pacotes puláveis de mods + configs) e ícones embutidos. Recomendado quando os dois lados usam GDLauncher; CurseForge .zip continua a escolha mais segura pra distribuição entre launchers."
  - question: "Onde encontro o manifest em uma instância instalada?"
    answer: "O GDLauncher não guarda o arquivo original depois da instalação. O conteúdo do pack acaba descompactado dentro da pasta da instância. O manifest com o qual o GDLauncher trabalha é guardado no banco de dados do launcher contra a instância; não dá pra navegar como arquivo. Pra inspecionar um manifest antes de instalar, abra o .zip / .mrpack / .gdlpack com qualquer ferramenta de arquivo e leia o JSON de dentro diretamente."
  - question: "Posso editar um manifest?"
    answer: "Você pode editar um dentro de um arquivo se estiver construindo seu próprio pack, mas não dá pra editar o manifest de uma instância instalada pela UI do launcher. Pra mudar quais mods uma instância tem, use a aba Addons (ou destrave uma instância travada antes). Pra mudar a versão de um pack pareado, abra a instância, vá na aba Settings e clique em Change Modpack Version."
  - question: "O que é um 'override' no jargão de packs?"
    answer: "Arquivos que o pack já vem com, não mods. Configs padrão, scripts, resource packs, às vezes um mundo inicial. São extraídos por cima dos mods na pasta da instância quando o pack instala. CurseForge lista no campo overrides; Modrinth marca arquivos do pack com env.client=required; GDLPack usa uma pasta overrides (configurável via manifest)."
---

# Formato do manifest de um modpack

## Por que importa

Normalmente você não precisa saber o que tem dentro de um arquivo de modpack. Mas quando algo dá errado na instalação você vê mensagens tipo "manifest", "invalid manifest format", "manifest missing field" ou "manifest version unsupported", e quer entender a que se referem. Esta página também é útil se estiver construindo um pack pra compartilhar ou tentando descobrir por que o mesmo pack instala limpo em um launcher e quebra em outro.

O GDLauncher lê e escreve três formatos:

- **CurseForge** (`.zip`, com `manifest.json` dentro).
- **Modrinth** (`.mrpack`, com `modrinth.index.json` dentro).
- **GDLauncher** (`.gdlpack`, com `gdlpack.json` dentro).

Os três são arquivos ZIP comuns por baixo do capô; a extensão só dá uma dica de qual esquema de manifest esperar.

## O que tem dentro de um arquivo de modpack

A forma geral é a mesma nos três formatos:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← manifest específico do formato
├── overrides/            ← configs, scripts, mundo inicial opcional
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (só CurseForge, opcional, lista de mods legível)
```

O manifest é o único arquivo que o launcher precisa estritamente pra saber instalar o pack. A pasta overrides é conteúdo pra descompactar na instância resultante.

## CurseForge: `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

Campos principais:

- `minecraft.version`: a versão do Minecraft que o pack mira.
- `minecraft.modLoaders`: qual loader e qual versão (formato: `<loader>-<version>`).
- `files`: cada mod é referenciado por `projectID` e `fileID` do CurseForge. O GDLauncher resolve contra a API do CurseForge pra baixar.
- `overrides`: nome da pasta dentro do zip cujo conteúdo é copiado pra instância após terminar os downloads de mods.

Se o `projectID` ou `fileID` de um arquivo não existe mais no CurseForge (o autor removeu), a instalação falha com erro "file not found" pra aquele mod específico.

## Modrinth: `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

Principais diferenças em relação ao CurseForge:

- `dependencies` declara as versões do Minecraft e do loader diretamente (sem objeto aninhado).
- Cada entrada em `files` inclui a **URL exata de download** e o **hash**, então as instalações não dependem de chamada de API de metadados.
- Cada arquivo declara se é requerido no cliente, servidor ou ambos via `env`.

Esse formato é mais simples e mais fácil de instalar offline (URLs já vêm embutidas). É também mais estrito: mismatch de hash significa recusa dura de instalação, não aviso.

## GDLauncher: `gdlpack.json`

O formato próprio de pack do GDLauncher é o que o GDLauncher escreve quando você escolhe **GDLauncher .gdlpack** em **Export Instance**. A propriedade definidora: cada mod é identificado apenas pelos hashes de conteúdo, não por ID específico de plataforma ou URL. O GDLauncher resolve cada hash contra CurseForge e Modrinth na hora de instalar e baixa da plataforma que de fato tiver o arquivo.

### Por que hashes em vez de IDs ou URLs

- **Multiplataforma a partir de uma fonte só.** Um mod que vive tanto no CurseForge quanto no Modrinth tem IDs diferentes de cada lado. Um `.gdlpack` não liga; uma lista de hashes funciona pros dois.
- **Sem apodrecimento de URL.** As URLs do CDN do Modrinth são endereçadas por conteúdo e estáveis, mas embutir URLs amarra a uma única origem de download. Hashes deixam o launcher cair em fallback quando uma plataforma está fora.
- **Verificação por construção.** Cada byte gravado na sua pasta `mods/` é checado contra o hash do manifest. Não dá pra substituir um JAR diferente silenciosamente.

### Estrutura do arquivo

```
mypack.gdlpack/
├── gdlpack.json          ← o manifest
├── .gdl/
│   └── icon.png          ← ícone do pack embutido (opcional)
└── overrides/            ← arquivos empacotados (configs, scripts, mods não resolvíveis)
    ├── config/
    └── ...
```

O manifest fica na raiz. A pasta `.gdl/` guarda metadados que o launcher embute (atualmente só o ícone). Os overrides funcionam como no CurseForge: o conteúdo é extraído na instância resultante após o passo de resolução de mods.

### Manifest mínimo

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

Apenas `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries` e `overrides` são obrigatórios pra carregar.

### Manifest completo, anotado

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### Campos de topo

| Campo | Tipo | Obrigatório | Significado |
|---|---|---|---|
| `formatVersion` | integer | sim | Versão do esquema do manifest. Atualmente sempre `1`. |
| `name` | string | sim | Nome de exibição do pack. Serve como nome padrão da instância na importação. |
| `version` | string | não | Versão do pack (semver recomendado, ex.: `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | não | Tagline de uma linha. |
| `author` | string | não | Autor ou equipe do pack. |
| `createdAt` | timestamp RFC 3339 | sim | Quando o arquivo foi exportado. |
| `icon` | string | não | Caminho dentro do arquivo pro arquivo de ícone (ex.: `.gdl/icon.png`). |
| `dependencies` | object | sim | Requisitos de Minecraft e modloader (ver abaixo). |
| `entries` | array | sim | Arquivos de plataforma e recursos opcionais (ver abaixo). Array vazio é válido pra pack só de overrides. |
| `overrides` | string | sim (padrão `"overrides"`) | Nome do diretório dentro do arquivo a extrair na instância. |
| `serverOverrides` | string | não | Diretório de overrides só de servidor. |
| `clientOverrides` | string | não | Diretório de overrides só de cliente. |
| `source` | object | não | Se este pack deriva de um pack CurseForge ou Modrinth, aponta de volta pro original (ver abaixo). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: versão do Minecraft requerida (ex.: `1.20.1`, `1.21.4`, `25w20a` pra snapshots).
- `modloaders`: zero ou mais entradas de loader. Cada uma tem:
  - `type`: um entre `forge`, `neoforge`, `fabric`, `quilt` (em minúsculas).
  - `version`: versão exata do loader contra a qual o pack foi construído.
  - `primary`: marca o loader recomendado quando vários são listados (ex.: um pack compatível com Fabric e Quilt).

Um pack vanilla tem array `modloaders` vazio.

#### `entries`

Dois tipos de entrada, discriminados pelo campo `type`:

**`platform`**, um mod obrigatório resolvido via hash:

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

Todos os três hashes precisam estar presentes. Cada um é usado de forma diferente na resolução:

| Hash | Usado para |
|---|---|
| `sha512` | Resolução Modrinth (endpoint `version_file` do Modrinth), verificação primária de integridade do arquivo baixado. |
| `sha1` | Resolução Modrinth (fallback), usado pelo próprio Minecraft ao verificar o arquivo no lançamento. |
| `murmur2` | Resolução CurseForge (endpoint `fingerprints` do CurseForge). |

Na instalação, o launcher tenta primeiro Modrinth (consulta sha512), recai pro CurseForge (fingerprint murmur2), e falha a entrada se nenhuma plataforma tiver o arquivo. Arquivos resolvidos sempre baixam da plataforma que respondeu.

**`optional`**, um grupo pulável de mods e/ou caminhos de overrides que o usuário pode incluir ou excluir:

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description`: mostrado ao usuário na pré-visualização de importação pra ele decidir se inclui esse recurso.
- `platforms`: zero ou mais entradas de hash que só são baixadas se o recurso for incluído.
- `overridePaths`: zero ou mais caminhos (arquivos ou pastas) dentro do diretório `overrides/` que só são extraídos se o recurso for incluído. Caminhos são relativos a `overrides/`.

Um recurso pode ter só arquivos de plataforma, só overrides, ou os dois. Serve pra empacotar conteúdo opcional relacionado: um mod de shader mais sua config, um preset hardcore que são só configs, um resource pack opcional.

#### `source`

Quando um pack foi exportado de uma instância de modpack CurseForge ou Modrinth existente, o GDLauncher registra a origem:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

ou

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

É informativo; o launcher não usa pra baixar nada (o array `entries` já cobre isso). Existe pra que derivados de packs públicos continuem atribuíveis ao original.

### Exports com bundle vs só manifest

Ao exportar uma instância pra `.gdlpack`, o toggle **Bundle Addons** no assistente de exportação decide o que vai pra onde:

- **Bundle desligado (só manifest).** Cada mod que o GDLauncher consegue resolver via CurseForge ou Modrinth vira uma entrada `platform` em `entries`. O JAR de fato *não* é incluído no arquivo. O launcher do destinatário rebaixa da plataforma na importação. Mods que o GDLauncher não consegue resolver (ex.: JARs colocados manualmente) são empacotados em `overrides/mods/` como fallback. Resultado: arquivo pequeno, o destinatário precisa de internet na instalação.
- **Bundle ligado (autocontido).** Nenhuma entrada `platform` é emitida; cada mod é copiado pra `overrides/mods/`. Resultado: arquivo maior (geralmente centenas de MB a vários GB), instalável offline assim que os assets do Minecraft estiverem em cache.

Resource packs e shader packs se comportam da mesma forma: os resolvíveis viram entradas `platform` com bundle desligado, tudo é empacotado direto com bundle ligado.

## Lado a lado: os três formatos

| Propriedade | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Nome do arquivo de manifest | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Identificação de mod | Project + File ID do CurseForge | URL de download + hash | Só hashes de conteúdo |
| Resolve a partir de | API do CurseForge | URL embutida no manifest | CurseForge **ou** Modrinth, qual responder |
| Instalação offline | Não (precisa de CDN) | Sim (URLs ainda podem precisar de CDN) | Sim quando `Bundle Addons` está ligado |
| Recursos opcionais | Flag `required: false` por mod | `env` por arquivo (client/server) | Entrada `optional` dedicada com vários arquivos |
| Pasta de overrides | `overrides/` | caminho por arquivo | `overrides/` (configurável) |
| Arquivos só de servidor | Pack separado | `env.server` | Diretório `serverOverrides` |
| Ícone | Externo | Externo | Embutido em `.gdl/icon.<ext>` |
| Rastreamento de origem | Nenhum | Nenhum | Campo `source` |
| Portabilidade | Mais ampla (a maioria dos launchers lê) | Launchers compatíveis com Modrinth | GDLauncher |

## Overrides, em detalhe

Quando o manifest referencia uma pasta de overrides ou arquivos de pack marcados como client-required, o launcher os extrai pra dentro da instância após o passo de resolução de mods. É assim que os packs empacotam:

- Configs padrão de mods (pra que o pack jogue igual logo de cara pra todo mundo).
- Scripts KubeJS ou CraftTweaker.
- Um mundo inicial (ocasionalmente).
- Resource packs que o pack espera ativos.
- `options.txt` com ajustes de jogo afinados pelo pack.

Overrides vencem padrões autogerados mas perdem pra qualquer coisa que o jogador mude manualmente depois. O GDLPack ainda suporta diretórios `serverOverrides` e `clientOverrides` pra arquivos que devem cair apenas em um lado, útil pra packs feitos pra entregar um bundle de cliente e servidor combinados.

## Quando manifests ficam desatualizados

Um manifest de pack é uma fotografia do que o autor publicou por último. Se um mod referenciado pelo manifest for removido da plataforma de origem depois que o manifest foi construído, a instalação daquela versão do pack vai falhar até o autor publicar uma nova. É o que acontece quando uma versão antiga do pack instala limpa mas uma mais nova quebra: a mais nova referencia algo que não está mais disponível.

A correção é do lado do autor; o launcher só pode fazer o que o manifest diz. O GDLPack tem uma vantagem parcial aqui porque não está preso a uma plataforma: se um mod é retirado do CurseForge mas continua no Modrinth (ou o contrário), o mesmo array `entries` ainda resolve.

## Ler um manifest você mesmo

Não precisa de nada especial. Renomeie `mypack.mrpack` ou `mypack.gdlpack` pra `mypack.zip`, abra com qualquer ferramenta de arquivo e olhe o arquivo JSON dentro (`modrinth.index.json` ou `gdlpack.json`). Mesmo com `.zip` do CurseForge e `manifest.json`. Os três são JSON simples, legível em qualquer editor de texto.

## Construir um pack

Se você está construindo um pack pra compartilhar, o caminho mais fácil é montar a instância no GDLauncher e usar **Export Instance** (clique direito → Export Instance). O assistente de exportação te deixa escolher o formato de destino: CurseForge `.zip`, Modrinth `.mrpack` ou o `.gdlpack` próprio do GDLauncher. Escolha `.gdlpack` se o destinatário também usa GDLauncher e você quer recursos opcionais ou ícones embutidos; escolha `.zip` pra compatibilidade mais ampla entre launchers.
