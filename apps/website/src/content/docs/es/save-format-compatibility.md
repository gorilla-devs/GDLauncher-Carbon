---
title: "Compatibilidad del formato de guardado"
description: "Por qué actualizar un mundo de Minecraft a una versión nueva suele ser de ida sin vuelta, cómo cambia realmente el formato de guardado, y cómo hacer backup correctamente antes."
faq:
  - question: "¿Puedo abrir un mundo de Minecraft 1.21 en 1.20?"
    answer: "No con seguridad. Minecraft solo migra mundos hacia adelante, nunca hacia atrás. Un mundo abierto en 1.21 tiene su level.dat y los region files reescritos al nuevo formato; las versiones antiguas se niegan a cargarlo o crashean. Si necesitas ambos, haz una copia del mundo antes de lanzar la versión más nueva."
  - question: "¿Avisa GDLauncher antes de actualizar un mundo?"
    answer: "El launcher en sí no, el aviso es del lado Minecraft. Al abrir un mundo guardado en una versión más antigua, Minecraft muestra un diálogo 'Este mundo fue guardado en una versión diferente' antes de cargar. Ese es el momento para echarte atrás y copiar la carpeta del mundo a otro sitio."
  - question: "¿Qué se reescribe al actualizar un mundo?"
    answer: "level.dat (metadatos del mundo), los region files en region/ (datos de chunks), playerdata/ (estado por jugador) y cualquier data pack propio del mundo. El campo Data Version en level.dat se actualiza al de la nueva versión de Minecraft; ese campo es el que leen las versiones para decidir si pueden abrir el mundo."
  - question: "¿Es imposible hacer downgrade?"
    answer: "En estricto sí. No hay un camino oficial de downgrade. Algunas herramientas comunitarias dicen revertir el Data Version pero no reescriben los chunks; el mundo queda parcialmente corrupto (biomas, bloques o entidades nuevos que la versión vieja no entiende). Trata las actualizaciones como de ida."
  - question: "¿Cómo respaldo un mundo antes de actualizar?"
    answer: "Clic derecho en la instancia en GDLauncher → Open Folder. Entra en instance/saves y copia la carpeta del mundo (con el nombre que aparece en la lista) a algún sitio fuera del directorio de la instancia. Guarda esa copia hasta estar seguro de que la versión actualizada funciona bien."
---

# Compatibilidad del formato de guardado

## Por qué cambian los formatos de guardado

El formato de archivo de mundos de Minecraft no es fijo. Cada gran actualización revisa la estructura de datos en disco. Bloques nuevos = IDs nuevas. Entidades nuevas = formas NBT nuevas. Biomas nuevos = registro de biomas nuevo. Tras bambalinas, cada mundo tiene un número llamado **Data Version** en `level.dat`, y Minecraft lo usa para decidir qué hacer al abrir el mundo.

Si la Data Version de tu mundo es más antigua que la de la versión actual de Minecraft, Minecraft ejecuta un paso único de **DataFixer** que reescribe el mundo al nuevo formato. Chunks, entidades, estados de bloques, datos de jugador, todo se convierte al esquema nuevo. La Data Version en `level.dat` se actualiza al nuevo valor.

Esta conversión es **destructiva y de ida**. Una vez reescritos los chunks, la versión más antigua de Minecraft ya no los puede leer.

## Lo que significa realmente "de ida"

Imagina un mundo 1.20.1. Lo abres en 1.21. Minecraft muestra el aviso "versión diferente", haces clic en "Convertir" (o carga igualmente), y empieza el juego. Detrás:

- `level.dat` se reescribe de modo que su campo `DataVersion` corresponda a 1.21 en vez de 1.20.1.
- Cada region file en `region/` que se carga (al menos todo dentro de la distancia de vista) se reescribe chunk a chunk.
- Bloques nuevos de 1.21 como Crafter o Trial Spawner pueden existir en el mundo; no existen en el registro de bloques 1.20.1.
- Entidades 1.20.1 y tile entities existentes migran a los esquemas 1.21.

Si ahora intentas abrir la misma carpeta en 1.20.1:

- Minecraft compara su `DataVersion` con la propia y se niega a cargar (o crashea cargando ciertos chunks).
- Incluso saltándose el check de versión, los bloques exclusivos de 1.21 aparecerían como bloques faltantes/error en el cliente más antiguo.

De ahí: **actualizar un mundo a una versión más nueva es permanente**. El único rollback seguro es restaurar desde un backup hecho *antes* de la actualización.

## Los mundos moddeados son peor

El DataFixer del Minecraft vanilla al menos es exhaustivo y bien probado. Los saves moddeados añaden otra capa de riesgo:

- Los mods retirados dejan errores de **bloque faltante** y **entidad faltante**. El mundo carga, pero los cubos que eran bloques mod se vuelven placeholders "?".
- Los mods sustituidos (versión vieja → nueva) cambian a veces IDs de bloque o claves NBT de entidad. La migración corre por cuenta del autor y no siempre es fluida.
- Saltos grandes de versión Minecraft dentro de un modpack (Forge 1.20.1 → 1.21.x, por ejemplo) coinciden a menudo con que la mayoría de mods migra a APIs completamente nuevas. Mundos que funcionaban en la vieja pueden tener comportamiento indefinido en la nueva.

Para instancias moddeadas, trata cualquier salto de versión como potencial evento de corrupción y haz backup antes.

## Respaldar un mundo bien

El backup más simple es una copia de carpeta. En GDLauncher:

1. Clic derecho en la instancia → **Open Folder**.
2. Abre `instance/saves/`.
3. Copia la carpeta con el nombre de tu mundo (el mismo de la lista de mundos) a algún sitio fuera de la instancia. Otro disco, una carpeta `~/Documents/mc-backups/`, donde sea que no vaya a sobreescribirse.

Esa copia es una instantánea del mundo en el momento de copiar. Guárdala hasta estar seguro de que la nueva versión funciona.

Para backups continuos, herramientas de terceros como FTBBackups (un mod) hacen instantáneas en juego a intervalos. Escriben en `backups/` dentro de la instancia y son restaurables por instantánea.

## Qué significan los avisos de "versión snapshot"

Si por error abres un mundo guardado en una snapshot Minecraft (build de desarrollo, como `24w11a`), el juego oficial muestra un aviso extra porque las Data Versions de snapshots a veces van por delante de cualquier versión estable. Un mundo de una snapshot puede no abrirse en la siguiente estable si la snapshot introdujo cambios de formato que se revirtieron antes del release. El camino seguro: no jugar mundos importantes en snapshots, o aceptar que el mundo queda atado a la snapshot.

## TL;DR

- Las actualizaciones de mundo son de ida; backup antes de abrir en una versión más nueva.
- Los mundos moddeados son más frágiles; trata cualquier salto de versión como potencial evento de corrupción.
- En actualizaciones de modpack que suben la versión de Minecraft, copia primero toda la carpeta saves y luego actualiza.
