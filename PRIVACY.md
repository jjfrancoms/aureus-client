# Política de privacidad de Aureus

Última actualización: 7 de agosto de 2026.

Aureus procesa localmente la configuración del mod, métricas de rendimiento y preferencias del launcher. No vende datos, no contiene telemetría y no envía contraseñas a servidores propios.

El inicio de sesión abre el navegador del sistema y utiliza Microsoft OAuth Authorization Code con PKCE. El código vuelve a un puerto temporal en `localhost`. Los tokens se mantienen únicamente en memoria durante la ejecución actual y no se escriben en registros ni archivos en esta versión.

Para validar la propiedad y obtener el perfil público del jugador, el launcher se comunica directamente con los servicios oficiales de Microsoft, Xbox Live, XSTS y Minecraft. Esos servicios se rigen por sus propias políticas de privacidad.

Este proyecto está en desarrollo. No introduzcas credenciales en ninguna ventana que no pertenezca al dominio oficial de Microsoft.
