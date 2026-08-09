# Política de privacidad de Aureus

Última actualización: 9 de agosto de 2026.

Aureus procesa localmente la configuración del mod, métricas de rendimiento y preferencias del launcher. No vende datos, la telemetría está ausente/desactivada por defecto y no envía contraseñas a servidores propios.

El inicio de sesión abre el navegador del sistema y utiliza Microsoft OAuth Authorization Code con PKCE. El código vuelve a un puerto temporal en `localhost`. Los tokens se guardan exclusivamente en el almacén seguro del sistema (Keychain de macOS, Credential Manager de Windows o Secret Service de Linux), nunca en archivos de configuración ni registros. El usuario puede cerrar la sesión desde el launcher.

Para validar la propiedad y obtener el perfil público del jugador, el launcher se comunica directamente con los servicios oficiales de Microsoft, Xbox Live, XSTS y Minecraft. Esos servicios se rigen por sus propias políticas de privacidad.

Este proyecto está en desarrollo. No introduzcas credenciales en ninguna ventana que no pertenezca al dominio oficial de Microsoft.

## Datos, conservación y eliminación

Aureus no opera servidores propios para cuentas, analítica o telemetría. La configuración, las instancias, los registros de diagnóstico y las métricas de rendimiento permanecen en el equipo del usuario. Los datos locales se conservan hasta que el usuario cierre sesión, elimine una instancia o desinstale la aplicación y borre sus datos. Las credenciales guardadas pueden eliminarse desde la sección de cuenta del launcher o desde el almacén seguro del sistema operativo.

Los archivos de diagnóstico solo se comparten si el usuario decide copiarlos o enviarlos. Aureus no los carga automáticamente.

Para preguntas de privacidad o solicitudes relacionadas con este proyecto, abre una incidencia en [GitHub Issues](https://github.com/jjfrancoms/aureus-client/issues) sin incluir tokens, contraseñas ni otros datos sensibles.

---

# Aureus Privacy Policy (English)

Last updated: August 9, 2026.

Aureus processes mod settings, performance metrics, launcher preferences and diagnostic logs locally. It does not sell personal data and does not operate proprietary analytics or telemetry services.

Microsoft sign-in uses OAuth Authorization Code with PKCE and a temporary localhost callback. Authentication tokens are stored only in the operating system's secure credential store (macOS Keychain, Windows Credential Manager or Linux Secret Service). They are not written to configuration files or logs. Users can remove stored credentials by signing out.

The launcher communicates directly with official Microsoft, Xbox Live, XSTS and Minecraft services to verify game ownership and obtain the player's public profile. Those services are governed by their respective privacy policies.

Configuration, instances, logs and performance data remain on the user's device until the user deletes them or removes the application's data. Diagnostic files are never uploaded automatically. For privacy questions, open a [GitHub issue](https://github.com/jjfrancoms/aureus-client/issues) without including passwords, tokens or other sensitive information.
