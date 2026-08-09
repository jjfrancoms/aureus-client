# Aureus Client

Aureus es un mod cliente de código abierto para Minecraft Java 1.21.11 y un launcher de escritorio en desarrollo. Su objetivo es ofrecer rendimiento medible y utilidades PvP de juego limpio sin automatizar el combate ni evadir sistemas anticheat.

## Funciones actuales

- HUD individual para FPS, frame time, 1% low, CPS, teclas, ping, coordenadas, memoria, armadura, efectos, cooldown y contadores PvP.
- Editor HUD arrastrable con escala, opacidad, búsqueda, perfiles por servidor y modo captura.
- Perfiles competitivo, máximo FPS, equilibrado, calidad, RAM mínima y batería.
- Benchmark A/B, telemetría local y recomendaciones por RAM, CPU y GPU.
- Gestor general de versiones, Fabric, dependencias y mods compatibles desde Modrinth.
- Descargas reanudables y verificadas, manifiestos, backups, rollback, reparación y modo seguro.
- Instancias aisladas, perfiles portátiles y preservación de mods instalados manualmente.
- Launcher Tauri compartido para Windows y macOS con actualizaciones firmadas.
- Inicio de sesión Microsoft con Authorization Code + PKCE y callback local.

## Estado de autenticación

El launcher usa inicio de sesión oficial de Microsoft mediante Authorization Code + PKCE. No incluye secretos y requiere que la cuenta conectada tenga una licencia válida de Minecraft Java.

## Juego limpio

Aureus no incluye autoclicker, aim assist, reach, velocity, triggerbot, automatización de inventario, modificación maliciosa de paquetes ni evasión de anticheat. Cada usuario debe respetar las reglas del servidor en el que juegue.

## Compilar el mod

Requiere Java 21:

```bash
./gradlew build
```

## Compilar el launcher

Requiere Node.js, Rust y las dependencias de Tauri 2:

```bash
cd launcher
npm install
npm run build
cd src-tauri
cargo check
```

Para generar un instalador local de prueba sin una clave privada de actualización:

```bash
cd launcher
npm run tauri:build:local
```

Este paquete local no representa una versión oficial firmada. Las publicaciones
oficiales conservan `createUpdaterArtifacts` activo y requieren los secretos
`TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` en GitHub.

## Descargas y firma de código

Consulta la [Code signing policy](CODE_SIGNING_POLICY.md), que define alcance,
procedencia de compilación y responsables de revisión y aprobación.

Las versiones oficiales se publican exclusivamente en [GitHub Releases](https://github.com/jjfrancoms/aureus-client/releases). Aureus ha solicitado participar en el programa gratuito de [SignPath Foundation](https://signpath.org/) para firmar los instaladores de Windows de este proyecto de código abierto. Las firmas se solicitarán desde GitHub Actions una vez que SignPath apruebe el proyecto; ningún mantenedor tendrá acceso directo a la clave privada de firma.

Cada versión se construye desde una etiqueta pública `v*` mediante [`.github/workflows/package.yml`](.github/workflows/package.yml). El flujo compila el mod, instala dependencias desde los archivos de bloqueo, prepara los recursos integrados y genera por separado los paquetes de Windows y macOS. La publicación solo se hace visible cuando ambos trabajos terminan correctamente.

### Verificación y reproducibilidad

- Código fuente y etiqueta de cada versión: públicos en este repositorio.
- Dependencias de Node y Rust: fijadas mediante `package-lock.json` y `Cargo.lock`.
- Historial de compilaciones: [GitHub Actions](https://github.com/jjfrancoms/aureus-client/actions).
- Artefactos oficiales: [GitHub Releases](https://github.com/jjfrancoms/aureus-client/releases).
- Política de privacidad: [PRIVACY.md](PRIVACY.md).

## Privacidad

Consulta [PRIVACY.md](PRIVACY.md). Aureus está en desarrollo y no es un producto oficial de Minecraft, Mojang ni Microsoft.

## Licencia

MIT. Las marcas Minecraft, Mojang, Microsoft y Xbox pertenecen a sus respectivos propietarios.
