---
title: "Memoria Java y Garbage Collection"
description: "Cómo usa Minecraft la RAM, por qué asignar más no siempre es más rápido, qué hacen realmente heap, pausas GC y los Aikar's flags, y cuándo es mejor dejar los valores por defecto."
faq:
  - question: "¿Cuánta RAM debo asignar a Minecraft?"
    answer: "Para vanilla: 2-4 GB suficiente. Para una instancia con pocos mods (20-40): 4-6 GB. Para un modpack grande (100+ mods, estilo ATM): 6-8 GB. Por encima de 8 GB rara vez ayuda porque el cuello de botella se mueve al garbage collector, no al tamaño de memoria, y heap gigantes implican pausas GC más largas y juego más entrecortado."
  - question: "¿Más RAM hará a Minecraft más rápido?"
    answer: "Hasta cierto punto, luego no. Una vez Minecraft tiene heap suficiente para sostener el mundo y todos los mods sin recolección constante, añadir memoria solo le da al GC más que escanear cuando corre, por lo tanto pausas más largas, juego más entrecortado. Lo correcto es 'lo justo', no 'todo lo que tengo'."
  - question: "¿Qué son los Aikar's flags y debo usarlos?"
    answer: "Los Aikar's flags son un conjunto de argumentos de la JVM que ajustan el garbage collector G1 para favorecer pausas cortas frente a throughput, originalmente diseñados para servidores Minecraft. Ayudan en servidores y en clientes moddeados grandes. GDLauncher no los aplica automáticamente; puedes pegarlos en Java Arguments en la configuración de la instancia. No son magia y no siempre son más rápidos."
  - question: "¿Por qué Minecraft tartamudea cada pocos segundos incluso con 16 GB asignados?"
    answer: "Casi siempre es stutter de pausa GC, no escasez de memoria. El GC corre menos con más heap, pero cuando corre en uno grande tarda más. Paradójicamente, baja la asignación, o cambia a un modpack más pequeño."
  - question: "¿Cuál es la diferencia entre Xmx y Xms?"
    answer: "Xmx es el tamaño máximo del heap (techo). Xms es el inicial (punto de partida). El slider de RAM de GDLauncher fija Xmx; Xms se pone en un valor razonable automáticamente. Para Minecraft, igualar Xmx y Xms no ayuda mucho, la JVM crece el heap según necesidad dentro de Xmx de todos modos."
---

# Memoria Java y Garbage Collection

## Cómo usa Minecraft la memoria

Minecraft es un programa Java. Como todos los programas Java, corre dentro de una Java Virtual Machine (JVM) que recibe una porción fija de RAM del sistema. Todo lo que hace Minecraft, chunks cargados, entidades, estado de mods, texturas, vive en esa porción.

Cuando ajustas **Instance Java Memory** en la configuración de instancia de GDLauncher, estás fijando `-Xmx`, el tamaño máximo de heap que la JVM puede usar. El código Java en sí (asignaciones de objetos, estructuras de datos de mods, estado del mundo) vive en este heap. Texturas y buffers OpenGL viven fuera, en memoria nativa, y no se ven afectados por Xmx.

## El verdadero cuello de botella es el garbage collector

Java no libera memoria a mano; tiene un **garbage collector** que escanea periódicamente el heap, encuentra objetos sin referencias y los recupera. El Minecraft moderno usa por defecto el colector **G1**.

GC corre en dos modos:

- **Young GC.** Corto, frecuente. Escanea una pequeña "young generation" de objetos recién creados. Usualmente uno o dos milisegundos.
- **Old GC / Mixed GC.** Largo, menos frecuente. Escanea el resto del heap. Puede tardar decenas de milisegundos o más en heaps grandes.

Cuando el GC corre, **Minecraft se pausa**. Cuanto más grande el heap, más largas son esas grandes colecciones. Por eso añadir RAM más allá de lo que el juego necesita realmente *empeora* el stutter de pausas, no lo mejora.

Es lo más contraintuitivo del tuning de memoria Java: **asignar menos puede ser más suave que asignar más**.

## La cantidad correcta de RAM

Guías aproximadas, basadas en lo que cabe en el heap activo sin thrashing:

| Carga | Xmx recomendado |
|---|---|
| Minecraft vanilla | 2-4 GB |
| Mods ligeros (20-40, estilo Sodium) | 4 GB |
| Modpack medio (80-120 mods) | 4-6 GB |
| Modpack grande (ATM, FTB Continents, 250+ mods) | 6-8 GB |
| Modpacks "kitchen sink" (500+ mods, pregeneración profunda) | 8-10 GB |

Por encima de 10 GB rara vez ayuda salvo que la documentación del pack lo diga explícitamente. Algunos packs sí piden más (combos de mods hambrientos de memoria como Better End con NetherEx); sigue lo que el pack recomienda.

## Cómo funciona el slider de GDLauncher

Abre una instancia, pulsa la pestaña Settings, baja hasta **Instance Java Memory**. Activa el toggle para habilitar el override por instancia y arrastra el slider; va de 1 GB hasta el total de RAM del sistema (con aviso por encima del 80 %). El launcher convierte el valor en `-Xmx<n>M` y se lo pasa a la JVM.

El mismo slider existe a nivel global en **Settings → Java → Java Memory**, como default para cualquier instancia que no lo sobrescriba. Deja el global bajo para uso casual y solo sube los modpacks pesados.

## Aikar's flags

Larga lista de argumentos JVM que tunean G1 para priorizar pausas cortas frente a throughput. Originalmente escritos para servidores Minecraft, útiles también en clientes moddeados. Tienen forma de:

```
-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ...
```

GDLauncher no los aplica por defecto. Para usarlos, pega la cadena entera en el campo **Instance Java Arguments** de la pestaña Settings de la instancia (o en **Settings → Java → Java Arguments** para aplicarlos globalmente). Los efectos varían; el beneficio más constante es menos pausas largas en heaps de 6-10 GB.

Algunas precauciones:

- Tuneados para versiones de Java antiguas. En Java 17+ los defaults ya son bastante buenos y las ganancias de Aikar son menores.
- Asumen patrones de asignación tipo servidor. En un escritorio con heap pequeño pueden hacer daño.
- No añaden memoria ni cambian cuánto usa el juego; solo cambian cómo se comporta el colector.

Sin un motivo concreto, deja el campo Java Arguments tal como lo dejó GDLauncher.

## Diagnosticar stutter

Si Minecraft hace pausas de cientos de milisegundos cada pocos segundos:

1. Abre la pantalla de debug F3. Mira la línea "Mem:" (arriba a la derecha). Si oscila rápido entre valores bajos y altos, es churn de GC.
2. Baja Xmx 1-2 GB y prueba. Contraintuitivo, pero heaps más pequeños hacen GC más rápido.
3. Si un mod concreto asigna como loco (algunos de pregeneración o de render), aparecerá en profilers del lado mod (Spark, JmxMC). El mod quizá necesite actualización.
4. Si tu CPU está al 100% durante las pausas, el GC trabaja muy duro. Baja Xmx más o quita mods hambrientos de memoria.

## TL;DR

- Pon Java Memory en *lo justo*, no en lo que tengas.
- El cuello de botella es el tiempo de pausa GC, no el tamaño bruto del heap.
- Los Aikar's flags ayudan en heaps grandes pero no son magia.
- 4-6 GB es lo apropiado para casi todo lo moddeado.
