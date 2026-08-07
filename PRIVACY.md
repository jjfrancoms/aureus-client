# Política de privacidad de Aureus

Última actualización: 7 de agosto de 2026.

Aureus procesa localmente la configuración del mod, métricas de rendimiento y preferencias del launcher. No vende datos, la telemetría está ausente/desactivada por defecto y no envía contraseñas a servidores propios.

El inicio de sesión abre el navegador del sistema y utiliza Microsoft OAuth Authorization Code con PKCE. El código vuelve a un puerto temporal en `localhost`. Los tokens se guardan exclusivamente en el almacén seguro del sistema (Keychain de macOS, Credential Manager de Windows o Secret Service de Linux), nunca en archivos de configuración ni registros. El usuario puede cerrar la sesión desde el launcher.

Para validar la propiedad y obtener el perfil público del jugador, el launcher se comunica directamente con los servicios oficiales de Microsoft, Xbox Live, XSTS y Minecraft. Esos servicios se rigen por sus propias políticas de privacidad.

Este proyecto está en desarrollo. No introduzcas credenciales en ninguna ventana que no pertenezca al dominio oficial de Microsoft.
