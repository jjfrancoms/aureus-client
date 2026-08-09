package dev.aureus.client.ui;

import dev.aureus.client.config.ClientConfig;
import dev.aureus.client.optimization.OptimizationProfile;
import dev.aureus.client.optimization.VanillaOptimizer;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.minecraft.ChatFormatting;

import java.util.function.BooleanSupplier;
import java.util.function.IntConsumer;
import java.util.function.IntSupplier;

public final class ConfigScreen extends Screen {
    private static final int ACCENT = 0xFFF4F4F2;
    private static final int TEXT = 0xFFF2F4EF;
    private static final int MUTED = 0xFF92988F;
    private enum Page { HOME, HUD, PERFORMANCE, APPEARANCE, SETTINGS }

    private final Screen parent;
    private Page page = Page.HOME;
    private int nextY;
    private int contentLeft;
    private int contentWidth;

    public ConfigScreen(Screen parent) {
        super(Component.literal("Aureus Client"));
        this.parent = parent;
    }

    @Override
    protected void init() {
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
        addModule("▣", "FPS", () -> config().showFps, value -> config().showFps = value, 0);
        addModule("⌁", "Frame time", () -> config().showFrameTime, value -> config().showFrameTime = value, 1);
        addModule("●", "CPS", () -> config().showCps, value -> config().showCps = value, 2);
        addModule("W", "Teclas", () -> config().showKeystrokes, value -> config().showKeystrokes = value, 3);
        addModule("◉", "Ping", () -> config().showPing, value -> config().showPing = value, 4);
        addModule("+", "Cooldown", () -> config().showAttackCooldown, value -> config().showAttackCooldown = value, 5);
        addModule("◇", "Armadura", () -> config().showArmor, value -> config().showArmor = value, 6);
        addModule("✦", "Efectos", () -> config().showEffects, value -> config().showEffects = value, 7);
        addModule("X", "Coordenadas", () -> config().showCoordinates, value -> config().showCoordinates = value, 8);
        addModule("N", "Dirección", () -> config().showDirection, value -> config().showDirection = value, 9);
        addModule("♧", "Bioma", () -> config().showBiome, value -> config().showBiome = value, 10);
        addModule("▤", "Memoria", () -> config().showMemory, value -> config().showMemory = value, 11);
        addModule("↗", "Métricas", () -> config().showSessionMetrics, value -> config().showSessionMetrics = value, 12);
        nextY = 70 + ((13 + moduleColumns() - 1) / moduleColumns()) * 25 + 8;
        addCycle("HUD X", () -> config().hudX, value -> config().hudX = value, 0, 500, 10, " px", 0);
        addCycle("HUD Y", () -> config().hudY, value -> config().hudY = value, 0, 500, 10, " px", 1); nextRow();
        addCycle("Teclas X", () -> config().keysXPercent, value -> config().keysXPercent = value, 10, 90, 5, "%", 0);
        addCycle("Teclas Y", () -> config().keysY, value -> config().keysY = value, 0, 500, 10, " px", 1);
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
                ClientConfig.save();
                button.setMessage(Component.literal("Perfil de servidor guardado"));
            }).bounds(contentLeft, nextY, Math.min(308, contentWidth), 20).build());
        }
    }

    private void openPage(Page selected) { page = selected; rebuildWidgets(); }

    private int sidebarWidth() { return config().menuCollapsed ? 48 : Math.min(172, Math.max(140, width / 5)); }
    private int columnWidth() { return Math.max(120, (contentWidth - 8) / 2); }
    private int columnX(int column) { return contentLeft + column * (columnWidth() + 8); }
    private int moduleColumns() { return contentWidth >= 540 ? 3 : 2; }
    private int moduleWidth() { return Math.max(100, (contentWidth - (moduleColumns() - 1) * 8) / moduleColumns()); }
    private void addModule(String icon, String label, BooleanSupplier getter, BooleanSetter setter, int index) {
        int columns = moduleColumns();
        int x = contentLeft + (index % columns) * (moduleWidth() + 8);
        int y = 70 + (index / columns) * 25;
        addRenderableWidget(Button.builder(moduleLabel(icon, label, getter.getAsBoolean()), button -> {
            boolean value = !getter.getAsBoolean();
            setter.set(value);
            button.setMessage(moduleLabel(icon, label, value));
            applyLive();
        }).bounds(x, y, moduleWidth(), 20).build());
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
    private static Component moduleLabel(String icon, String label, boolean enabled) {
        return Component.literal((enabled ? "●  " : "○  ") + icon + "  " + label)
                .withStyle(enabled ? ChatFormatting.GREEN : ChatFormatting.GRAY);
    }
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
    @FunctionalInterface private interface BooleanSetter { void set(boolean value); }
}
