---
title: "Solução de problemas"
description: "Resolva problemas comuns ao iniciar o GDLauncher e o Minecraft. Caminho de dados, caminho de runtime, localização dos logs e soluções comprovadas."
faq:
  - question: "Onde o GDLauncher armazena seus dados?"
    answer: "No Windows: C:\\Users\\<você>\\AppData\\Roaming\\gdlauncher_carbon. No macOS: /Users/<você>/Library/Application Support/gdlauncher_carbon. No Linux: $XDG_DATA_HOME/gdlauncher_carbon (ou ~/.local/share/gdlauncher_carbon se o XDG não estiver definido)."
  - question: "Onde ficam os logs do GDLauncher?"
    answer: "O GDLauncher grava dois logs no nível do app em arquivos diferentes: main.log (Electron) na pasta App Data, e arquivos <timestamp>.log com carimbo de data/hora na pasta __gdl_logs__ do caminho de runtime (Rust core; os 10 mais recentes são mantidos). Ao reportar problemas, envie os dois. Os caminhos exatos estão no guia Share App Logs."
  - question: "O GDLauncher não abre. O que faço?"
    answer: "Primeiro, confira os logs na pasta de dados em busca de algum erro. Causas comuns: runtime corrompido, antivírus bloqueando o executável ou uma atualização aplicada parcialmente. Reinstalar o GDLauncher do zero e restaurar as instâncias geralmente resolve ambos os casos."
  - question: "Por que meu modpack trava ao iniciar?"
    answer: "A maioria das travadas ao iniciar vem de incompatibilidade entre versão do Minecraft, mod loader e mods. Veja o arquivo mais recente em __gdl_logs__ para encontrar o erro. Se um mod específico for citado, normalmente é o culpado, desative-o na aba Addons e tente de novo. Se for OutOfMemoryError, aumente a RAM nas configurações da instância."
  - question: "Como movo o GDLauncher para outro disco ou pasta?"
    answer: "Abra Configurações → Geral → Caminho do runtime. Altere para o novo local e o GDLauncher migrará suas instâncias e downloads automaticamente. A migração é executada uma única vez na próxima inicialização."
  - question: "Posso usar o GDLauncher offline?"
    answer: "Você pode jogar offline em instâncias já instaladas. A autenticação ainda exige conexão online ao menos uma vez (conta Microsoft), e o download de novos mods ou modpacks precisa de internet."
---

## Caminho de dados do app

É o caminho onde o GDLauncher armazena os dados do Electron e, por padrão, o Caminho de Runtime do Core Module.

### Windows

`C:\Users\\{{Seu nome de usuário}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Seu nome de usuário}}/Library/Application Support/gdlauncher_carbon`

### Linux

- se a variável de ambiente `$XDG_DATA_HOME` estiver disponível: `$XDG_DATA_HOME/gdlauncher_carbon`
- caso contrário: `{{homedir}}/.local/share/gdlauncher_carbon`

[Mais detalhes sobre homedir](https://nodejs.org/api/os.html#oshomedir)

## Caminho de runtime do Core Module

É o caminho onde o Core Module armazena todos os seus dados, incluindo todas as instâncias, assets e bibliotecas.
Geralmente fica no mesmo local do caminho de dados do app, dentro da pasta `data`, a menos que você defina explicitamente outro local.

### Banco de dados do app

O banco de dados do app fica no caminho de runtime do Core Module e é um arquivo SQLite chamado `gdl_conf.db`.

**NÃO ENVIE ESTE ARQUIVO PARA NINGUÉM, ELE CONTÉM DADOS SENSÍVEIS.**

### Logs do app

O GDLauncher grava dois logs no nível do app em arquivos diferentes. No suporte, **sempre envie os dois**, as duas metades do launcher trocam trabalho entre si, e a causa de uma falha em um lado costuma aparecer no log do outro.

- **`main.log`** no App Data Path: o log do processo principal do Electron. Cobre criação de janela, IPC, auto-update, diálogos nativos e crashes duros do shell desktop.
- **`__gdl_logs__/<timestamp>.log`** no Core Module Runtime Path: o log do Rust core. Cobre login, downloads de assets, instalação de mod loaders, execuções de instância, mudanças de settings. Os 10 mais recentes são mantidos.

Caminhos por SO e prints no guia [Share App Logs](/guides/share-app-logs).

**OS LOGS PODEM CONTER DADOS SENSÍVEIS. TENHA CUIDADO AO COMPARTILHÁ-LOS.**

### Alterar o caminho de runtime

Se você alterar o caminho de runtime, o app moverá automaticamente todas as suas instâncias e arquivos de configuração para o novo local.

Se a pasta de destino já estiver em uso, o app apenas trocará a configuração do caminho de runtime e nenhum arquivo será movido ou copiado.

#### Erro de migração

Se a migração falhar, o app exibirá uma mensagem de erro.

A primeira coisa a fazer é tentar entender o que a mensagem está dizendo.
Se todos os arquivos foram copiados com sucesso, é provável que o erro tenha ocorrido ao excluir os arquivos antigos. Você pode fechar o app e excluir os arquivos antigos manualmente.

NÃO EXCLUA o arquivo chamado `runtime_path_override` no caminho de runtime antigo, pois ele é usado pelo app para detectar que o caminho foi alterado.

Em caso de dúvida, entre em nosso [servidor do Discord](https://discord.gdlauncher.com) e peça ajuda.
