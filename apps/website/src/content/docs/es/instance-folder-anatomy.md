---
title: "Anatomía de una carpeta de instancia"
description: "Qué hay dentro de una carpeta de instancia de GDLauncher. Dónde viven mods, mundos, configs, capturas y logs, y qué se puede borrar a mano sin riesgo."
faq:
  - question: "¿Dónde se guardan mis mundos de Minecraft?"
    answer: "Cada mundo está en <runtime_path>/instances/<instance>/instance/saves/<nombre-mundo>/. La carpeta saves/ se crea la primera vez que generas o importas un mundo. Clic derecho en la instancia → Open Folder, luego entra en instance/saves. Para hacer backup, copia la carpeta del mundo entera."
  - question: "¿Dónde están los crash reports?"
    answer: "Dentro de la instancia: instance/crash-reports/. La carpeta solo existe si Minecraft ha crasheado de verdad; si nunca has visto el juego morir feo, aún no está. Cada report es un archivo de texto con timestamp (crash-<fecha>-server.txt, etc.)."
  - question: "¿Dónde van los JAR de mods?"
    answer: "instance/mods/ dentro de la carpeta de instancia. Soltar un JAR ahí a mano funciona, pero evita el tracking de GDLauncher; prefiere la pestaña Addons salvo motivo concreto. Las instancias modpack bloqueadas inhabilitan el botón Add, pero una copia manual de archivo pasa."
  - question: "¿Qué diferencia hay entre los logs del launcher y los de Minecraft?"
    answer: "Dos juegos distintos. Los logs de sesión del launcher viven en <instance>/logs/. Los logs del juego Minecraft (latest.log, crash-reports, todo lo que escribe el juego) un nivel más abajo, en <instance>/instance/logs/ y <instance>/instance/crash-reports/."
  - question: "¿Qué son instance.json y packinfo.json?"
    answer: "Archivos de metadatos que GDLauncher escribe en la raíz de cada instancia. instance.json contiene el nombre, el icono, el mod loader, la versión de Minecraft, la última vez jugada y el tiempo total. packinfo.json (solo presente para instancias pareadas con un modpack CurseForge o Modrinth) rastrea qué archivos pertenecen al pack para distinguir mods del pack y mods añadidos a mano. No los borres a mano."
---

# Anatomía de una carpeta de instancia

## Dónde vive la carpeta de instancia

Cada instancia de GDLauncher es una subcarpeta del Runtime Path:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← la carpeta de instancia
```

`<shortpath>` es una versión sanitizada del nombre visible. Clic derecho en la instancia en GDLauncher → **Open Folder** te lleva ahí directo.

## Qué hay dentro

La carpeta de instancia se divide en unos pocos elementos que GDLauncher rastrea a nivel raíz, más una subcarpeta `instance/` que es el directorio de juego real de Minecraft. Algunas cosas están siempre presentes; muchas se crean a demanda, la primera vez que algo escribe en ellas.

```
<shortpath>/
├── instance.json          ← metadatos GDLauncher de la instancia (siempre presente)
├── packinfo.json          ← info de pareo de modpack (solo para modpacks pareados)
├── icon.png | icon.webp   ← icono custom (solo si pusiste uno)
├── logs/                  ← logs del launcher GDLauncher por instancia
└── instance/              ← directorio de juego de Minecraft
    ├── mods/              ← JARs de mods
    ├── config/            ← configs de mods
    ├── shaderpacks/       ← shader packs (si has instalado alguno)
    ├── options.txt        ← ajustes del cliente Minecraft (tras el primer arranque)
    ├── logs/              ← logs de sesión de Minecraft (latest.log, etc.)
    ├── saves/             ← mundos (se crea cuando generas uno)
    ├── screenshots/       ← capturas F2 (creada en el primer F2)
    ├── crash-reports/     ← crash dumps (solo si el juego ha crasheado)
    ├── resourcepacks/     ← resource packs propios (al añadir uno)
    ├── datapacks/         ← data packs globales (los por mundo viven en saves/<mundo>/datapacks/)
    └── (específico del pack) ← kubejs/, defaultconfigs/, packmenu/, etc., solo si el modpack los aporta
```

No te sorprendas si en una instancia recién creada faltan algunos de estos. El launcher y Minecraft solo crean lo que necesitan, cuando lo necesitan. Una instancia que nunca has jugado no tiene `saves/`, ni `screenshots/`, ni `options.txt`. Una instancia vanilla no tiene `mods/` ni `config/`.

## Qué tiene cada cosa

### Archivos raíz

- **`instance.json`**: metadatos de GDLauncher, nombre, ruta del icono, mod loader y versión, versión de Minecraft, cuándo se creó, cuándo se jugó por última vez, tiempo total de juego. Siempre presente.
- **`packinfo.json`**: manifest de hashes de los archivos venidos del modpack. Permite al launcher distinguir mods del pack de los añadidos por ti. Solo presente para instancias pareadas con un modpack CurseForge o Modrinth.
- **`icon.png`** o **`icon.webp`**: el icono custom que subiste. Falta si usas el de por defecto.

### `logs/` (raíz)

Logs propios de GDLauncher por instancia. Lo que muestra la acción **View Logs** del menú contextual. Cubren el lanzamiento visto desde el *launcher* (argumentos Java, descargas de assets, install del mod loader, código de salida); útiles cuando el juego no llega a escribir su propio log.

### `instance/mods/`

Los archivos JAR de los mods. Minecraft carga todo lo que haya aquí al arrancar (según reglas del mod loader). El launcher rastrea qué mods pertenecen a una instancia en su base de datos (con nombre de archivo y hash de contenido como clave), no hay archivos sidecar. Los JARs dejados a mano también se reconocen; al launcher solo le faltan metadatos de CurseForge/Modrinth para ellos.

### `instance/config/`

Una subcarpeta o archivo por mod. Aquí persisten los ajustes de cada mod. La mayoría escribe `config/<modid>.toml` o una carpeta `config/<modid>/`. Editar a mano suele ser seguro; muchos mods releen al reiniciar.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

Asset packs. Resource packs para texturas y sonidos, shader packs para el renderizado (necesitan Iris/OptiFine instalados como mod), data packs para recetas/loot/funciones. Los data packs para un mundo concreto van en `saves/<mundo>/datapacks/`. Estas carpetas solo se crean cuando realmente tienes contenido para ellas.

### `instance/saves/`

Una subcarpeta por mundo. Dentro: `level.dat` (maestro), `region/` (datos de chunks), `playerdata/` (estado por jugador), `datapacks/` (data packs por mundo). Para backup, copia toda la carpeta `<mundo>/`. `saves/` aparece la primera vez que generas un mundo.

### `instance/screenshots/`

Todo lo capturado con F2 en juego. PNGs con timestamp como nombre. Creada la primera vez que sacas una captura.

### `instance/logs/` y `instance/crash-reports/`

La salida diagnóstica de Minecraft. `logs/latest.log` es siempre el último lanzamiento (rotado a `logs/<fecha>-1.log.gz` en el siguiente). `crash-reports/` guarda los crash dumps completos y solo aparece una vez ha habido un crash real.

### `instance/options.txt`

Ajustes del cliente Minecraft (gráficos, controles, sonido). Texto plano, key=value. Editable si te empeñas.

### Carpetas específicas de modpack

Muchos modpacks grandes traen carpetas extra. Las más comunes:

- **`kubejs/`**: scripts KubeJS (`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). Los autores de pack lo usan para ajustes runtime.
- **`defaultconfigs/`**: snapshot de "cómo deberían verse las configs por defecto". El script de arranque del pack copia entradas faltantes a `config/` en cada inicio.
- **`packmenu/`**: assets del menú principal con la temática del pack (botones custom, fondos, splash text).
- **`defaultsettings/`**: como `defaultconfigs/`, pero para `options.txt` y los keybinds.

Solo existen si el pack las trae. Vanilla y la mayoría de instancias custom no tienen ninguna.

## Qué es seguro borrar

| Carpeta | ¿Seguro? | Efecto |
|---|---|---|
| `instance/mods/` (un JAR concreto) | Sí | Ese mod se va. Mundos que lo usaban pueden romperse. |
| `instance/config/<modid>/` | Sí | Mod vuelve a defaults en el próximo arranque. |
| Contenido de `instance/resourcepacks/`, `instance/shaderpacks/` | Sí | El pack se va. |
| `instance/saves/<mundo>/` | Sí | Mundo borrado para siempre. Backup antes. |
| `instance/logs/`, `crash-reports/` | Sí | Libera disco. |
| `instance/screenshots/` | Sí | Adiós a viejas capturas. |
| `logs/` (logs launcher) | Sí | Igual. |
| `instance/options.txt` | Sí | Ajustes del juego a defaults. |
| `instance.json` | No | El launcher pierde el rastro de la instancia. |
| `packinfo.json` | Posible (despareja de hecho) | El launcher deja de tratar la instancia como modpack pareado. Mismo efecto que el botón Unpair en la pestaña Settings de la instancia, pero sucio. |
| Toda la carpeta `instance/` | No | La instancia queda rota. Usa Delete en la UI. |

## Nota sobre instancias modpack bloqueadas

La carpeta de instancia es solo una carpeta normal en disco; el bloqueo que aplica GDLauncher a instancias modpack se aplica en la UI, no en el sistema de archivos. Soltar un JAR en `instance/mods/` de una instancia bloqueada funciona mecánicamente, el launcher solo no lo verá vía la pestaña Addons. Para quitarlo hay que pasar de nuevo por el sistema de archivos. Para flujos más seguros, abre la instancia, ve a la pestaña **Settings** y pulsa **Unlock** en la sección Modpack.
