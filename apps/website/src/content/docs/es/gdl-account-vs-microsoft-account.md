---
title: "Cuenta GDL vs cuenta Microsoft"
description: "GDLauncher usa dos tipos de cuenta distintos. Microsoft para jugar a Minecraft, GDL para compartir y funciones sociales. Qué hace cada una y cuáles necesitas."
faq:
  - question: "¿Necesito una cuenta GDL para jugar a Minecraft?"
    answer: "No. Para jugar solo necesitas una cuenta Microsoft (la que usaste para comprar Minecraft Java Edition). La cuenta GDL es opcional y solo desbloquea funciones propias de GDLauncher: compartir instancias, friend codes, historial de nombre, edición de perfil. Puedes usar GDLauncher perfectamente sin ella."
  - question: "¿Qué desbloquea una cuenta GDL?"
    answer: "Hoy, principalmente compartir instancias: generas un código con clic derecho → Share y otro usuario de GDLauncher lo pega para importar la instancia. También consigues un nombre estable con historial de renombres y una tarjeta de perfil con friend code que te identifica en las previsualizaciones de compartido. Todo lo que implica a otros usuarios de GDLauncher pasa por la cuenta GDL."
  - question: "¿Puedo usar GDLauncher sin cuenta Microsoft?"
    answer: "No. La cuenta Microsoft demuestra que posees Minecraft y obtiene el token de lanzamiento de Mojang. Sin ella, el launcher no tiene con qué autenticarse ante los servidores de Minecraft."
  - question: "¿Puedo tener varias cuentas Microsoft en GDLauncher?"
    answer: "Sí. Settings → Accounts muestra todas las cuentas Microsoft conectadas en una tabla. Puedes añadir, eliminar y elegir cuál está Activa (la que usa Play). La cuenta activa aparece resaltada en la columna izquierda."
  - question: "¿Qué es el friend code de mi perfil GDL?"
    answer: "Un identificador corto y estable de tu cuenta GDL. No cambia aunque renombres tu nombre visible, y aparece en las previsualizaciones de compartido para que otros sepan quién comparte. Copiable desde Settings → Accounts → tarjeta de perfil GDL."
---

# Cuenta GDL vs cuenta Microsoft

## Dos sistemas de cuentas, un launcher

GDLauncher tiene dos sistemas de cuenta. **Microsoft** demuestra que posees Minecraft y es obligatoria para jugar. **GDL** es la cuenta opcional propia de GDLauncher, para funciones que tocan el backend GDL (compartir instancias, perfil, historial de nombre).

### Cuenta Microsoft

La cuenta con la que compraste Minecraft Java Edition, la que tiene la licencia. Microsoft la exige para lanzar Minecraft. GDLauncher inicia sesión en Microsoft, guarda los tokens y al lanzar pasa el correcto a Mojang para que los servidores sepan que posees el juego.

Necesitas al menos una cuenta Microsoft conectada para jugar. Sin ella, Play no hace nada.

Almacenado localmente por cuenta: access token, refresh token, ID token, el nombre Minecraft y UUID, una referencia de skin, y la expiración del access token. El launcher refresca el access token en segundo plano usando el refresh token; normalmente ni te enteras.

Qué desbloquea: lanzar Minecraft, unirse a servidores, tener el juego.

### Cuenta GDL

El sistema de cuentas propio de GDLauncher. Opcional. Existe solo para activar las funciones que GDLauncher proporciona, las cosas que no atañen a Microsoft.

Te registras con un email y un nombre visible, y recibes un friend code estable. Desde ahí puedes usar las funciones que implican a otros usuarios de GDLauncher.

Localmente solo se guarda el vínculo: a qué cuenta Microsoft pertenece esta identidad GDL, y un JWT para hablar con el backend GDL. Nombre, friend code, email, foto de perfil, etc., viven en el backend GDL y la UI los pide cuando los necesita.

Qué desbloquea:

- **Cloud Instance Sharing.** Clic derecho en instancia → Share genera un código que otros usuarios de GDLauncher pegan para importar.
- **Historial de nombre.** Renombrarte registra el historial de cambios; puedes ver nombres pasados desde la tarjeta de perfil y borrarlos si quieres.
- **Edición de perfil.** Nombre visible, foto de perfil, ajustes de email de recuperación, todo desde la tarjeta de perfil GDL en Settings → Accounts.

## Cuándo necesitas cada una

| Escenario | Microsoft | GDL |
|---|---|---|
| Solo lanzar Minecraft | Requerida | No hace falta |
| Instalar mods y modpacks de CurseForge/Modrinth | Requerida | No hace falta |
| Compartir una instancia con un amigo | Requerida | Requerida |
| Recibir un código de instancia | Requerida | Requerida |
| Usar el sistema de amigos | Requerida | Requerida |
| Jugar offline (instancia ya instalada) | Auth en caché vale un rato | No hace falta |

## Cómo gestionarlas

Las dos viven en **Settings → Accounts**.

La sección GDL Account está arriba. Desconectado: un botón Sign in / Sign up. Conectado: tarjeta de perfil con nombre, friend code (copiable), email de recuperación, estado de verificación. Una "Danger Zone" abajo permite programar la eliminación con un cooldown de 7 días.

La sección Microsoft Accounts viene debajo en forma de tabla. Columnas: Active, Username, Type, Status, UUID, Actions. Status indica el estado del token por cuenta:

- **ok** (check verde): token válido, la cuenta puede lanzar.
- **expired** (alerta amarilla): token expirado. La columna Actions muestra un icono de refresh, al hacer clic vuelves al flujo de inicio de sesión Microsoft.
- **refreshing** (refresh amarillo): el launcher está refrescando el token en segundo plano. Nada que hacer.
- **invalid** (X roja): el token no se pudo refrescar. Mismo icono refresh que expired, al hacer clic te lleva por el flujo de inicio de sesión Microsoft.

Para cambiar de cuenta activa, pulsa la celda Active de la fila que quieras. La fila activa muestra un icono de doble check; otras filas lo enseñan tenuemente al pasar el ratón.

## Eliminar cuentas

Eliminar la única cuenta Microsoft te desconecta de GDLauncher por completo y te lleva a la página de inicio.

Eliminar una cuenta Microsoft vinculada a tu cuenta GDL abre un modal de confirmación, preguntando si quieres realmente romper el vínculo antes de eliminar.

Eliminar tu cuenta GDL es una acción demorada de 7 días. Durante el cooldown puedes cancelarla desde la misma página.
