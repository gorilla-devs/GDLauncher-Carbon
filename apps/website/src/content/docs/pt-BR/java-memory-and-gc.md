---
title: "Memória Java e Garbage Collection"
description: "Como o Minecraft usa RAM, por que alocar mais nem sempre é mais rápido, o que heap, pausas GC e os Aikar's flags realmente fazem, e quando deixar os defaults em paz."
faq:
  - question: "Quanto RAM devo alocar pro Minecraft?"
    answer: "Vanilla: 2-4 GB já dá. Instância modded pequena (20-40 mods): 4-6 GB. Modpack grande (100+ mods, estilo ATM): 6-8 GB. Acima de 8 GB raramente ajuda, o gargalo migra pro garbage collector, não pro tamanho da memória, e heaps gigantes significam pausas GC mais longas e jogo mais entrecortado."
  - question: "Mais RAM deixa o Minecraft mais rápido?"
    answer: "Até certo ponto, depois não. Quando o Minecraft tem heap suficiente pra segurar o mundo e os mods sem coleta constante, mais memória só dá ao GC mais coisa pra escanear quando ele roda, ou seja, pausas mais longas, jogo mais entrecortado. O certo é 'o suficiente', não 'tudo que eu tenho'."
  - question: "O que são os Aikar's flags e devo usá-los?"
    answer: "Aikar's flags são um conjunto de argumentos JVM que tunam o garbage collector G1 pra favorecer pausas curtas em vez de throughput, originalmente pra servidores Minecraft. Ajudam em servidores e em clientes modded grandes. O GDLauncher não aplica automaticamente; você pode colar em Java Arguments nas Settings da instância. Não é mágica e nem sempre é mais rápido."
  - question: "Por que o Minecraft trava a cada poucos segundos mesmo com 16 GB alocados?"
    answer: "Quase sempre é stutter de pausa GC, não falta de memória. O GC roda menos com mais heap, mas quando roda em heap maior, demora mais. Paradoxal: diminua a alocação, ou troque pra um modpack menor."
  - question: "Qual a diferença entre Xmx e Xms?"
    answer: "Xmx é o tamanho máximo do heap (teto). Xms é o tamanho inicial (ponto de partida). O slider de RAM do GDLauncher define Xmx; Xms é setado num valor sensato automaticamente. Pro Minecraft, deixar Xmx igual a Xms não ajuda significativamente, a JVM cresce o heap conforme necessário dentro do Xmx."
---

# Memória Java e Garbage Collection

## Como o Minecraft usa memória

O Minecraft é um programa Java. Como todo programa Java, ele roda dentro de uma Java Virtual Machine (JVM) que recebe uma fatia fixa de RAM do sistema. Tudo que o Minecraft faz, chunks carregados, entidades, estado dos mods, texturas, vive nessa fatia.

Quando você define **Instance Java Memory** nas configurações de instância do GDLauncher, você está definindo `-Xmx`, o tamanho máximo de heap que a JVM pode usar. O código Java em si (alocações de objeto, estruturas de dados de mods, estado do mundo) vive nesse heap. Texturas e buffers OpenGL vivem fora, em memória nativa, e não são afetados pelo Xmx.

## O verdadeiro gargalo é o garbage collector

O Java não libera memória à mão; tem um **garbage collector** que escaneia periodicamente o heap, acha objetos sem referência e recupera. O Minecraft moderno usa o coletor **G1** por padrão.

O GC roda em dois modos:

- **Young GC.** Curto, frequente. Escaneia uma "young generation" pequena de objetos recém-criados. Geralmente um ou dois milissegundos.
- **Old GC / Mixed GC.** Mais longo, menos frequente. Escaneia o resto do heap. Pode levar dezenas de milissegundos ou mais em heaps grandes.

Quando o GC roda, **o Minecraft pausa**. Quanto maior o heap, mais longas as coletas grandes. Por isso adicionar mais RAM além do que o jogo precisa de verdade *piora* o stutter de pausa, não melhora.

É a coisa mais contraintuitiva do tuning de memória Java: **alocar menos pode ser mais suave que alocar mais**.

## A quantidade certa de RAM

Diretrizes aproximadas, baseadas no que cabe no heap ativo sem thrashing:

| Workload | Xmx recomendado |
|---|---|
| Minecraft vanilla | 2-4 GB |
| Mods leves (20-40, estilo Sodium) | 4 GB |
| Modpack médio (80-120 mods) | 4-6 GB |
| Modpack grande (ATM, FTB Continents, 250+ mods) | 6-8 GB |
| Modpacks "kitchen sink" (500+ mods, pregen de chunks profunda) | 8-10 GB |

Acima de 10 GB raramente é útil, a menos que a documentação do pack diga explicitamente. Alguns packs realmente pedem mais (combos de mods famintos por memória tipo Better End com NetherEx); siga a recomendação do pack.

## Como o slider do GDLauncher funciona

Abra uma instância, clique na aba Settings, desça até **Instance Java Memory**. Ligue o toggle pra ativar o override por instância e arraste o slider; vai de 1 GB até o total de RAM do sistema (com aviso acima de 80 %). O launcher converte o valor em `-Xmx<n>M` e passa pra JVM.

O mesmo slider existe no nível global em **Settings → Java → Java Memory**, como default pra qualquer instância que não sobrescreva. Deixe o global baixo pro uso casual e suba só os modpacks pesados.

## Aikar's flags

Lista longa de argumentos JVM que tunam o G1 pra priorizar pausas curtas em vez de throughput. Originalmente escritos pra servidores Minecraft, mas úteis em clientes modded também. Parecem com:

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

O GDLauncher não aplica por padrão. Pra usar, cole a string inteira no campo **Instance Java Arguments** na aba Settings da instância (ou em **Settings → Java → Java Arguments** pra aplicar globalmente). Efeitos variam; o benefício mais consistente é menos pausas longas em heaps de 6-10 GB.

Alguns alertas:

- Tunados pra versões antigas do Java. No Java 17+ os defaults já são bem bons e os ganhos do Aikar são menores.
- Assumem padrões de alocação tipo servidor. Em desktop com heap pequeno, podem prejudicar.
- Não adicionam memória nem mudam quanto o jogo usa; só mudam como o coletor se comporta.

Sem motivo específico, deixe o campo Java Arguments do jeito que o GDLauncher deixou.

## Diagnosticando stutter

Se o Minecraft pausa por centenas de milissegundos a cada poucos segundos:

1. Abra a tela de debug F3. Olhe a linha "Mem:" (canto superior direito). Se oscila rápido entre baixo e alto, é churn de GC.
2. Diminua o Xmx em 1-2 GB e teste. Contraintuitivo, mas heaps menores fazem GC mais rápido.
3. Se um mod específico aloca feito louco (alguns mods de pregen ou render fazem isso), aparecerá em profilers do lado do mod (Spark, JmxMC). O mod talvez precise de atualização.
4. Se seu CPU está em 100% durante as pausas, o GC está trabalhando duro mesmo. Baixe o Xmx mais ou tire mods famintos.

## TL;DR

- Coloque Java Memory em *o suficiente*, não no que você tem.
- O gargalo é tempo de pausa GC, não tamanho bruto do heap.
- Aikar's flags ajudam em heaps grandes mas não são panaceia.
- 4-6 GB é o certo pra quase tudo modded.
