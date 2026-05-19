---
title: "Solución de problemas"
description: "Soluciona problemas comunes al iniciar GDLauncher y Minecraft. Ruta de datos, ruta del runtime, ubicación de los registros y soluciones probadas."
faq:
  - question: "¿Dónde guarda GDLauncher sus datos?"
    answer: "En Windows: C:\\Users\\<tú>\\AppData\\Roaming\\gdlauncher_carbon. En macOS: /Users/<tú>/Library/Application Support/gdlauncher_carbon. En Linux: $XDG_DATA_HOME/gdlauncher_carbon (o ~/.local/share/gdlauncher_carbon si XDG no está definido)."
  - question: "¿Dónde están los registros de GDLauncher?"
    answer: "GDLauncher escribe dos registros a nivel de app en archivos distintos: main.log (Electron) en la carpeta App Data, y archivos <timestamp>.log con marca de tiempo en la carpeta __gdl_logs__ de la ruta de runtime (Rust core; se conservan los 10 más recientes). Al reportar problemas, envía los dos. Las rutas exactas están en la guía Share App Logs."
  - question: "GDLauncher no se abre. ¿Qué hago?"
    answer: "Primero revisa los registros en la carpeta de datos para localizar el error. Causas habituales: runtime corrupto, antivirus que bloquea el ejecutable o una actualización aplicada parcialmente. Una reinstalación limpia de GDLauncher y la restauración de las instancias suele resolver ambos casos."
  - question: "¿Por qué se cuelga mi modpack al iniciar?"
    answer: "La mayoría de los cuelgues al iniciar se deben a una incompatibilidad entre la versión de Minecraft, el mod loader y los mods. Revisa el archivo más reciente dentro de __gdl_logs__ para ver el error. Si se nombra un mod concreto, suele ser el culpable: desactívalo en la pestaña Addons y vuelve a iniciar. Si es un OutOfMemoryError, aumenta la RAM en los ajustes de la instancia."
  - question: "¿Cómo muevo GDLauncher a otra unidad o carpeta?"
    answer: "Abre Ajustes → General → Ruta del runtime. Cámbiala a la nueva ubicación y GDLauncher migrará automáticamente tus instancias y descargas. La migración se ejecuta una sola vez en el siguiente inicio."
  - question: "¿Puedo usar GDLauncher sin conexión?"
    answer: "Puedes jugar sin conexión a las instancias que ya tengas instaladas. La autenticación requiere conectarse al menos una vez (cuenta de Microsoft), y descargar nuevos mods o modpacks necesita conexión a internet."
---

## Ruta de datos de la aplicación

Es la ruta donde GDLauncher guarda los datos de Electron, así como la ruta del runtime del Core Module por defecto.

### Windows

`C:\Users\\{{Tu nombre de usuario}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{Tu nombre de usuario}}/Library/Application Support/gdlauncher_carbon`

### Linux

- si la variable de entorno `$XDG_DATA_HOME` está definida: `$XDG_DATA_HOME/gdlauncher_carbon`
- en caso contrario: `{{homedir}}/.local/share/gdlauncher_carbon`

[Más detalles sobre homedir](https://nodejs.org/api/os.html#oshomedir)

## Ruta del runtime del Core Module

Es la ruta donde el Core Module guarda todos sus datos, incluidas todas las instancias, los assets y las librerías.
Suele encontrarse en la misma ruta que la ruta de datos, dentro de la carpeta `data`, salvo que indiques otra ubicación.

### Base de datos de la aplicación

La base de datos se encuentra en la ruta del runtime del Core Module y es un archivo SQLite llamado `gdl_conf.db`.

**NO ENVÍES ESTE ARCHIVO A NADIE, CONTIENE DATOS SENSIBLES.**

### Registros de la aplicación

GDLauncher escribe dos registros a nivel de app en archivos distintos. Para soporte, **siempre envía los dos**, las dos mitades del launcher se pasan trabajo entre ellas, y la causa de un fallo en un lado suele aparecer en el log del otro lado.

- **`main.log`** en el App Data Path: el log del proceso principal Electron. Cubre creación de ventana, IPC, auto-update, diálogos nativos y crashes duros del shell del escritorio.
- **`__gdl_logs__/<timestamp>.log`** en el Core Module Runtime Path: el log del Rust core. Cubre inicio de sesión, descargas de assets, instalación de mod loaders, lanzamientos de instancia, cambios de settings. Se conservan los 10 más recientes.

Rutas por SO y capturas en la guía [Share App Logs](/guides/share-app-logs).

**LOS REGISTROS PUEDEN CONTENER DATOS SENSIBLES; TEN CUIDADO AL COMPARTIRLOS.**

### Cambiar la ruta del runtime

Si cambias la ruta del runtime, la app moverá automáticamente todas tus instancias y archivos de configuración a la nueva ubicación.

Si la carpeta de destino ya está en uso, la app simplemente actualizará la configuración de la ruta del runtime y no se moverán ni copiarán archivos.

#### Error de migración

Si la migración falla, la app mostrará un mensaje de error.

Lo primero es intentar entender qué dice el mensaje.
Si todos los archivos se copiaron correctamente, probablemente el error ocurrió al borrar los antiguos. Puedes cerrar la app y eliminarlos manualmente.

NO ELIMINES el archivo llamado `runtime_path_override` en la antigua ruta del runtime: la app lo usa para detectar que la ruta ha cambiado.

Si tienes dudas, únete a nuestro [servidor de Discord](https://discord.gdlauncher.com) y pide ayuda.
