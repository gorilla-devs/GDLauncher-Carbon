---
title: "Erros de autenticação da Microsoft"
description: "Resolva os erros mais comuns de autenticação da Microsoft no GDLauncher. Soluções para Invalid Grant, conta banida, acesso de console necessário e erros do Xbox Live."
faq:
  - question: "Por que estou recebendo um erro 'Invalid Grant' no GDLauncher?"
    answer: "Um erro 'Invalid Grant' geralmente indica que há um problema de segurança na sua conta Microsoft. As soluções mais comuns são habilitar a autenticação em duas etapas na conta Microsoft, definir uma senha caso ainda não tenha, ou sair e entrar novamente."
  - question: "Por que o GDLauncher diz que minha conta está banida?"
    answer: "Se o GDLauncher informa que sua conta está banida, o banimento veio da Mojang ou da Microsoft, não do GDLauncher. Faça login em minecraft.net ou na sua conta Microsoft para ver o motivo. O GDLauncher apenas repassa a resposta de autenticação, não existe lista de banimentos do lado do GDLauncher."
  - question: "Por que o GDLauncher diz que eu preciso de acesso de console?"
    answer: "Isso costuma aparecer em contas infantis ou contas com restrições de grupo familiar. A conta dos pais precisa permitir que a conta da criança jogue Minecraft na plataforma que está sendo usada. Ajuste as configurações de família em account.microsoft.com/family."
  - question: "Estou recebendo erros de autenticação do Xbox Live o tempo todo. O que faço?"
    answer: "Erros do Xbox Live geralmente indicam que a configuração de país/região da conta Microsoft não permite Xbox Live, ou que a conta ainda não aceitou os termos do Xbox Live. Faça login uma vez em xbox.com com a mesma conta Microsoft para aceitar os termos e tente de novo no GDLauncher."
  - question: "Preciso comprar o Minecraft de novo para usar o GDLauncher?"
    answer: "Não. O GDLauncher usa sua conta Minecraft Microsoft / Mojang existente. Não há compra ou assinatura separada. Se você já tem o Minecraft Java Edition, pode entrar no GDLauncher com a mesma conta."
---

# Erros de autenticação da Microsoft

Quando você entra no GDLauncher com uma conta Microsoft, o launcher fala em seu nome com o serviço OAuth da Microsoft e a API de autenticação da Mojang. Os erros devolvidos por esses serviços aparecem direto no launcher; o texto vem da Microsoft, não do GDLauncher.

Abaixo, os mais comuns e o que significam.

## Invalid Grant

Aparece quando a Microsoft recusa a troca OAuth. Causas mais comuns:

- A conta não tem senha definida (conta Microsoft criada via link de email ou login social). Defina uma senha em [account.microsoft.com](https://account.microsoft.com).
- A conta usa um fluxo de login antigo sem autenticação em duas etapas. Ativar 2FA em [account.microsoft.com/security](https://account.microsoft.com/security) resolve pra maioria.
- Os tokens em cache estão expirados. Saia da conta em **Settings → Accounts** e entre novamente.

## Conta banida

O GDLauncher repassa a resposta da Mojang sem alterar. O banimento é do lado da Mojang; o GDLauncher não mantém lista própria de banimentos. Entre em [minecraft.net](https://minecraft.net) com a mesma conta pra ver o motivo do banimento e opções de recurso.

## Acesso de console necessário

Costuma aparecer em contas infantis dentro de um grupo familiar Microsoft. A conta do responsável precisa autorizar Minecraft Java Edition pra criança em [account.microsoft.com/family](https://account.microsoft.com/family). Depois de autorizar, saia e entre novamente no GDLauncher.

## Erros do Xbox Live

Falhas no Xbox Live geralmente caem em uma de duas categorias:

- A configuração de país/região da conta Microsoft não permite Xbox Live. Ajuste em [account.microsoft.com/profile](https://account.microsoft.com/profile).
- A conta não aceitou os termos do Xbox Live. Entre uma vez em [xbox.com](https://xbox.com) com a mesma conta Microsoft pra aceitar os termos e tente de novo no GDLauncher.

## Conta expirada

O refresh token da Microsoft expirou ou foi revogado (na maioria das vezes porque você mudou a senha da conta em outro lugar). O GDLauncher mostra um prompt "Account expired" e oferece reautenticar. Entre de novo em **Settings → Accounts**.

## Se nada acima resolve

Se a mensagem de erro não bate com nenhuma das anteriores, compartilhe os dois logs no nível do app no nosso [Discord](https://discord.gdlauncher.com): `main.log` (Electron) e o mais recente `__gdl_logs__/<timestamp>.log` (Rust core). Onde encontrá-los está no [Share App Logs](/guides/share-app-logs). Quase sempre precisamos dos dois, o fluxo de autenticação passa pelos dois processos.
