---
title: "Conta GDL vs conta Microsoft"
description: "O GDLauncher usa dois tipos de conta diferentes. Microsoft para jogar Minecraft, GDL para compartilhar e funcionalidades sociais. O que cada uma faz e quais você precisa."
faq:
  - question: "Preciso de uma conta GDL para jogar Minecraft?"
    answer: "Não. Para jogar você só precisa de uma conta Microsoft (a que comprou Minecraft Java Edition). A conta GDL é opcional e só destrava funcionalidades próprias do GDLauncher: compartilhar instâncias, friend codes, histórico de nome, edição de perfil. Você usa o GDLauncher bem sem ela."
  - question: "O que uma conta GDL destrava?"
    answer: "Hoje, principalmente compartilhamento de instância: você gera um código em botão direito → Share, e outro usuário GDLauncher cola para importar a instância. Também recebe um nome de exibição estável com histórico de renomeações e um cartão de perfil com friend code que te identifica nas previews de compartilhamento. Tudo que envolve outros usuários do GDLauncher passa pela conta GDL."
  - question: "Posso usar o GDLauncher sem conta Microsoft?"
    answer: "Não. A conta Microsoft prova que você possui Minecraft e obtém o token de lançamento da Mojang. Sem ela, o launcher não tem com o que se autenticar nos servidores do Minecraft."
  - question: "Posso ter várias contas Microsoft no GDLauncher?"
    answer: "Pode. Settings → Accounts mostra todas as contas Microsoft logadas em uma tabela. Você pode adicionar, remover e escolher qual está Active (a que o Play usa). A conta ativa fica destacada na coluna da esquerda."
  - question: "O que é o friend code do meu perfil GDL?"
    answer: "Um identificador curto e estável da sua conta GDL. Ele não muda quando você renomeia o nome de exibição, e aparece nas previews de compartilhamento pra que outros saibam quem está compartilhando. Copiável em Settings → Accounts → cartão de perfil GDL."
---

# Conta GDL vs conta Microsoft

## Dois sistemas de conta, um launcher

O GDLauncher tem dois sistemas de conta. **Microsoft** prova que você é dono do Minecraft e é obrigatório pra jogar. **GDL** é a conta opcional própria do GDLauncher, pra funcionalidades que usam o backend GDL (compartilhar instâncias, perfil, histórico de nome).

### Conta Microsoft

A conta com a qual você comprou Minecraft Java Edition, a que detém a licença. A Microsoft exige ela para lançar Minecraft. O GDLauncher loga na Microsoft, segura os tokens, e na hora do lançamento entrega pra Mojang para que os servidores saibam que você possui o jogo.

Você precisa de pelo menos uma conta Microsoft logada para jogar. Sem ela, o botão Play não faz nada.

Guardado localmente por conta: access token, refresh token, ID token, nome de usuário Minecraft e UUID, uma referência da skin, e a expiração do access token. O launcher renova o access token em segundo plano via refresh token; normalmente você nem percebe.

O que destrava: lançar Minecraft, entrar em servidores, ser dono do jogo.

### Conta GDL

O sistema de conta próprio do GDLauncher. Opcional. Existe só pra ligar funcionalidades que o GDLauncher entrega, coisas que a Microsoft não deveria se importar.

Você cria com email e nome de exibição e recebe um friend code estável. A partir daí pode usar as funcionalidades que envolvem outros usuários do GDLauncher.

Localmente, só o vínculo é salvo: a qual conta Microsoft essa identidade GDL pertence, e um JWT pra falar com o backend GDL. Nome de exibição, friend code, email, foto de perfil etc. ficam no backend GDL, a UI puxa quando precisa.

O que destrava:

- **Cloud Instance Sharings.** Botão direito na instância → Share gera um código que outros usuários GDLauncher colam pra importar.
- **Histórico de nome.** Renomear o nome de exibição registra o histórico das mudanças; você vê os nomes antigos no cartão de perfil e pode apagá-los se quiser.
- **Edição de perfil.** Nome de exibição, foto de perfil, configurações de email de recuperação, tudo a partir do cartão de perfil GDL em Settings → Accounts.

## Quando precisa de qual

| Cenário | Microsoft | GDL |
|---|---|---|
| Só lançar Minecraft | Obrigatória | Não precisa |
| Instalar mods e modpacks do CurseForge/Modrinth | Obrigatória | Não precisa |
| Cloud Instance Sharing com amigo | Obrigatória | Obrigatória |
| Receber código de instância | Obrigatória | Obrigatória |
| Usar o sistema de amigos | Obrigatória | Obrigatória |
| Jogar offline (instância já instalada) | Auth em cache funciona um tempo | Não precisa |

## Como gerenciar

Os dois ficam em **Settings → Accounts**.

A seção GDL Account é em cima. Deslogado: botão Sign in / Sign up. Logado: cartão de perfil com nome de exibição, friend code (copiável), email de recuperação, status de verificação. Uma "Danger Zone" no rodapé permite agendar exclusão da conta com cooldown de 7 dias.

A seção Microsoft Accounts vem embaixo em formato de tabela. Colunas: Active, Username, Type, Status, UUID, Actions. Status mostra o estado do token por conta:

- **ok** (check verde): token válido, a conta consegue lançar.
- **expired** (alerta amarelo): token expirado. A coluna Actions mostra um ícone de refresh, clicar te manda de volta pelo fluxo de login Microsoft.
- **refreshing** (refresh amarelo): o launcher está renovando o token em segundo plano. Nada a fazer.
- **invalid** (X vermelho): o token não pôde ser renovado. Mesmo ícone refresh que expired, clicar te conduz pelo fluxo de login Microsoft.

Para trocar de conta ativa, clique na célula Active da linha desejada. A linha ativa mostra um ícone duplo-check; as outras mostram fraco no hover.

## Remover contas

Remover a única conta Microsoft te desloga do GDLauncher inteiro e te leva pra home.

Remover uma conta Microsoft que é a vinculada à sua conta GDL abre um modal de confirmação, perguntando se você realmente quer quebrar o vínculo antes da exclusão.

Excluir sua conta GDL é uma ação atrasada em 7 dias. Durante o cooldown você pode cancelar pela mesma página.
