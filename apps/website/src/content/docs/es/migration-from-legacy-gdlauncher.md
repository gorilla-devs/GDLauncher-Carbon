---
title: "Migración desde el GDLauncher antiguo"
---

# Migración desde el GDLauncher antiguo

Si todavía usas GDLauncher Legacy, migrar a GDLauncher Carbon es sencillo. Aquí tienes cómo hacerlo.

## Descarga GDLauncher Carbon

Puedes descargar GDLauncher Carbon desde nuestro [sitio oficial](https://gdlauncher.com).

## Instala / sustituye GDLauncher Carbon

Una vez descargada la nueva versión, instálala siguiendo las instrucciones para tu sistema operativo [aquí](/docs/installation).
Instalar GDLauncher Carbon reemplazará la versión antigua, ya que ambas comparten el mismo identificador de aplicación. No perderás ninguna de tus instancias antiguas: se guardan en una carpeta diferente.

## Importar automáticamente tus instancias antiguas

Al iniciar GDLauncher Carbon e iniciar sesión, te recibirá una pantalla de bienvenida que te preguntará si quieres importar tus instancias antiguas.

## Importar manualmente tus instancias antiguas

Si por algún motivo la importación automática no funciona, primero infórmanos del problema [aquí](https://discord.gdlauncher.com).
Puedes importar tus instancias antiguas manualmente del siguiente modo:

- Crea una nueva instancia en GDLauncher Carbon con la misma configuración. Si es un modpack, instala exactamente la misma versión del modpack. Si es vanilla, instala la misma versión de Minecraft y el mismo mod loader.
- Abre la ruta de datos del GDLauncher antiguo.
- Abre la nueva instancia Carbon con clic derecho → **More Options** → **Open Folder** y entra en la subcarpeta `instance/`. Copia allí el contenido del antiguo directorio del juego: los mundos deben quedar en `instance/saves/` y los mods en `instance/mods/`. Consulta los datos de las rutas en [solución de problemas](/docs/troubleshooting).

## Eliminar las instancias antiguas (opcional)

Cuando hayas terminado de importar, puedes eliminar opcionalmente tus instancias antiguas. Esto no se hará de forma automática.

Antes de eliminar el origen, inicia la nueva instancia Carbon y comprueba que tus mundos y mods están presentes y funcionan correctamente.
