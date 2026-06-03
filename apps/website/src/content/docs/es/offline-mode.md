---
title: "Modo sin conexión"
description: "Qué puedes y no puedes hacer en GDLauncher sin conexión a internet. Qué se cachea, qué necesita salir a la red, y cómo se comporta realmente el vencimiento de tokens."
faq:
  - question: "¿Puedo jugar a Minecraft sin conexión a través de GDLauncher?"
    answer: "Sí. El singleplayer funciona totalmente sin conexión. Si tu token cacheado sigue vigente, pulsas Play y Minecraft arranca normal. Si ha expirado, el launcher muestra un prompt 'Account Expired' con un botón 'Launch anyway'; elige eso y aún puedes jugar singleplayer. La auth en línea solo es necesaria para servidores multijugador que verifican identidad vía Mojang."
  - question: "¿Cuánto tiempo puedo estar sin conexión antes de que expiren los tokens?"
    answer: "Depende de qué quieras hacer. Para singleplayer no hay límite real de tiempo: el launcher ofrecerá 'Launch anyway' una vez el token haya expirado. Para servidores multijugador que verifican identidad vía Mojang necesitas un token fresco, lo cual significa volver a estar online para refrescar. El launcher refresca el token de auth de Minecraft de forma proactiva unas 12 horas antes de su expiración de 24 horas, así que si has estado online recientemente, el multijugador sigue funcionando."
  - question: "¿Puedo instalar mods o modpacks nuevos sin conexión?"
    answer: "No. Las descargas de mods vienen de los CDN de CurseForge y Modrinth, ambos necesitan internet. Igual para descargas de Java, assets de Minecraft y manifests de modpack. Todo lo relacionado con instalación necesita conexión."
  - question: "¿Puedo actualizar una instancia existente sin conexión?"
    answer: "No. Misma razón: las actualizaciones traen archivos nuevos desde los CDN. El launcher pone la actualización en cola y reintenta cuando ve conexión."
  - question: "¿Y la cuenta GDL? ¿funciona sin conexión?"
    answer: "Parcialmente. El launcher recuerda que estás conectado en GDL, pero cualquier cosa que requiera hablar con el servicio GDL (compartir instancia, edición de perfil, ver tus compartidos) necesita internet. La cuenta Microsoft es la que controla el lanzamiento; GDL es para funcionalidades más allá del lanzamiento."
---

# Modo sin conexión

## Qué significa realmente "sin conexión" aquí

El comportamiento offline de GDLauncher depende de tres necesidades de red distintas:

1. **Auth Microsoft** (probarle a Mojang que posees Minecraft).
2. **Descargas de mods y assets** (CurseForge, Modrinth, CDN de bibliotecas de Mojang).
3. **Funciones de cuenta GDL** (compartir instancia, perfil, historial de nombre, etc.).

Cada una falla diferente cuando la red está caída, y el launcher se comporta acorde en cada caso.

## Lanzar una instancia ya instalada sin conexión

El escenario más común: estás en un avión, en una cabaña, o tu internet de casa está caído y quieres jugar algo que ya tienes instalado.

**Suele funcionar**, porque GDLauncher cachea los datos necesarios para lanzar:

- Los tokens de auth de Mojang se guardan localmente con timestamps de expiración.
- Las bibliotecas y assets de Minecraft ya están en disco (en el runtime path).
- Las instancias moddeadas tienen sus mods instalados localmente.

Cuando pulsas Play sin conexión, el launcher:

1. Comprueba si el token de auth de Minecraft de la cuenta Microsoft activa sigue vigente (no expirado).
2. Si sí, lanza Minecraft directamente con ese token. Minecraft en sí no necesita internet para arrancar un mundo singleplayer.
3. Si el token de acceso ha expirado pero el refresh token sigue vigente, el launcher intenta llamar al endpoint de refresh de Microsoft, que necesita internet. Offline esa llamada falla, y el estado de la cuenta cambia a "expired" en Settings → Accounts.
4. Si la cuenta está expirada y aún así pulsas Play, el launcher abre un modal Account Expired con dos botones: **Launch anyway** (usa el token cacheado, vale para singleplayer) y **Back to login** (te envía por el flujo de sign-in de Microsoft, requiere internet).

Así que para singleplayer, 'Launch anyway' funciona sin importar cuándo fue la última vez que estuviste online: el token no es verificado por nada una vez Minecraft ha arrancado. Para servidores multijugador que verifican identidad, necesitas un token no expirado, así que tienes que haber estado online lo suficientemente reciente para refrescar.

### Por qué expiran los tokens

Esto lo definen los servidores de auth de Microsoft y Mojang, no GDLauncher. La cadena de auth produce dos tokens que importan al launcher:

- Un **token de acceso OAuth Microsoft** (~1 hora). Es el que el launcher usa para hablar con las APIs de auth de Microsoft / Xbox / Mojang. Es corto, pero el launcher lo renueva con un refresh token cuando está online; rara vez te das cuenta.
- Un **token de auth Minecraft** (~24 horas). Es el que se le pasa a Minecraft al lanzar, así que es el que controla el juego sin conexión. GDLauncher lo refresca de forma proactiva unas 12 horas antes de la expiración mientras estás online.

El refresh token de Microsoft dura meses, pero puede invalidarse del lado del servidor, por ejemplo cuando cambias tu contraseña de Microsoft, activas una nueva función de seguridad o cierras sesión desde la web de Microsoft. Si tu refresh token se invalida mientras estás offline, no hay nada que el launcher pueda hacer hasta que vuelvas a estar online para reautenticar.

## Unirse a servidores multijugador sin conexión

**No funciona**, porque los servidores multijugador verifican tu identidad contra el session server de Mojang, lo que requiere internet en ambos extremos. El multijugador LAN puede funcionar entre máquinas en la misma LAN offline si ambas han autenticado online recientemente.

## Instalar nuevas instancias, mods o modpacks sin conexión

**No funciona.** Cada flujo de instalación descarga de un CDN:

- Los modpacks bajan su manifest y luego los archivos de mods individuales.
- Añadir un mod desde la pestaña Addons descarga su JAR.
- Crear una instancia custom para una versión de Minecraft que no tienes baja el manifest JSON de esa versión, el JAR de la versión, los assets, el instalador del mod loader.

Todo eso fallará sin conexión con timeouts o errores de DNS. El launcher no reintenta indefinidamente, verás un fallo en el modal de creación de instancia o en el panel Tasks.

Si sabes que vas a un sitio sin conexión, pre-instala las instancias que vas a querer antes de irte.

## Funciones de cuenta GDL sin conexión

**Mayoritariamente no funciona**, porque las funciones de cuenta GDL son por definición "hablar con el backend de GDL". En concreto:

- Cloud Instance Sharing (generar un código): falla, servicio GDL no alcanzable.
- Importar una instancia compartida: falla por la misma razón.
- Editar tu perfil GDL: falla.
- Ver tus compartidos: muestra estado cacheado, no puede refrescar.

El launcher recuerda que estás conectado en GDL mientras estás offline, pero la UI muestra datos desactualizados y rechaza acciones que requerirían una llamada de red.

## TL;DR

- Instancia ya instalada, token fresco: lanzar offline funciona.
- Instancia ya instalada, token expirado: el launcher pregunta, elige 'Launch anyway' para singleplayer.
- Multijugador con token expirado: bloqueado hasta poder llegar a Microsoft para refrescar.
- Cualquier cosa que descargue: bloqueado.
- Cualquier cosa que hable con el backend GDL: bloqueado.
- Mundos singleplayer: 100% capaces offline una vez la instancia está en disco.
