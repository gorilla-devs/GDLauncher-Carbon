---
title: "Errores de autenticación de Microsoft"
description: "Resuelve los errores comunes de autenticación de Microsoft en GDLauncher. Soluciones para Invalid Grant, cuenta baneada, acceso de consola requerido y errores de Xbox Live."
faq:
  - question: "¿Por qué me aparece un error 'Invalid Grant' en GDLauncher?"
    answer: "Un error 'Invalid Grant' suele significar que hay un problema con la seguridad de tu cuenta de Microsoft. Las soluciones más comunes son activar la verificación en dos pasos en tu cuenta de Microsoft, establecer una contraseña si no tenías una, o cerrar sesión y volver a iniciarla."
  - question: "¿Por qué GDLauncher dice que mi cuenta está baneada?"
    answer: "Si GDLauncher reporta tu cuenta como baneada, el baneo proviene de Mojang o Microsoft, no de GDLauncher. Inicia sesión en minecraft.net o en tu cuenta de Microsoft para ver el motivo. GDLauncher solo retransmite la respuesta de autenticación: no existe una lista de baneos del lado de GDLauncher."
  - question: "¿Por qué GDLauncher dice que necesito acceso de consola?"
    answer: "Esto suele aparecer en cuentas infantiles o sometidas a restricciones de grupo familiar. La cuenta del padre/madre debe otorgar permiso a la cuenta del menor para jugar a Minecraft en la plataforma que se va a usar. Ajusta los controles familiares en account.microsoft.com/family."
  - question: "Sigo recibiendo errores de autenticación de Xbox Live. ¿Qué hago?"
    answer: "Los errores de Xbox Live suelen significar que la configuración de país/región de la cuenta de Microsoft no permite Xbox Live, o que la cuenta no ha aceptado los términos de Xbox Live. Inicia sesión una vez en xbox.com con la misma cuenta para aceptar los términos y vuelve a intentarlo en GDLauncher."
  - question: "¿Tengo que volver a comprar Minecraft para usar GDLauncher?"
    answer: "No. GDLauncher utiliza tu cuenta existente de Minecraft con Microsoft / Mojang. No hay compra ni suscripción aparte. Si ya tienes Minecraft Java Edition, puedes iniciar sesión en GDLauncher con la misma cuenta."
---

# Errores de autenticación de Microsoft

Cuando inicias sesión en GDLauncher con una cuenta Microsoft, el launcher habla por ti con el servicio OAuth de Microsoft y con la API de autenticación de Mojang. Los errores devueltos por esos servicios se muestran tal cual en el launcher; el texto viene de Microsoft, no de GDLauncher.

Aquí los más comunes y qué significan.

## Invalid Grant

Aparece cuando Microsoft rechaza el intercambio OAuth. Las causas más comunes:

- La cuenta no tiene contraseña establecida (es una cuenta Microsoft creada con un enlace de email o login social). Añade una contraseña en [account.microsoft.com](https://account.microsoft.com).
- La cuenta usa un flujo de inicio de sesión antiguo sin autenticación en dos pasos. Activar 2FA en [account.microsoft.com/security](https://account.microsoft.com/security) lo soluciona en la mayoría de los casos.
- Los tokens cacheados están caducados. Cierra sesión de la cuenta en **Settings → Accounts** e inicia sesión de nuevo.

## Cuenta baneada

GDLauncher retransmite la respuesta de Mojang sin alterar. El baneo está del lado de Mojang; GDLauncher no mantiene su propia lista de baneos. Inicia sesión en [minecraft.net](https://minecraft.net) con la misma cuenta para ver el motivo y opciones de apelación.

## Se requiere acceso de consola

Suele aparecer en cuentas infantiles dentro de un grupo familiar de Microsoft. La cuenta parental debe autorizar Minecraft Java Edition para la cuenta del menor en [account.microsoft.com/family](https://account.microsoft.com/family). Tras dar permiso, cierra sesión y vuelve a entrar en GDLauncher.

## Errores de Xbox Live

Los fallos de Xbox Live caen en una de dos categorías:

- La configuración de país/región de la cuenta Microsoft no permite Xbox Live. Ajústala en [account.microsoft.com/profile](https://account.microsoft.com/profile).
- La cuenta no ha aceptado los términos de Xbox Live. Entra una vez en [xbox.com](https://xbox.com) con la misma cuenta Microsoft para aceptarlos y vuelve a intentar en GDLauncher.

## Cuenta expirada

El refresh token de Microsoft ha expirado o ha sido revocado (lo más habitual: has cambiado la contraseña de la cuenta en otro sitio). GDLauncher muestra un prompt "Account expired" y ofrece reautenticar. Inicia sesión de nuevo desde **Settings → Accounts**.

## Si nada de lo anterior funciona

Si el mensaje de error no coincide con ninguno de los anteriores, comparte los dos logs a nivel de app en nuestro [Discord](https://discord.gdlauncher.com): `main.log` (Electron) y el más reciente `__gdl_logs__/<timestamp>.log` (Rust core). Las rutas exactas están en [Share App Logs](/guides/share-app-logs). Casi siempre necesitamos ambos, el flujo de autenticación cruza entre los dos procesos.
