---
title: "Instancias de modpack bloqueadas"
description: "Qué significa que una instancia de modpack esté bloqueada, por qué GDLauncher la bloquea, y cómo desbloquearla (Unlock) o desemparejarla (Unpair) cuando lo necesites."
---

## ¿Qué es una instancia bloqueada?

Cuando instalas un modpack desde CurseForge o Modrinth en GDLauncher, la instancia queda **bloqueada (locked)** por defecto. Aparece un icono de candado junto a la instancia, y las acciones que cambiarían el contenido del pack, añadir, quitar o actualizar mods individuales, quedan deshabilitadas. Puedes seguir jugando, cambiar Java o RAM, hacer capturas y todo lo demás; el bloqueo solo protege el *conjunto de mods gestionado por el pack*.

El bloqueo existe porque un modpack es una colección de mods probada y con versiones fijadas. Los autores del pack eligen su lista de mods con cuidado y fijan versiones concretas para garantizar la compatibilidad. Si cambias un mod por una versión más nueva, puedes romper otro mod hermano que dependía de la versión antigua. El bloqueo evita ese error antes de que ocurra.

## Qué puedes y qué no puedes hacer en bloqueo

Con la instancia bloqueada **puedes**:

- Lanzar y jugar la instancia.
- Cambiar la RAM, los argumentos de Java y el Java Override.
- Hacer capturas y revisar logs.
- Cambiar el nombre y el icono de la instancia (Edit Instance).
- Actualizar el modpack entero a una release más nueva (Settings → Change Modpack Version).

**No puedes**:

- Añadir nada desde la pestaña Addons, eso incluye **mods, shaders, resource packs, data packs y worlds**. Mientras el bloqueo esté activo, el botón Add aparece deshabilitado en todos los tipos de addon.
- Quitar o desactivar un mod o addon gestionado por el pack.
- Actualizar individualmente mods gestionados por el pack.

Las pestañas Mods y Addons muestran un aviso «Esta instancia está bloqueada, no se pueden aplicar cambios» junto a las acciones deshabilitadas. El botón Install del navegador de Addons queda también bloqueado en instancias bloqueadas.

## Tres estados: Locked / Unlocked / Unpaired

Estos tres términos aparecen en GDLauncher y no son sinónimos.

- **Locked (bloqueada)**: la instancia está emparejada con un modpack de CurseForge o Modrinth y el conjunto de mods del pack es de solo lectura. Estado por defecto tras la instalación.
- **Unlocked (desbloqueada)**: sigue emparejada con el modpack (el nombre y la versión se siguen rastreando), pero el set de mods es libremente editable. GDLauncher recuerda el pack, así que puedes seguir actualizando a una nueva release más adelante; pero la coherencia del set queda en tus manos.
- **Unpaired (desemparejada)**: ya no está asociada al modpack. La instancia se convierte en una instancia custom, mismos archivos, pero GDLauncher no rastrea actualizaciones del pack ni la trata como instancia modpack. De Unlocked a Unpaired es un viaje sin vuelta.

## Cómo desbloquear una instancia (Unlock)

1. Abre la instancia y pulsa el icono del engranaje (o clic derecho en la instancia → Settings).
2. Ve a la sección **Modpack Info** arriba de la página de configuración. Verás el icono, el nombre y la versión actual del pack, con una fila de botones debajo.
3. Pulsa **Unlock** (el botón con el icono de candado). La instancia pasa al estado desbloqueado al instante.

Una vez desbloqueada, la cabecera de la sección cambia a «Unlocked» con el candado abierto. Puedes volver a bloquear desde el mismo flujo, pero en la práctica, una vez que has tocado el set de mods, no suele tener sentido.

## Cómo desemparejar (Unpair)

1. En la misma sección Modpack Info, pulsa **Unpair** (icono de rama git).
2. Confirma en el modal. GDLauncher avisa de que la acción es permanente.

Tras desemparejar, la sección Modpack Info desaparece. La instancia se convierte en una instancia custom y las opciones **Change Modpack Version** y **Reinstall** ya no aplican.

## Reinstall vs Unlock

La sección Modpack Info también incluye la acción **Reinstall**. Es distinta de Unlock y sirve para otra cosa: reinstala el modpack en su versión actual, sobrescribiendo los mods gestionados por el pack y las configs según el manifest. Úsalo para reparar una instalación rota (jar corrupto, configs borradas, etc.) sin perder tus mundos.

| Acción | Efecto en los mods del pack | Vínculo con el pack |
|--------|------------------------------|---------------------|
| Unlock | Se mantienen, pero editables | Se mantiene |
| Unpair | Se mantienen como archivos, ya no son «mods del pack» | Se elimina |
| Reinstall | Reset a la versión del manifest | Se mantiene |
| Change Modpack Version | Reemplazo por el manifest de la nueva versión | Se mantiene (nueva versión) |

## Cuándo desbloquear, y cuándo no

Desbloquea cuando:
- Un mod del pack tiene un bug crítico o un parche de seguridad y el pack no está actualizado.
- Quieres añadir tu propio mod, shader, resource pack, data pack o world encima de lo que trae el pack, el botón Add de la pestaña Addons está bloqueado por el candado, así que para instalarlo desde la UI hay que desbloquear.
- Estás manteniendo tú mismo un pack abandonado.

Mantén el bloqueo cuando:
- El pack se mantiene activamente, deja al autor gestionar las versiones y espera el siguiente release.
- Estás jugando una experiencia curada y no quieres desviarte del set previsto.

Patrón habitual: desbloquea un momento, instala tus añadidos y deja la instancia desbloqueada. Lo que añadiste tú permanece aunque vuelvas a bloquear, porque el bloqueo solo afecta al set *gestionado por el pack*, aunque en la práctica, una vez has empezado a mantener la instancia tú, hay poco motivo para volver a bloquear.

## Lo que el bloqueo no es

El bloqueo no es un sistema de permisos ni una frontera de seguridad. Es una barandilla para evitar ediciones accidentales en la UI. La carpeta de la instancia es una carpeta normal, cualquier cosa que escriba directamente en `mods` (una herramienta de terceros, una copia manual) salta el bloqueo por completo.

Los jars añadidos así aparecen en la pestaña Mods junto a los del pack. Para quitarlos hay que pasar por el sistema de archivos, no por la UI.

## Solución rápida de problemas

- **«No puedo actualizar un mod individual.»** El bloqueo funciona como debe. O bien Unlock (Settings → Unlock), o usa Change Modpack Version para actualizar el pack entero.
- **«Update All está en gris en una instancia bloqueada.»** La misma razón. Usa Change Modpack Version o desbloquea primero.
- **«Mi mod user-added sigue visible tras volver a bloquear.»** El bloqueo afecta a los mods del pack; los que añadiste tú permanecen siempre.
- **«Reinstall sobrescribió una config que había editado.»** Comportamiento esperado. Reinstall recrea el manifest. Haz copia de tus configs antes de Reinstall.
