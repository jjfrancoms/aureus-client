# Aureus Client

Aureus es un mod cliente de código abierto para Minecraft Java 1.21.11 y un launcher de escritorio en desarrollo. Su objetivo es ofrecer rendimiento medible y utilidades PvP de juego limpio sin automatizar el combate ni evadir sistemas anticheat.

## Funciones actuales

- HUD de FPS, frame time, promedio y 1% low.
- CPS, teclas, ping, coordenadas y memoria.
- Durabilidad de armadura, efectos y cooldown de ataque.
- Perfiles MAX_FPS, BALANCED y QUALITY.
- Presupuesto configurable de partículas mediante Mixin.
- Detección de Sodium, Lithium, FerriteCore e ImmediatelyFast.
- Launcher Tauri que detecta Java/Minecraft e instala el mod.
- Inicio de sesión Microsoft con Authorization Code + PKCE y callback local.

## Estado de autenticación

El Client ID de Aureus está pendiente de revisión para la allowlist de Minecraft Java Game Services. Hasta que Mojang lo apruebe, Minecraft Services rechazará el último paso del inicio de sesión. No se incluyen secretos en este repositorio.

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

## Descargas y firma de código

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
