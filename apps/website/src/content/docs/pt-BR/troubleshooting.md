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

Leia a mensagem e identifique a etapa que falhou. Preserve o runtime antigo, o destino e as cópias de recuperação até verificar os dados do destino e o runtime realmente usado após reiniciar. Concluir a cópia não prova que o ponteiro foi publicado. Versões antigas podiam reverter os arquivos copiados no destino após uma falha posterior; excluir a origem podia então remover a única cópia utilizável.

**Se, depois de reiniciar, Settings → Runtime Path ainda mostra o local antigo**, a mudança parou antes de terminar e seus dados estão seguros no local antigo. O launcher apaga o que tinha copiado pra pasta nova, então corrija a causa mostrada no erro (por exemplo, libere espaço no drive destino) e mova de novo, pro mesmo caminho ou pra outra pasta vazia. Se ele não conseguiu apagar alguns desses arquivos (por exemplo, porque outro programa estava com eles abertos), a pasta nova guarda esses arquivos, marcada por um arquivo `.gdl_migration_in_progress`. O launcher não vai usar essa pasta nem mover dados pra ela de novo, e Settings avisa que ela tem uma mudança inacabada se você escolhê-la. Você pode apagar essa pasta e depois mover de novo.

Se uma mudança falhar antes de o ponteiro ser publicado, o launcher atual apaga os dados que copiou pro destino; o que ele não consegue apagar fica, marcado com `.gdl_migration_in_progress`, e o runtime antigo continua selecionado. Após publicar o ponteiro, o destino é selecionado salvo quando um caminho explícito tem prioridade; avisos posteriores não o revertem. Depois disso, o launcher exclui cada original do runtime antigo após verificar que o arquivo ainda corresponde à sua cópia, então a mudança libera o espaço no drive antigo. Um arquivo que mudou depois de copiado fica no runtime antigo, assim como um que não pôde ser excluído (outro programa o mantinha aberto, ou você não tem permissão). O launcher reinicia de qualquer forma e, na primeira inicialização a partir do novo local, lista esses arquivos em dois grupos. Os que mudaram podem ser mais recentes que a cópia no novo local: se precisar de algum, copie-o para o mesmo lugar no novo local antes de jogar. Os que não puderam ser excluídos são iguais à cópia, então você pode apagá-los. Se o launcher não conseguir confirmar que o novo local foi salvo no disco, ele não exclui nada e avisa isso depois de reiniciar; depois de verificar que suas instâncias estão no novo local, você pode apagar a pasta antiga.

O ponteiro `runtime_path_override` fica no diretório `userData` do Electron (os dados de aplicativo do launcher), não no runtime antigo. Preserve-o. Um caminho explícito na linha de comando tem prioridade sobre o caminho do ambiente, e ambos sobre o ponteiro. Verifique o local efetivo no ajuste de caminho do runtime. Se o diálogo de recuperação exigir Restart ou Retry, siga essa ação e mantenha as duas pastas se falhar novamente.

Veja [o marcador e os locais de dados do aplicativo](/pt-BR/docs/runtime-path-vs-app-data#o-marcador-runtime_path_override) para encontrar o ponteiro autoritativo.

Em caso de dúvida, entre em nosso [servidor do Discord](https://discord.gdlauncher.com) e peça ajuda.
