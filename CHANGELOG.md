# Changelog

## 0.3.7

- Rediseña el menú dentro de Minecraft como un panel de módulos visuales con estado verde/gris.
- Oculta por defecto la línea técnica con los nombres de los mods instalados, incluso al migrar configuraciones antiguas.
- Añade módulos propios de dirección y bioma al HUD configurable.
- Incorpora para Fabric 1.21.11 HUD editable, minimapa, tooltips de shulkers, barras de efectos, inventario organizado, Mouse Tweaks y HUD de montura.
- Resuelve automáticamente esos complementos y sus dependencias en otras versiones cuando exista una compilación compatible.

## 0.3.6

- Simplifica Inicio a una única acción PLAY que prepara Minecraft automáticamente.
- Bloquea el inicio sin una cuenta Microsoft válida y dirige a Cuenta mediante una notificación.
- Elimina de Inicio la tarjeta con PID y cronómetro y evita reconstruir procesos ajenos al abrir Aureus.

## 0.3.5

- Publica una actualización únicamente cuando macOS y Windows estén completos.
- Sustituye el error técnico de plataforma por un estado comprensible de preparación.

## 0.3.4

- Corrige la falsa detección de Minecraft ejecutándose en Windows.
- Limpia automáticamente el panel después de cancelar un inicio.
- Añade estado y progreso de actualizaciones en la esquina superior derecha.

## 0.3.3

- Evita que PowerShell aparezca al iniciar Aureus en Windows.
- Ejecuta la detección de procesos y memoria en segundo plano.

## 0.3.2

- Añade Debugify y FPSFlow al stack estable de Fabric 1.21.11.
- Añade Bura y Rhenium como experimentales opcionales con respaldo previo.
- Incorpora perfiles Competitivo PvP y Batería.
- Añade comparación A/B de FPS, 1% low y memoria.
- Sincroniza automáticamente distancia, simulación y límite de FPS con el mod.

## 0.3.1

- Preparar Minecraft ya no inicia el juego.
- Añade confirmación previa con versión, instancia, memoria, perfil y contenido.
- Permite abrir Rendimiento para configurar antes de iniciar o cancelar el arranque.

## Unreleased

- Added persistent launcher instances with validated memory and JVM settings.
- Added local diagnostics for Java, Fabric, Fabric API, Aureus and Minecraft logs.
- Added automatic preparation of Fabric Loader, Fabric API and Aureus.
- Added simplified navigation, customizable animated cover and collapsible sidebar.
- Added in-game optimization profiles, adaptive particles and dockable settings menu.
- Added macOS and Windows quality checks in GitHub Actions.
