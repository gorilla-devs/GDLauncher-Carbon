---
title: "Mod loaders comparados: Forge, NeoForge, Fabric, Quilt"
description: "GDLauncher soporta cuatro mod loaders de Minecraft. Qué es cada uno, en qué se diferencian, y cuál elegir para un mod o modpack concreto."
faq:
  - question: "¿Qué mod loader debo usar para Minecraft?"
    answer: "El que pida tu mod o modpack, en la práctica eso decide casi siempre. Si vas a elegir desde cero: Fabric para mods de rendimiento/QoL en versiones recientes, NeoForge para grandes mods de contenido modernos, Forge para modpacks viejos y la mayor biblioteca histórica."
  - question: "¿Los mods de Forge funcionan en Fabric?"
    answer: "No. Forge y Fabric no son intercambiables. Un mod escrito para uno no cargará en el otro. Muchos mods populares ofrecen builds separadas para Forge y Fabric; revisa la página del mod para ver loaders y versiones soportadas."
  - question: "¿NeoForge sustituye a Forge?"
    answer: "En la práctica sí para versiones Minecraft nuevas. NeoForge nació en 2023 como fork de Forge con la misma API; los dos se han ido separando desde entonces, un mod actual suele publicar una build NeoForge aparte en lugar de correr en los dos. Desde 1.20.4 muchos mods Forge se compilan ahora para NeoForge. Para 1.20.1 y anteriores, Forge sigue siendo el estándar."
  - question: "¿Los mods de Fabric funcionan en Quilt?"
    answer: "La mayoría sí. Quilt es un fork de Fabric y ejecuta mods Fabric directamente. Algunos mods solo de Quilt usan APIs Quilt y no funcionan en Fabric. Si tienes una lista de mods, todos Fabric, cualquiera de los dos loaders sirve y da el mismo resultado."
  - question: "¿Puedo correr dos mod loaders a la vez?"
    answer: "En la misma instancia no. Cada instancia toma exactamente un loader. Para usar ambos crea dos instancias. El sistema de instancias de GDLauncher está pensado para eso: una Forge, una Fabric, cambio con un clic."
---

# Mod loaders comparados: Forge, NeoForge, Fabric, Quilt

## Los cuatro mod loaders que soporta GDLauncher

GDLauncher puede instalar y ejecutar cualquiera de los cuatro grandes mod loaders de Minecraft Java Edition, además de vanilla (sin loader). Al crear una instancia custom eliges uno. Al instalar un modpack, el loader es el que indique el manifest del pack.

### Forge

El mod loader original, iniciado en 2011. Forge tiene la mayor biblioteca histórica de mods, especialmente para mods pesados en contenido (árboles tecnológicos, sistemas de magia, mundos nuevos: Tinkers' Construct, Twilight Forest, Create en versiones antiguas). También es el que apuntan la mayoría de modpacks antiguos.

Forge se actualiza más lento que Fabric. Las nuevas versiones de Minecraft ven a menudo el release Forge semanas o meses después.

### NeoForge

Un fork de 2023 de Forge, nacido de una división en la comunidad Forge. NeoForge mantiene el estilo de API de Forge (los mods son en general source-compatibles) pero saca releases más rápido y a él ha migrado buena parte del desarrollo de mods Forge.

En Minecraft 1.20.4 y posteriores, NeoForge es el más activo de los dos. Muchos mods grandes publican ya builds NeoForge a la par con Forge o en lugar de Forge.

### Fabric

Filosofía distinta: pequeño, rápido, modular. Fabric llega casi el día que sale una nueva versión Minecraft, a veces en cuestión de horas. Su ecosistema mod tiende a rendimiento (Sodium, Lithium, FerriteCore), QoL (Mod Menu, Iris), y mods de contenido moderno de alta calidad.

Si el rendimiento es prioridad o juegas en una versión Minecraft de última hora, Fabric es el loader.

### Quilt

Fork de 2022 de Fabric con un modelo de gobernanza distinto y algunas APIs extra. Quilt ejecuta mods Fabric directamente, así que la diferencia práctica es pequeña: pon Quilt si un mod concreto lo exige, si no Fabric vale igual.

Quilt tiene un ecosistema dedicado más pequeño que Fabric pero es casi del todo compatible con el contenido Fabric.

## Matriz de compatibilidad

| Mod construido para | Corre en Forge | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | Sí | A veces (NeoForge en sus inicios podía correr mods Forge sin tocar, porque era un fork recién hecho; las APIs se han ido separando desde entonces, y la mayoría de mods Forge actuales necesitan una build NeoForge) | No | No |
| NeoForge | No | Sí | No | No |
| Fabric | No | No | Sí | Sí |
| Quilt | No | No | Mods Quilt-API: no; resto: sí | Sí |

No existe puente cross-loader en producción. Los JARs que pones en `mods/` deben corresponder al loader de la instancia.

## Elegir para una instancia nueva

Normalmente los mods o el modpack eligen por ti:

- **¿Instalas un modpack de CurseForge o Modrinth?** GDLauncher lee el manifest e instala el loader indicado. Sin opción.
- **¿Montas una instancia custom alrededor de un mod concreto?** Mira la página del mod. Si pone "Fabric 1.21.x", creas una Fabric 1.21.x.
- **¿Montas una custom con una lista de mods?** Mira para cada uno qué loaders soporta, busca la intersección. La mayoría de mods de rendimiento son Fabric-only; los grandes de contenido, Forge/NeoForge.

Sin restricción y como recomendación: **Fabric** para setups orientados a rendimiento/visual, **NeoForge** para survival con mucho contenido modded.

## Cambiar el loader en una instancia existente

GDLauncher permite cambiar el mod loader de una instancia tras su creación, ver [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). Resumen: clic derecho en la instancia → Edit → elige otro loader. La carpeta mods no se vacía, así que los JAR del loader anterior se quedan; quita a mano los incompatibles antes de lanzar.

## Nota sobre versiones de loader

Cada loader tiene su flujo de versiones independiente de Minecraft. Cuando eliges "Forge" también eliges una versión de Forge (algo como `47.2.0` para Minecraft 1.20.1). Para los mods, la versión del loader rara vez importa más allá de "la misma mayor que espera el pack", aunque algunos exigen un build mínimo. La página CurseForge o Modrinth del mod lo dirá.
