package dev.aureus.client.ui;

import dev.aureus.client.config.ClientConfig;
import dev.aureus.client.optimization.OptimizationProfile;
import dev.aureus.client.optimization.VanillaOptimizer;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

import java.util.function.BooleanSupplier;
import java.util.function.IntConsumer;
import java.util.function.IntSupplier;
import java.util.ArrayList;
import java.util.List;

public final class ConfigScreen extends Screen {
    private static final int ACCENT = 0xFFF4F4F2;
    private static final int TEXT = 0xFFF2F4EF;
    private static final int MUTED = 0xFF92988F;
    private enum Page { HOME, HUD, PERFORMANCE, APPEARANCE, SETTINGS }
    private enum ModuleFilter { ALL, HUD, PVP, INFO }

    private final Screen parent;
    private Page page = Page.HOME;
    private int nextY;
    private int contentLeft;
    private int contentWidth;
    private ModuleFilter moduleFilter = ModuleFilter.ALL;
    private int moduleIndex;
    private final List<ModuleCard> moduleCards = new ArrayList<>();
    private String moduleSearch = "";

    public ConfigScreen(Screen parent) {
        super(Component.literal("Aureus Client"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        moduleCards.clear();
        int sidebar = sidebarWidth();
        contentLeft = sidebar + 20;
        contentWidth = Math.max(308, width - contentLeft - 20);
        buildSidebar(sidebar);
        nextY = 70;
        switch (page) {
            case HOME -> buildHomePage();
            case HUD -> buildHudPage();
            case PERFORMANCE -> buildPerformancePage();
            case APPEARANCE -> buildAppearancePage();
            case SETTINGS -> buildSettingsPage();
        }
        addRenderableWidget(Button.builder(Component.literal("Guardar y volver al juego"), button -> onClose())
                .bounds(contentLeft, height - 29, Math.min(210, contentWidth), 20).build());
    }

    private void buildSidebar(int sidebar) {
        int buttonWidth = sidebar - 12;
        addRenderableWidget(Button.builder(Component.literal(config().menuCollapsed ? ">" : "<"), button -> {
            config().menuCollapsed = !config().menuCollapsed;
            ClientConfig.save();
            rebuildWidgets();
        }).bounds(sidebar - 29, 10, 23, 20).build());
        int y = 52;
        addNav(Page.HOME, "Inicio", "I", y, buttonWidth); y += 25;
        addNav(Page.HUD, "HUD y PvP", "H", y, buttonWidth); y += 25;
        addNav(Page.PERFORMANCE, "Rendimiento", "R", y, buttonWidth); y += 25;
        addNav(Page.APPEARANCE, "Apariencia", "A", y, buttonWidth); y += 25;
        addNav(Page.SETTINGS, "Ajustes", "C", y, buttonWidth);
    }

    private void addNav(Page target, String label, String icon, int y, int buttonWidth) {
        String text = config().menuCollapsed ? icon : icon + "   " + label;
        addRenderableWidget(Button.builder(Component.literal(text), button -> openPage(target))
                .bounds(6, y, buttonWidth, 20).build());
    }

    private void buildHomePage() {
        addRenderableWidget(Button.builder(profileLabel(), button -> {
            OptimizationProfile.selected(config()).next().apply(config());
            applyLive();
            rebuildWidgets();
        }).bounds(contentLeft, nextY, Math.min(308, contentWidth), 20).build());
        nextRow();
        addToggle("Mostrar FPS", () -> config().showFps, value -> config().showFps = value, 0);
        addToggle("Optimización activa", () -> config().applyVanillaOptimizations,
                value -> config().applyVanillaOptimizations = value, 1);
    }

    private void buildHudPage() {
        moduleIndex = 0;
        addFilter(ModuleFilter.ALL, "TODOS", 0);
        addFilter(ModuleFilter.HUD, "HUD", 1);
        addFilter(ModuleFilter.PVP, "PVP", 2);
        addFilter(ModuleFilter.INFO, "INFO", 3);
        EditBox search = new EditBox(font, contentLeft, 82, Math.min(260, contentWidth), 20,
                Component.literal("Buscar módulos"));
        search.setHint(Component.literal("Buscar módulos…"));
        search.setValue(moduleSearch);
        search.setResponder(value -> { moduleSearch = value; layoutModuleCards(); });
        addRenderableWidget(search);
        addModule(ModuleFilter.HUD, "▣", "FPS", "Rendimiento en tiempo real", () -> config().showFps, value -> config().showFps = value);
        addModule(ModuleFilter.HUD, "⌁", "Frame time", "Estabilidad de cada frame", () -> config().showFrameTime, value -> config().showFrameTime = value);
        addModule(ModuleFilter.PVP, "●", "CPS", "Clics por segundo", () -> config().showCps, value -> config().showCps = value);
        addModule(ModuleFilter.HUD, "W", "Teclas", "WASD y botones del ratón", () -> config().showKeystrokes, value -> config().showKeystrokes = value);
        addModule(ModuleFilter.PVP, "◉", "Ping", "Latencia con el servidor", () -> config().showPing, value -> config().showPing = value);
        addModule(ModuleFilter.PVP, "+", "Cooldown", "Indicador de ataque", () -> config().showAttackCooldown, value -> config().showAttackCooldown = value);
        addModule(ModuleFilter.PVP, "◇", "Armadura e ítems", "Iconos, barra y durabilidad", () -> config().showArmor, value -> config().showArmor = value);
        addModule(ModuleFilter.PVP, "✦", "Efectos", "Pociones activas", () -> config().showEffects, value -> config().showEffects = value);
        addModule(ModuleFilter.PVP, "#", "Contadores PvP", "Manzanas, tótems y cargas", () -> config().showItemCounters, value -> config().showItemCounters = value);
        addModule(ModuleFilter.INFO, "X", "Coordenadas", "Posición XYZ", () -> config().showCoordinates, value -> config().showCoordinates = value);
        addModule(ModuleFilter.INFO, "N", "Dirección", "Orientación cardinal", () -> config().showDirection, value -> config().showDirection = value);
        addModule(ModuleFilter.INFO, "♧", "Bioma", "Bioma actual", () -> config().showBiome, value -> config().showBiome = value);
        addModule(ModuleFilter.INFO, "▤", "Memoria", "RAM utilizada", () -> config().showMemory, value -> config().showMemory = value);
        addModule(ModuleFilter.INFO, "↗", "Métricas", "Promedio y 1% low", () -> config().showSessionMetrics, value -> config().showSessionMetrics = value);
        layoutModuleCards();
        nextY = 112 + ((Math.max(moduleIndex, 1) + moduleColumns() - 1) / moduleColumns()) * 46 + 8;
        addCycle("HUD X", () -> config().hudX, value -> config().hudX = value, 0, 500, 10, " px", 0);
        addCycle("HUD Y", () -> config().hudY, value -> config().hudY = value, 0, 500, 10, " px", 1); nextRow();
        addCycle("Teclas X", () -> config().keysXPercent, value -> config().keysXPercent = value, 10, 90, 5, "%", 0);
        addCycle("Teclas Y", () -> config().keysY, value -> config().keysY = value, 0, 500, 10, " px", 1); nextRow();
        addRenderableWidget(Button.builder(Component.literal("Editar HUD visualmente"), button ->
                minecraft.setScreen(new HudEditorScreen(this)))
                .bounds(contentLeft, nextY, Math.min(308, contentWidth), 20).build());
    }

    private void buildPerformancePage() {
        addRenderableWidget(Button.builder(profileLabel(), button -> {
            OptimizationProfile.selected(config()).next().apply(config());
            applyLive();
            rebuildWidgets();
        }).bounds(contentLeft, nextY, Math.min(308, contentWidth), 20).build()); nextRow();
        addCycle("Renderizado", () -> config().renderDistance, value -> config().renderDistance = value, 2, 32, 2, " chunks", 0);
        addCycle("Simulación", () -> config().simulationDistance, value -> config().simulationDistance = value, 5, 32, 1, " chunks", 1); nextRow();
        addCycle("Distancia entidades", () -> config().entityDistancePercent, value -> config().entityDistancePercent = value, 50, 500, 25, "%", 0);
        addCycle("Mezcla biomas", () -> config().biomeBlendRadius, value -> config().biomeBlendRadius = value, 0, 7, 1, "", 1); nextRow();
        addCycle("Mipmaps", () -> config().mipmapLevels, value -> config().mipmapLevels = value, 0, 4, 1, "", 0);
        addCycle("FPS objetivo", () -> config().targetFps, value -> config().targetFps = value, 30, 240, 15, "", 1); nextRow();
        addCycle("Partículas/tick", () -> config().maxParticlesPerTick, value -> config().maxParticlesPerTick = value, 20, 1000, 20, "", 0);
        addToggle("Partículas adaptativas", () -> config().adaptiveParticles, value -> config().adaptiveParticles = value, 1); nextRow();
        addToggle("Sombras entidades", () -> config().entityShadows, value -> config().entityShadows = value, 0);
        addToggle("Movimiento cámara", () -> config().viewBobbing, value -> config().viewBobbing = value, 1);
    }

    private void buildAppearancePage() {
        addToggle("HUD compacto", () -> config().compactHud, value -> config().compactHud = value, 0);
        addToggle("Mostrar teclas", () -> config().showKeystrokes, value -> config().showKeystrokes = value, 1); nextRow();
        addToggle("Mostrar armadura", () -> config().showArmor, value -> config().showArmor = value, 0);
        addToggle("Mostrar efectos", () -> config().showEffects, value -> config().showEffects = value, 1); nextRow();
        addToggle("Coordenadas", () -> config().showCoordinates, value -> config().showCoordinates = value, 0);
        addToggle("Memoria", () -> config().showMemory, value -> config().showMemory = value, 1);
        nextRow();
        addCycle("Opacidad HUD", () -> config().hudOpacity, value -> config().hudOpacity = value, 20, 100, 10, "%", 0);
        addCycle("Escala HUD", () -> config().hudScalePercent, value -> config().hudScalePercent = value, 50, 150, 10, "%", 1); nextRow();
        addToggle("Modo captura (F8)", () -> config().captureMode, value -> config().captureMode = value, 0);
    }

    private void buildSettingsPage() {
        addToggle("Aplicar al juego", () -> config().applyVanillaOptimizations,
                value -> config().applyVanillaOptimizations = value, 0);
        addToggle("Límite partículas", () -> config().limitParticles, value -> config().limitParticles = value, 1); nextRow();
        addToggle("Reducir FPS en segundo plano", () -> config().reduceBackgroundFps,
                value -> config().reduceBackgroundFps = value, 0);
        addCycle("FPS segundo plano", () -> config().backgroundFps, value -> config().backgroundFps = value,
                10, 120, 10, "", 1);
        nextRow();
        if (minecraft.getCurrentServer() != null) {
            addRenderableWidget(Button.builder(Component.literal("Guardar perfil para " + minecraft.getCurrentServer().ip), button -> {
                config().serverProfiles.put(minecraft.getCurrentServer().ip, config().profile);
                config().serverHudProfiles.put(minecraft.getCurrentServer().ip, config().currentHudLayout());
                ClientConfig.save();
                button.setMessage(Component.literal("Rendimiento y HUD guardados"));
            }).bounds(contentLeft, nextY, Math.min(308, contentWidth), 20).build());
        }
    }

    private void openPage(Page selected) { page = selected; rebuildWidgets(); }

    private int sidebarWidth() { return config().menuCollapsed ? 48 : Math.min(172, Math.max(140, width / 5)); }
    private int columnWidth() { return Math.max(120, (contentWidth - 8) / 2); }
    private int columnX(int column) { return contentLeft + column * (columnWidth() + 8); }
    private int moduleColumns() { return contentWidth >= 540 ? 3 : 2; }
    private int moduleWidth() { return Math.max(100, (contentWidth - (moduleColumns() - 1) * 8) / moduleColumns()); }
    private void addFilter(ModuleFilter target, String label, int index) {
        Component text = Component.literal(label).withColor(moduleFilter == target ? 0xF4F4F2 : 0x8B938D);
        addRenderableWidget(Button.builder(text, button -> { moduleFilter = target; rebuildWidgets(); })
                .bounds(contentLeft + index * 63, 56, 58, 20).build());
    }
    private void addModule(ModuleFilter category, String icon, String label, String description,
                           BooleanSupplier getter, BooleanSetter setter) {
        if (moduleFilter != ModuleFilter.ALL && moduleFilter != category) return;
        ModuleCard card = new ModuleCard(contentLeft, 112, moduleWidth(), 38, icon, label, description, getter, setter);
        moduleCards.add(card);
        addRenderableWidget(card);
    }

    private void layoutModuleCards() {
        String query = moduleSearch.trim().toLowerCase(java.util.Locale.ROOT);
        moduleIndex = 0;
        for (ModuleCard card : moduleCards) {
            card.visible = query.isEmpty() || card.matches(query);
            card.active = card.visible;
            if (!card.visible) continue;
            int index = moduleIndex++;
            card.setX(contentLeft + (index % moduleColumns()) * (moduleWidth() + 8));
            card.setY(112 + (index / moduleColumns()) * 46);
            card.setWidth(moduleWidth());
        }
    }

    private void addToggle(String label, BooleanSupplier getter, BooleanSetter setter, int column) {
        addRenderableWidget(Button.builder(toggleLabel(label, getter.getAsBoolean()), button -> {
            boolean value = !getter.getAsBoolean();
            setter.set(value);
            button.setMessage(toggleLabel(label, value));
            applyLive();
        }).bounds(columnX(column), nextY, columnWidth(), 20).build());
    }

    private void addCycle(String label, IntSupplier getter, IntConsumer setter, int min, int max, int step,
                          String suffix, int column) {
        addRenderableWidget(Button.builder(valueLabel(label, getter.getAsInt(), suffix), button -> {
            int value = getter.getAsInt() + step;
            if (value > max) value = min;
            setter.accept(value);
            button.setMessage(valueLabel(label, value, suffix));
            config().profile = "CUSTOM";
            applyLive();
        }).bounds(columnX(column), nextY, columnWidth(), 20).build());
    }

    private void applyLive() { VanillaOptimizer.apply(minecraft); ClientConfig.save(); }
    private void nextRow() { nextY += 25; }
    private static ClientConfig config() { return ClientConfig.get(); }
    private static Component toggleLabel(String label, boolean enabled) { return Component.literal(label + ": " + (enabled ? "ON" : "OFF")); }
    private static Component valueLabel(String label, int value, String suffix) { return Component.literal(label + ": " + value + suffix); }
    private static Component profileLabel() { return Component.literal("Perfil: " + config().profile); }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        graphics.fill(0, 0, width, height, 0xD9101210);
        int sidebar = sidebarWidth();
        graphics.fill(0, 0, sidebar, height, 0xF0181B18);
        graphics.fill(sidebar - 1, 0, sidebar, height, 0xFF343934);
        graphics.drawString(font, config().menuCollapsed ? "A" : "AUREUS", 12, 17, ACCENT, true);
        graphics.drawString(font, pageTitle(), contentLeft, 18, TEXT, true);
        graphics.drawString(font, pageDescription(), contentLeft, 36, MUTED, false);
        if (page == Page.HOME) renderHomeSummary(graphics);
        super.render(graphics, mouseX, mouseY, delta);
    }

    private void renderHomeSummary(GuiGraphics graphics) {
        int y = 130;
        graphics.drawString(font, "ESTADO DEL CLIENTE", contentLeft, y, MUTED, false);
        graphics.drawString(font, "Aureus activo", contentLeft, y + 18, ACCENT, true);
        graphics.drawString(font, "Perfil  " + config().profile, contentLeft, y + 34, TEXT, false);
        graphics.drawString(font, "Renderizado  " + config().renderDistance + " chunks", contentLeft, y + 48, TEXT, false);
        graphics.drawString(font, "Objetivo  " + config().targetFps + " FPS", contentLeft, y + 62, TEXT, false);
        graphics.drawString(font, "Usa el menú izquierdo para personalizar cada módulo.", contentLeft, y + 88, MUTED, false);
    }

    private String pageTitle() {
        return switch (page) { case HOME -> "Inicio"; case HUD -> "HUD y PvP"; case PERFORMANCE -> "Rendimiento"; case APPEARANCE -> "Apariencia"; case SETTINGS -> "Ajustes"; };
    }
    private String pageDescription() {
        return switch (page) { case HOME -> "Panel principal de Aureus Client"; case HUD -> "Elementos visibles durante la partida"; case PERFORMANCE -> "Cambios aplicados inmediatamente"; case APPEARANCE -> "Organiza la información en pantalla"; case SETTINGS -> "Comportamiento general del cliente"; };
    }

    @Override
    public void onClose() { applyLive(); minecraft.setScreen(parent); }
    private final class ModuleCard extends AbstractWidget {
        private final String icon;
        private final String label;
        private final String description;
        private final BooleanSupplier getter;
        private final BooleanSetter setter;

        private ModuleCard(int x, int y, int width, int height, String icon, String label, String description,
                           BooleanSupplier getter, BooleanSetter setter) {
            super(x, y, width, height, Component.literal(label));
            this.icon = icon;
            this.label = label;
            this.description = description;
            this.getter = getter;
            this.setter = setter;
        }

        private boolean matches(String query) {
            return label.toLowerCase(java.util.Locale.ROOT).contains(query)
                    || description.toLowerCase(java.util.Locale.ROOT).contains(query);
        }

        @Override
        protected void renderWidget(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
            boolean enabled = getter.getAsBoolean();
            int x = getX();
            int y = getY();
            int right = getRight();
            int bottom = getBottom();

            // Soft shadow plus layered translucent surfaces: a lightweight glass effect
            // that relies on the pause screen blur instead of an extra per-card shader.
            graphics.fill(x + 2, y + 3, right + 2, bottom + 3, 0x38000000);
            int border = enabled ? 0xB8F4F4F2 : isHovered ? 0x906F756F : 0x553F4540;
            int background = isHovered ? 0xB52B2E2C : 0x9C1B1E1C;
            graphics.fill(x, y, right, bottom, border);
            graphics.fill(x + 1, y + 1, right - 1, bottom - 1, background);
            graphics.fill(x + 2, y + 2, right - 2, y + 3,
                    enabled ? 0x4DFFFFFF : 0x24FFFFFF);

            int iconSurface = enabled ? 0xD8F0F1EE : 0xA7353936;
            graphics.fill(x + 8, y + 8, x + 30, y + 30, iconSurface);
            graphics.fill(x + 9, y + 9, x + 29, y + 10,
                    enabled ? 0x5AFFFFFF : 0x22FFFFFF);
            graphics.drawCenteredString(font, icon, x + 19, y + 15,
                    enabled ? 0xFF151815 : 0xFFB8BDB8);
            graphics.drawString(font, label, getX() + 37, getY() + 8, 0xFFF2F4F1, false);
            graphics.drawString(font, description, getX() + 37, getY() + 21, 0xFF858C87, false);
            int switchX = getRight() - 31;
            graphics.fill(switchX, getY() + 13, getRight() - 8, getY() + 25,
                    enabled ? 0xFFE9ECE8 : 0xB8464B47);
            graphics.fill(switchX + 1, getY() + 14, getRight() - 9, getY() + 15,
                    enabled ? 0xFFFFFFFF : 0x32FFFFFF);
            int knobX = enabled ? getRight() - 16 : switchX + 2;
            graphics.fill(knobX, getY() + 15, knobX + 8, getY() + 23,
                    enabled ? 0xFF171A18 : 0xFFF4F5F3);
        }

        @Override
        public void onClick(MouseButtonEvent event, boolean doubleClick) {
            setter.set(!getter.getAsBoolean());
            applyLive();
        }

        @Override
        protected void updateWidgetNarration(NarrationElementOutput output) {
            defaultButtonNarrationText(output);
        }
    }
    @FunctionalInterface private interface BooleanSetter { void set(boolean value); }
}
