---
title: "Instâncias de modpack bloqueadas"
description: "O que significa uma instância de modpack estar bloqueada, por que o GDLauncher bloqueia, e como desbloquear (Unlock) ou desvincular (Unpair) quando precisar."
---

## O que é uma instância bloqueada?

Quando você instala um modpack do CurseForge ou do Modrinth no GDLauncher, a instância fica **bloqueada (locked)** por padrão. Aparece um ícone de cadeado ao lado da instância, e as ações que mudam o conteúdo do pack, adicionar, remover ou atualizar mods individuais, ficam desativadas. Você ainda pode jogar, mudar Java ou RAM, tirar prints e tudo o mais; o cadeado só protege o *conjunto de mods gerenciado pelo pack*.

O bloqueio existe porque um modpack é uma coleção de mods testada e com versões fixadas. Os autores do pack escolhem a lista de mods de propósito e fixam versões específicas para garantir compatibilidade. Se você troca um mod por uma versão nova, pode quebrar outro mod que dependia da versão antiga. O bloqueio evita esse erro antes de acontecer.

## O que dá e o que não dá fazer com o bloqueio

Com a instância bloqueada **dá** para:

- Iniciar e jogar a instância.
- Mudar RAM, argumentos do Java e Java Override.
- Tirar prints e ler logs.
- Mudar o nome e o ícone da instância (Edit Instance).
- Atualizar o modpack inteiro para uma release mais nova (Settings → Change Modpack Version).

**Não dá** para:

- Adicionar nada pela aba Addons, isso inclui **mods, shaders, resource packs, data packs e worlds**. Enquanto o bloqueio está ativo, o botão Add fica desativado em todos os tipos de addon.
- Remover ou desativar um mod ou addon gerenciado pelo pack.
- Atualizar individualmente um mod gerenciado pelo pack.

As abas Mods e Addons mostram um aviso «Esta instância está bloqueada, alterações não podem ser aplicadas» ao lado das ações desativadas. O botão Install do navegador de Addons também é bloqueado em instâncias bloqueadas.

## Três estados: Locked / Unlocked / Unpaired

Esses três termos aparecem no GDLauncher e não são sinônimos.

- **Locked (bloqueada)**: a instância está vinculada a um modpack do CurseForge ou Modrinth e o conjunto de mods gerenciado pelo pack é somente leitura. Estado padrão após a instalação.
- **Unlocked (desbloqueada)**: ainda vinculada ao modpack (o nome e a versão continuam sendo rastreados), mas o conjunto de mods passa a ser livremente editável. O GDLauncher continua lembrando o pack, então dá para atualizar para uma nova release depois, só que a consistência fica por sua conta.
- **Unpaired (desvinculada)**: sem ligação com o modpack. A instância vira uma instância custom, mesmos arquivos, mas o GDLauncher não rastreia mais atualizações do pack nem a trata como instância de modpack. De Unlocked para Unpaired é mão única.

## Como desbloquear uma instância (Unlock)

1. Abra a instância e clique no ícone de engrenagem (ou clique com o botão direito na instância → Settings).
2. Vá até a seção **Modpack Info** no topo da página de Settings. Você vê o ícone, o nome e a versão atual do pack, com uma linha de botões abaixo.
3. Clique em **Unlock** (botão com o ícone de cadeado). A instância vai para o estado desbloqueado na hora.

Depois de desbloquear, o cabeçalho da seção muda para «Unlocked» com o cadeado aberto. Dá para bloquear de novo pelo mesmo fluxo, mas, na prática, depois de mexer no conjunto de mods, raramente faz sentido.

## Como desvincular (Unpair)

1. Na mesma seção Modpack Info, clique em **Unpair** (ícone de ramo git).
2. Confirme no modal. O GDLauncher avisa que a ação é permanente.

Depois de desvincular, a seção Modpack Info some por completo. A instância vira uma instância custom e as opções **Change Modpack Version** e **Reinstall** não se aplicam mais.

## Reinstall vs Unlock

A seção Modpack Info também tem uma ação **Reinstall**. É separada do Unlock e tem outro propósito: reinstalar o modpack na versão atual, sobrescrevendo os mods e configs gerenciados pelo pack conforme o manifest. Use para consertar uma instalação quebrada (jar corrompido, configs apagadas, etc.) sem perder seus mundos.

| Ação | Efeito nos mods do pack | Vínculo com o pack |
|--------|------------------------------|---------------------|
| Unlock | Permanecem, mas editáveis | Mantido |
| Unpair | Permanecem como arquivos, mas não são mais «mods do pack» | Removido |
| Reinstall | Reset para a versão do manifest | Mantido |
| Change Modpack Version | Substituídos pelo manifest da nova versão | Mantido (nova versão) |

## Quando desbloquear, e quando não

Desbloqueie quando:
- Um mod do pack tem bug crítico ou correção de segurança e o pack não foi atualizado.
- Você quer adicionar seu próprio mod, shader, resource pack, data pack ou world em cima do que o pack traz, o botão Add da aba Addons fica bloqueado pelo cadeado, então para instalar pela UI é preciso desbloquear.
- Você está mantendo sozinho um pack que foi abandonado.

Mantenha bloqueada quando:
- O pack está sendo mantido, deixe o autor gerenciar o pinning das versões e espere a próxima release.
- Você joga uma experiência curada e não quer fugir do conjunto previsto.

Padrão comum: desbloqueie por um momento, instale seus adicionais, e deixe a instância desbloqueada. O que você adicionou continua mesmo se bloquear de novo, porque o bloqueio só afeta o conjunto *gerenciado pelo pack*, mas, na prática, depois que você começou a manter a instância, raramente vale a pena bloquear de volta.

## O que o bloqueio não é

Bloqueio não é sistema de permissões nem fronteira de segurança. É um guard rail para evitar edições acidentais de mods pela UI. A pasta da instância no disco é uma pasta normal, qualquer coisa que escreva direto em `mods` (uma ferramenta de terceiros, uma cópia manual) ignora o bloqueio.

Jars colocados assim aparecem na aba Mods junto com os mods do pack. Para removê-los é preciso ir pelo sistema de arquivos, não pela UI.

## Solução rápida

- **«Não consigo atualizar um único mod.»** O bloqueio funcionando como esperado. Use Unlock (Settings → Unlock) ou Change Modpack Version para atualizar o pack inteiro.
- **«Update All está em cinza na instância bloqueada.»** Mesmo motivo. Use Change Modpack Version ou desbloqueie antes.
- **«Por que meu mod adicionado por mim ainda aparece após bloquear de novo?»** O bloqueio só vale para mods do pack; o que você adicionou continua visível.
- **«Reinstall sobrescreveu uma config que eu editei.»** Comportamento esperado. Reinstall reaplica o manifest. Faça backup das configs antes de Reinstall.
