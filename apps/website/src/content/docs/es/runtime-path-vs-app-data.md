---
title: "Runtime Path vs App Data Path"
description: "GDLauncher usa dos rutas distintas para guardar datos. Qué contiene cada una, por qué están separadas y cuál te conviene mover."
faq:
  - question: "¿Cuál es la diferencia entre App Data Path y Runtime Path?"
    answer: "El App Data Path es el directorio estándar por usuario donde Electron pone sus cachés y el marcador runtime_path_override. El Runtime Path es donde el core de GDLauncher guarda lo pesado: instancias, assets y bibliotecas de Minecraft, instalaciones de Java, la base de datos del launcher y los logs globales. Por defecto el Runtime Path está dentro del App Data Path, pero el que está pensado para moverse es el Runtime Path."
  - question: "¿Dónde está el App Data Path en mi sistema?"
    answer: "Windows: C:\\Users\\<tú>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<tú>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, o ~/.local/share/gdlauncher_carbon si XDG no está definido."
  - question: "¿Debo mover el Runtime Path o el App Data Path?"
    answer: "El Runtime Path. Es el que crece con cada instancia. El App Data Path se mantiene pequeño y ligado al perfil del SO. GDLauncher expone el cambio en Settings → Runtime Path; el App Data Path lo gestiona Electron y no se mueve desde la UI."
  - question: "¿Para qué sirve el archivo runtime_path_override?"
    answer: "Cuando cambias el Runtime Path, GDLauncher escribe un pequeño archivo de texto llamado runtime_path_override dentro del App Data Path. Contiene el nuevo Runtime Path; en cada arranque el launcher lo lee para saber dónde están tus datos. Si falta, el launcher cae al Runtime Path por defecto."
  - question: "¿Puedo compartir el Runtime Path entre dos equipos?"
    answer: "No. La base de datos del launcher contiene estado por máquina (rutas, tokens, instalaciones de Java) y no está pensada para uso simultáneo. Si quieres las mismas instancias en otro PC, cópialas manualmente o usa Cloud Instance Share."
---

# Runtime Path vs App Data Path

## Dos rutas, dos propósitos

GDLauncher reparte sus archivos en dos sitios: un **App Data Path** para las cosas pequeñas del lado de Electron, y un **Runtime Path** para lo gordo (instancias, assets, Java, base de datos). El Runtime Path es el que de vez en cuando quieres mover; el App Data Path casi nunca se toca.

### App Data Path

Es el directorio app por usuario estándar del SO, el que apunta `userData` de Electron. GDLauncher lo usa para:

- El marcador `runtime_path_override`, un archivo de texto de una línea que dice al launcher dónde vive en realidad el Runtime Path
- El Runtime Path por defecto, en un subdirectorio `data/`, si no lo has movido
- Las cachés propias de Chromium de Electron (Network/, GPUCache/, Cookies, etc.)
- Los logs del proceso principal de Electron

Ubicación estándar por SO:

- **Windows:** `C:\Users\<tú>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<tú>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, o `~/.local/share/gdlauncher_carbon` si no está definida

Sin el subdirectorio `data/`, suele ser pequeño. GDLauncher no expone ajuste para mover este directorio; las convenciones del SO y Electron esperan que esté ahí.

### Runtime Path (Core Module)

Donde el core en Rust de GDLauncher pone todo lo demás:

- Tus instancias (bajo `instances/`)
- Los assets compartidos de Minecraft (texturas, sonidos que entrega Mojang)
- Las bibliotecas compartidas (JARs de Mojang y los mod loaders)
- Java descargado por GDLauncher
- La base de datos del launcher, `gdl_conf.db`
- Los logs globales en `__gdl_logs__/`

Por defecto está en `<App Data Path>/data/`, así que vive dentro del App Data. Puedes moverlo a donde quieras en **Settings → Runtime Path**. Es la ruta que se hace grande, unos pocos modpacks pesados se pasan de 50 GB.

## Cuándo mover el Runtime Path

Dos razones habituales:

1. **Tu SSD se está llenando.** Mover instancias a un HDD o un segundo SSD más grande.
2. **Quieres backups aparte del perfil del SO.** Ponerlo en una unidad que respaldas por separado va bien; no lo sincronices activamente mientras juegas, el launcher y la herramienta de sync se pelearán por los archivos.

Para uso normal no necesitas mover el Runtime Path. La ubicación por defecto va bien.

## Cómo es el cambio

Abre **Settings → Runtime Path**. Escribe el nuevo destino, o usa el icono de carpeta para navegar. El botón de aplicar (el icono de flecha en círculo a la derecha de la fila) se enciende en cuanto la ruta es distinta de la actual y es válida. Pulsarlo abre un modal de confirmación mostrando la ruta antigua y la nueva.

Si la carpeta destino está vacía (o no existe aún), al confirmar arranca una migración completa: un overlay muestra escaneo, luego copia archivo a archivo, luego eliminación archivo a archivo del origen. No cierres la app ni apagues el equipo mientras esto corre. Al terminar el launcher se reinicia solo.

Si el destino ya contiene un Runtime Path de GDLauncher (lo moviste antes y quieres que una instalación nueva apunte a esos datos), el modal te avisa en amarillo de que la carpeta no está vacía. Confirmar hace un "switch only": el marcador se reescribe apuntando a los datos existentes, no se copia nada y el launcher se reinicia. Los datos que queden en la ubicación antigua se vuelven huérfanos; puedes borrarlos a mano.

Si una migración falla a medias, el overlay se pone rojo y muestra el error. El launcher hace rollback: los archivos que creó en la ruta nueva se eliminan y el marcador sigue apuntando a la ruta antigua, así puedes reintentar sin perder datos. Las dos causas habituales son permisos de escritura ausentes en la unidad destino y espacio libre insuficiente.

### El marcador runtime_path_override

Al cambiar el Runtime Path, GDLauncher escribe un pequeño archivo de texto llamado `runtime_path_override` dentro del **App Data Path** (no dentro del Runtime Path). Su contenido es el nuevo Runtime Path en texto plano. En cada arranque la app lee ese archivo para saber dónde viven tus datos.

Si borras el marcador, GDLauncher cae a su Runtime Path por defecto (`<App Data>/data/`). Tus datos no se pierden, siguen donde los moviste, pero el launcher no los ve hasta que vayas a **Settings → Runtime Path** y vuelvas a apuntar a esa carpeta. Como la carpeta ya tiene datos de GDLauncher, el launcher lo trata como un "switch only" y solo actualiza el marcador sin copiar nada.

## Por qué no compartir la base de datos

El archivo `gdl_conf.db` del Runtime Path contiene tokens de cuenta, refresh tokens de Microsoft, estado de la cuenta GDL y metadatos de instancia. Local por máquina y con credenciales sensibles: **no lo compartas con nadie.** Dos equipos con la misma DB se pelearán por refrescar tokens y los dos quedarán desconectados.

Si quieres las mismas instancias en otro PC, copia manualmente las carpetas de instancia desde `instances/`, o usa la función Cloud Instance Share, diseñada exactamente para eso.
