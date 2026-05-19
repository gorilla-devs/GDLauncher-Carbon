---
title: "Modo offline"
description: "O que dá e o que não dá pra fazer no GDLauncher sem internet. O que é cacheado, o que precisa falar com a rede, e como o vencimento de token se comporta na prática."
faq:
  - question: "Posso jogar Minecraft offline pelo GDLauncher?"
    answer: "Sim. Singleplayer funciona totalmente offline. Se o seu token cacheado ainda está válido, você clica Play e o Minecraft inicia normalmente. Se já expirou, o launcher mostra um prompt 'Account Expired' com um botão 'Launch anyway'; escolha esse e dá pra jogar singleplayer mesmo assim. Auth online só é exigida para servidores multiplayer que verificam identidade via Mojang."
  - question: "Quanto tempo posso ficar offline antes dos tokens expirarem?"
    answer: "Depende do que você quer fazer. Pra singleplayer, praticamente não tem limite de tempo: o launcher oferece 'Launch anyway' assim que o token expira. Pra servidores multiplayer que verificam identidade via Mojang você precisa de um token fresco, o que significa voltar a ficar online pra renovar. O launcher renova o token de auth do Minecraft proativamente cerca de 12 horas antes da expiração das 24 horas, então enquanto você tiver ficado online recentemente, o multiplayer continua funcionando."
  - question: "Posso instalar mods ou modpacks novos offline?"
    answer: "Não. Downloads de mod vêm dos CDNs do CurseForge e Modrinth, ambos precisam de internet. Mesma coisa pra downloads de Java, assets do Minecraft e manifests de modpack. Tudo que envolve instalação precisa de conexão."
  - question: "Posso atualizar uma instância existente offline?"
    answer: "Não. Mesmo motivo: updates puxam arquivos novos dos CDNs. O launcher enfileira o update e tenta de novo quando vê conexão."
  - question: "E a conta GDL, funciona offline?"
    answer: "Parcialmente. O launcher lembra que você está logado no GDL, mas qualquer coisa que precise falar com o serviço GDL (compartilhar instância, editar perfil, ver seus compartilhamentos) precisa de internet. A conta Microsoft é que controla o lançamento; GDL é pra funcionalidades além do lançar."
---

# Modo offline

## O que "offline" significa aqui de verdade

O comportamento offline do GDLauncher depende de três necessidades de rede diferentes:

1. **Auth Microsoft** (provar pra Mojang que você possui Minecraft).
2. **Downloads de mod e asset** (CurseForge, Modrinth, CDN de libraries da Mojang).
3. **Funcionalidades de conta GDL** (compartilhamento de instância, perfil, histórico de nome, etc.).

Cada uma falha diferente quando a rede cai, e o launcher reage de modo diferente em cada caso.

## Lançando uma instância instalada offline

Cenário mais comum: você tá num avião, num chalé, ou sua internet caiu em casa, e quer jogar algo que já tem instalado.

**Geralmente funciona**, porque o GDLauncher cacheia os dados necessários pro lançamento:

- Tokens de auth da Mojang ficam guardados localmente com timestamps de expiração.
- As libraries e assets do Minecraft já estão no disco (no runtime path).
- Instâncias moddadas têm os mods instalados localmente.

Quando você clica Play offline, o launcher:

1. Verifica se o token de auth do Minecraft da conta Microsoft ativa ainda é válido (não expirou).
2. Se sim, lança o Minecraft direto com esse token. O Minecraft em si não precisa de internet pra lançar um mundo single-player.
3. Se o token de acesso expirou mas o refresh token ainda é válido, o launcher tenta chamar o endpoint de refresh da Microsoft, que precisa de internet. Offline, essa chamada falha, e o status da conta vira "expired" em Settings → Accounts.
4. Se a conta está expirada e você clica Play mesmo assim, o launcher abre um modal Account Expired com dois botões: **Launch anyway** (usa o token em cache, basta pra singleplayer) e **Back to login** (te manda pelo fluxo de sign-in da Microsoft, precisa de internet).

Pra singleplayer, então, 'Launch anyway' funciona independente de quando você ficou online pela última vez: o token não é verificado por nada depois que o Minecraft inicia. Pra servidores multiplayer que verificam identidade, você precisa de um token não expirado, ou seja, ter ficado online recente o suficiente pra renovar.

### Por que tokens expiram

Isso é definido pelos servidores de auth da Microsoft e da Mojang, não pelo GDLauncher. A cadeia de auth produz dois tokens que importam pro launcher:

- Um **token de acesso OAuth Microsoft** (~1 hora). É o que o launcher usa pra falar com as APIs de auth da Microsoft / Xbox / Mojang. Curto, mas o launcher renova com um refresh token quando está online; quase nunca você nota.
- Um **token de auth Minecraft** (~24 horas). É o que é entregue ao Minecraft no lançamento, então é o que controla o jogo offline. O GDLauncher renova proativamente cerca de 12 horas antes da expiração enquanto você está online.

O refresh token da Microsoft dura meses, mas pode ser invalidado do lado do servidor, por exemplo quando você muda a senha Microsoft, ativa uma feature de segurança nova, ou desloga pelo site da Microsoft. Se seu refresh token for invalidado durante o offline, não tem o que o launcher possa fazer até você voltar a ficar online pra reautenticar.

## Entrar em servidores multiplayer offline

**Não funciona**, porque servidores multiplayer verificam sua identidade contra o session server da Mojang, e isso precisa de internet dos dois lados. Multiplayer LAN pode funcionar entre máquinas na mesma LAN offline desde que as duas tenham autenticado online recentemente.

## Instalar novas instâncias, mods ou modpacks offline

**Não funciona.** Todo fluxo de instalação baixa de um CDN:

- Modpacks puxam o manifest e depois os arquivos de mod individuais.
- Adicionar um mod pela aba Addons baixa o JAR.
- Criar uma instância custom pra uma versão do Minecraft que você não tem baixa o manifest JSON daquela versão, o JAR da versão, os assets, o instalador do mod loader.

Tudo isso falha offline com erros de timeout ou DNS. O launcher não tenta indefinidamente, você vai ver a falha no modal de criação de instância ou no painel Tasks.

Se você sabe que vai pra um lugar offline, pré-instale as instâncias que vai querer antes de sair.

## Funcionalidades de conta GDL offline

**Quase sempre não funciona**, porque funcionalidades de conta GDL são por definição "falar com o backend do GDL". Especificamente:

- Cloud Instance Sharing (gerar código): falha, serviço GDL inalcançável.
- Importar uma instância compartilhada: falha pelo mesmo motivo.
- Editar perfil GDL: falha.
- Ver seus compartilhamentos: mostra estado cacheado, não atualiza.

O launcher lembra que você está logado no GDL enquanto está offline, mas a UI mostra dados desatualizados e recusa ações que precisariam de chamada de rede.

## TL;DR

- Instância já instalada, token fresco: lançar offline funciona.
- Instância já instalada, token expirado: o launcher pergunta, escolha 'Launch anyway' pra singleplayer.
- Multiplayer com token expirado: bloqueado até conseguir alcançar a Microsoft pra renovar.
- Qualquer coisa que baixe: bloqueado.
- Qualquer coisa que fale com o backend do GDL: bloqueado.
- Mundos single-player: 100% capazes offline assim que a instância está no disco.
