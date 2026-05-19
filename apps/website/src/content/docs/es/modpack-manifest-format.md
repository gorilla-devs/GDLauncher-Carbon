---
title: "Formato del manifest de un modpack"
description: "Lo que hay dentro de los tres archivos de modpack que GDLauncher lee y escribe: CurseForge .zip, Modrinth .mrpack y el propio .gdlpack de GDLauncher basado en hashes. Referencia campo a campo, comparativa lado a lado y cómo resuelve cada formato los mods al instalar."
faq:
  - question: "¿Qué es un manifest de modpack?"
    answer: "Un archivo JSON dentro del archivo de modpack que le dice al launcher lo que contiene el pack: versión de Minecraft, mod loader, lista de mods y qué archivos incluidos (overrides) extraer dentro de la instancia. GDLauncher lee el manifest lo primero cuando instala un pack."
  - question: "¿Cuál es la diferencia entre los formatos CurseForge, Modrinth y GDLauncher?"
    answer: "Los tres son zips con un manifest JSON. CurseForge usa manifest.json con IDs de proyecto/archivo de CurseForge. Modrinth usa modrinth.index.json con URLs de descarga directas más hashes. GDLauncher usa gdlpack.json sólo con hashes de contenido (sha512 + sha1 + murmur2) y resuelve cada archivo contra la plataforma que realmente lo tiene. No son intercambiables; el launcher despacha según el JSON que encuentra en la raíz del archivo."
  - question: "¿Qué es un .gdlpack y por qué existe?"
    answer: "El formato propio de modpack de GDLauncher. Cada mod se identifica por hashes de contenido, así un mismo pack funciona viva el mod en CurseForge, en Modrinth o en ambos, sin URLs que se pudran. También tiene soporte de primera para funcionalidades opcionales (paquetes salteables de mods + configs) e iconos embebidos. Recomendado cuando ambos lados usan GDLauncher; CurseForge .zip sigue siendo lo más seguro para distribución entre launchers."
  - question: "¿Dónde encuentro el manifest en una instancia instalada?"
    answer: "GDLauncher no conserva el archivo original tras instalar. El contenido del pack acaba descomprimido en la carpeta de la instancia. El manifest con el que trabaja GDLauncher se guarda en la base de datos del launcher contra la instancia; no es navegable como archivo. Para inspeccionar un manifest antes de instalar, abre el .zip / .mrpack / .gdlpack con cualquier herramienta de archivo y lee el JSON de dentro directamente."
  - question: "¿Puedo editar un manifest?"
    answer: "Puedes editar uno dentro de un archivo si estás construyendo tu propio pack, pero no puedes editar el manifest de una instancia instalada desde la UI del launcher. Para cambiar qué mods tiene una instancia, usa la pestaña Addons (o desbloquea primero una instancia bloqueada). Para cambiar la versión de un pack emparejado, abre la instancia, ve a la pestaña Settings y haz clic en Change Modpack Version."
  - question: "¿Qué es un 'override' en jerga de packs?"
    answer: "Archivos que el pack trae empaquetados de antemano, no mods. Configs por defecto, scripts, resource packs, a veces un mundo inicial. Se extraen encima de los mods en la carpeta de la instancia cuando el pack se instala. CurseForge los lista en el campo overrides; Modrinth marca archivos del pack con env.client=required; GDLPack usa una carpeta overrides (configurable vía el manifest)."
---

# Formato del manifest de un modpack

## Por qué importa

Normalmente no necesitas saber qué hay dentro de un archivo de modpack. Pero cuando algo va mal durante la instalación verás mensajes como «manifest», «invalid manifest format», «manifest missing field» o «manifest version unsupported», y querrás saber a qué se refieren. Esta página también es útil si estás construyendo un pack para compartir o intentando averiguar por qué el mismo pack se instala limpio en un launcher y se rompe en otro.

GDLauncher lee y escribe tres formatos:

- **CurseForge** (`.zip`, con `manifest.json` dentro).
- **Modrinth** (`.mrpack`, con `modrinth.index.json` dentro).
- **GDLauncher** (`.gdlpack`, con `gdlpack.json` dentro).

Los tres son archivos ZIP planos por debajo; la extensión sólo indica qué esquema de manifest esperar.

## Qué hay dentro de un archivo de modpack

La forma general es la misma para los tres formatos:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← manifest específico del formato
├── overrides/            ← configs, scripts, mundo inicial opcional
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (sólo CurseForge, opcional, lista de mods legible)
```

El manifest es el único archivo que el launcher necesita estrictamente para saber instalar el pack. La carpeta overrides es contenido a descomprimir en la instancia resultante.

## CurseForge: `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

Campos clave:

- `minecraft.version`: la versión de Minecraft a la que apunta el pack.
- `minecraft.modLoaders`: qué loader y qué versión (formato: `<loader>-<version>`).
- `files`: cada mod se referencia por `projectID` y `fileID` de CurseForge. GDLauncher los resuelve contra la API de CurseForge para descargar.
- `overrides`: nombre de la carpeta dentro del zip cuyo contenido se copia a la instancia tras completar las descargas de mods.

Si el `projectID` o `fileID` de un archivo ya no existe en CurseForge (el autor lo retiró), la instalación falla con un error «file not found» para ese mod en concreto.

## Modrinth: `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

Diferencias clave con CurseForge:

- `dependencies` declara las versiones de Minecraft y loader directamente (sin objeto anidado).
- Cada entrada `files` incluye la **URL de descarga exacta** y el **hash**, así las instalaciones no dependen de una llamada API de metadatos.
- Cada archivo declara si es requerido en cliente, servidor o ambos vía `env`.

Este formato es más simple y más fácil de instalar sin conexión (las URLs vienen incrustadas). También es más estricto: un mismatch de hash supone rechazo duro de instalación, no advertencia.

## GDLauncher: `gdlpack.json`

El formato propio de pack de GDLauncher es el que GDLauncher escribe cuando eliges **GDLauncher .gdlpack** en **Export Instance**. La propiedad definitoria: cada mod se identifica sólo por sus hashes de contenido, no por un ID específico de plataforma o una URL. GDLauncher resuelve cada hash contra CurseForge y Modrinth al instalar y descarga desde la plataforma que tenga el archivo.

### Por qué hashes y no IDs o URLs

- **Multiplataforma desde una sola fuente.** Un mod que vive tanto en CurseForge como en Modrinth tiene IDs diferentes en cada lado. A un `.gdlpack` le da igual; una lista de hashes funciona en cualquiera.
- **Sin pudrición de URLs.** Las URLs del CDN de Modrinth están direccionadas por contenido y son estables, pero incrustar URLs fija un único origen de descarga. Los hashes permiten al launcher hacer fallback cuando una plataforma está caída.
- **Verificación por construcción.** Cada byte que se escribe en tu carpeta `mods/` se comprueba contra el hash del manifest. No hay forma de sustituir silenciosamente otro JAR.

### Estructura del archivo

```
mypack.gdlpack/
├── gdlpack.json          ← el manifest
├── .gdl/
│   └── icon.png          ← icono embebido del pack (opcional)
└── overrides/            ← archivos empaquetados (configs, scripts, mods no resolubles)
    ├── config/
    └── ...
```

El manifest está en la raíz. La carpeta `.gdl/` contiene metadatos que el launcher embebe (actualmente sólo el icono). Los overrides funcionan igual que en CurseForge: el contenido se extrae en la instancia resultante tras el paso de resolución de mods.

### Manifest mínimo

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

Sólo `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries` y `overrides` son requeridos para cargar.

### Manifest completo, anotado

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### Campos de primer nivel

| Campo | Tipo | Requerido | Significado |
|---|---|---|---|
| `formatVersion` | integer | sí | Versión del esquema del manifest. Actualmente siempre `1`. |
| `name` | string | sí | Nombre del pack para mostrar. Sirve como nombre de instancia por defecto al importar. |
| `version` | string | no | Versión del pack (semver recomendado, ej. `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | no | Tagline de una línea. |
| `author` | string | no | Autor o equipo del pack. |
| `createdAt` | timestamp RFC 3339 | sí | Cuándo se exportó el archivo. |
| `icon` | string | no | Ruta dentro del archivo al archivo del icono (ej. `.gdl/icon.png`). |
| `dependencies` | object | sí | Requisitos de Minecraft y modloader (ver abajo). |
| `entries` | array | sí | Archivos de plataforma y funcionalidades opcionales (ver abajo). Un array vacío es válido para un pack sólo de overrides. |
| `overrides` | string | sí (por defecto `"overrides"`) | Nombre del directorio dentro del archivo a extraer en la instancia. |
| `serverOverrides` | string | no | Directorio de overrides sólo de servidor. |
| `clientOverrides` | string | no | Directorio de overrides sólo de cliente. |
| `source` | object | no | Si este pack deriva de un pack de CurseForge o Modrinth, apunta al original (ver abajo). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: versión de Minecraft requerida (ej. `1.20.1`, `1.21.4`, `25w20a` para snapshots).
- `modloaders`: cero o más entradas de loader. Cada una tiene:
  - `type`: uno de `forge`, `neoforge`, `fabric`, `quilt` (en minúsculas).
  - `version`: versión exacta del loader contra la que se construyó el pack.
  - `primary`: marca el loader recomendado cuando hay varios listados (ej. un pack compatible con Fabric y Quilt).

Un pack vanilla tiene un array `modloaders` vacío.

#### `entries`

Dos tipos de entrada, distinguidos por el campo `type`:

**`platform`**, un mod requerido resuelto vía hash:

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

Los tres hashes deben estar presentes. Cada uno se usa diferente en la resolución:

| Hash | Usado para |
|---|---|
| `sha512` | Resolución Modrinth (endpoint `version_file` de Modrinth), verificación primaria de integridad del archivo descargado. |
| `sha1` | Resolución Modrinth (fallback), lo usa el propio Minecraft al verificar el archivo al lanzar. |
| `murmur2` | Resolución CurseForge (endpoint `fingerprints` de CurseForge). |

Al instalar, el launcher prueba primero Modrinth (consulta sha512), cae a CurseForge (fingerprint murmur2), y falla la entrada si ninguna plataforma tiene el archivo. Los archivos resueltos siempre se descargan desde la plataforma que respondió.

**`optional`**, un grupo salteable de mods y/o rutas de overrides que el usuario puede incluir o excluir:

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description`: se muestra al usuario en la vista previa de importación para que decida si incluir esta funcionalidad.
- `platforms`: cero o más entradas hash que sólo se descargan si la funcionalidad se incluye.
- `overridePaths`: cero o más rutas (archivos o carpetas) dentro del directorio `overrides/` que sólo se extraen si la funcionalidad se incluye. Las rutas son relativas a `overrides/`.

Una funcionalidad puede tener sólo archivos de plataforma, sólo overrides, o ambos. Sirve para agrupar contenido opcional relacionado: un mod de shaders más su config, un preset hardcore que son sólo configs, un resource pack opcional.

#### `source`

Cuando un pack se exportó desde una instancia de modpack de CurseForge o Modrinth existente, GDLauncher registra de dónde viene:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

o

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

Es informativo; el launcher no lo usa para descargar nada (de eso ya se encarga el array `entries`). Existe para que los derivados de packs públicos sigan siendo atribuibles al original.

### Exports con bundle vs sólo manifest

Al exportar una instancia a `.gdlpack`, el toggle **Bundle Addons** en el asistente de exportación decide qué va a dónde:

- **Bundle apagado (sólo manifest).** Cada mod que GDLauncher puede resolver vía CurseForge o Modrinth se vuelve una entrada `platform` en `entries`. El JAR real *no* se incluye en el archivo. El launcher del destinatario lo redescarga desde la plataforma al importar. Los mods que GDLauncher no puede resolver (ej. JARs que pusiste a mano) se empaquetan en `overrides/mods/` como fallback. Resultado: archivo pequeño, el destinatario necesita internet al instalar.
- **Bundle encendido (autocontenido).** No se emite ninguna entrada `platform`; cada mod se copia a `overrides/mods/`. Resultado: archivo más grande (a menudo cientos de MB a varios GB), instalable sin conexión una vez los assets de Minecraft están en caché.

Los resource packs y shader packs se comportan igual: los resolubles se vuelven entradas `platform` con bundle apagado, todo se empaqueta directamente con bundle encendido.

## Lado a lado: los tres formatos

| Propiedad | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| Nombre del manifest | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Identificación de mods | Project + File ID de CurseForge | URL de descarga + hash | Sólo hashes de contenido |
| Resuelve desde | API CurseForge | URL embebida en el manifest | CurseForge **o** Modrinth, la que responda |
| Instalación sin conexión | No (requiere CDN) | Sí (las URLs aún pueden necesitar CDN) | Sí cuando `Bundle Addons` está activado |
| Funcionalidades opcionales | Flag `required: false` por mod | `env` por archivo (client/server) | Entrada `optional` dedicada con varios archivos |
| Carpeta de overrides | `overrides/` | ruta por archivo | `overrides/` (configurable) |
| Archivos sólo de servidor | Pack separado | `env.server` | Directorio `serverOverrides` |
| Icono | Externo | Externo | Embebido en `.gdl/icon.<ext>` |
| Seguimiento de origen | Ninguno | Ninguno | Campo `source` |
| Portabilidad | La más amplia (la mayoría de launchers lo leen) | Launchers compatibles con Modrinth | GDLauncher |

## Overrides, en detalle

Cuando el manifest referencia una carpeta de overrides o archivos del pack marcados como client-required, el launcher los extrae a la instancia tras el paso de resolución de mods. Así es como los packs incluyen:

- Configs por defecto de los mods (para que el pack se juegue igual recién instalado para todos).
- Scripts de KubeJS o CraftTweaker.
- Un mundo inicial (a veces).
- Resource packs que el pack espera tener activos.
- `options.txt` con ajustes de juego adaptados al pack.

Los overrides ganan a los defaults autogenerados pero pierden frente a lo que el jugador cambie manualmente después. GDLPack soporta además directorios `serverOverrides` y `clientOverrides` para archivos que sólo deberían acabar en un lado, útil para packs pensados para entregar un bundle cliente y servidor a juego.

## Cuando los manifests se quedan desactualizados

Un manifest de pack es una instantánea de lo que el autor publicó por última vez. Si un mod referenciado por el manifest se elimina de la plataforma fuente después de construido el manifest, la instalación de esa versión del pack fallará hasta que el autor publique una nueva. Esto es lo que pasa cuando una versión vieja del pack se instala limpia pero una más nueva se rompe: la más nueva referencia algo que ya no está disponible.

El fix es del lado del autor; el launcher sólo puede hacer lo que dice el manifest. GDLPack tiene una ventaja parcial aquí porque no está atado a una sola plataforma: si un mod se retira de CurseForge pero sigue en Modrinth (o al revés), el mismo array `entries` sigue resolviendo.

## Leer un manifest tú mismo

No necesitas nada especial. Renombra `mypack.mrpack` o `mypack.gdlpack` a `mypack.zip`, ábrelo con cualquier herramienta de archivo, y mira el archivo JSON de dentro (`modrinth.index.json` o `gdlpack.json`). Igual con los `.zip` de CurseForge y `manifest.json`. Los tres son JSON plano, legible en cualquier editor de texto.

## Construir un pack

Si estás construyendo un pack para compartir, el camino más fácil es configurar la instancia en GDLauncher y luego usar **Export Instance** (clic derecho → Export Instance). El asistente de exportación te deja elegir el formato destino: CurseForge `.zip`, Modrinth `.mrpack` o el propio `.gdlpack` de GDLauncher. Elige `.gdlpack` si el destinatario también usa GDLauncher y quieres funcionalidades opcionales o iconos embebidos; elige `.zip` para la compatibilidad entre launchers más amplia.
