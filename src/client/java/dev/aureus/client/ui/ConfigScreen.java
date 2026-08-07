package dev.aureus.client.ui;

import dev.aureus.client.config.ClientConfig;
import dev.aureus.client.optimization.OptimizationProfile;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

import java.util.function.BooleanSupplier;

public final class ConfigScreen extends Screen {
    private final Screen parent;
    private int nextY;

    public ConfigScreen(Screen parent) {
        super(Component.literal("Aureus Client"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        nextY = 42;
        addRenderableWidget(Button.builder(profileLabel(), button -> {
            OptimizationProfile next = OptimizationProfile.selected(ClientConfig.get()).next();
            next.apply(ClientConfig.get());
            button.setMessage(profileLabel());
        }).bounds(width / 2 - 75, nextY, 150, 20).build());
        nextY += 24;
        addToggle("FPS", () -> ClientConfig.get().showFps, value -> ClientConfig.get().showFps = value, 0);
        addToggle("CPS", () -> ClientConfig.get().showCps, value -> ClientConfig.get().showCps = value, 1);
        nextY += 24;
        addToggle("Teclas", () -> ClientConfig.get().showKeystrokes, value -> ClientConfig.get().showKeystrokes = value, 0);
        addToggle("Ping", () -> ClientConfig.get().showPing, value -> ClientConfig.get().showPing = value, 1);
        nextY += 24;
        addToggle("Armadura", () -> ClientConfig.get().showArmor, value -> ClientConfig.get().showArmor = value, 0);
        addToggle("Efectos", () -> ClientConfig.get().showEffects, value -> ClientConfig.get().showEffects = value, 1);
        nextY += 24;
        addToggle("Cooldown", () -> ClientConfig.get().showAttackCooldown, value -> ClientConfig.get().showAttackCooldown = value, 0);
        addToggle("Coordenadas", () -> ClientConfig.get().showCoordinates, value -> ClientConfig.get().showCoordinates = value, 1);
        nextY += 24;
        addToggle("Frame time", () -> ClientConfig.get().showFrameTime, value -> ClientConfig.get().showFrameTime = value, 0);
        addToggle("Memoria", () -> ClientConfig.get().showMemory, value -> ClientConfig.get().showMemory = value, 1);
        nextY += 24;
        addToggle("Métricas sesión", () -> ClientConfig.get().showSessionMetrics,
                value -> ClientConfig.get().showSessionMetrics = value, 0);
        addToggle("Mods detectados", () -> ClientConfig.get().showCompatibility,
                value -> ClientConfig.get().showCompatibility = value, 1);
        nextY += 24;
        addToggle("Límite partículas", () -> ClientConfig.get().limitParticles,
                value -> ClientConfig.get().limitParticles = value, 0);

        addRenderableWidget(Button.builder(Component.literal("Guardar y cerrar"), button -> onClose())
                .bounds(width / 2 - 100, height - 32, 200, 20).build());
    }

    private void addToggle(String label, BooleanSupplier getter, BooleanSetter setter, int column) {
        int x = width / 2 - 154 + column * 158;
        addRenderableWidget(Button.builder(toggleLabel(label, getter.getAsBoolean()), button -> {
            boolean value = !getter.getAsBoolean();
            setter.set(value);
            button.setMessage(toggleLabel(label, value));
        }).bounds(x, nextY, 150, 20).build());
    }

    private static Component toggleLabel(String label, boolean enabled) {
        return Component.literal(label + ": " + (enabled ? "ON" : "OFF"));
    }

    private static Component profileLabel() {
        return Component.literal("Perfil: " + OptimizationProfile.selected(ClientConfig.get()).name());
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        super.render(graphics, mouseX, mouseY, delta);
        graphics.drawCenteredString(font, title, width / 2, 18, 0xFFFFFFFF);
    }

    @Override
    public void onClose() {
        ClientConfig.save();
        minecraft.setScreen(parent);
    }

    @FunctionalInterface
    private interface BooleanSetter {
        void set(boolean value);
    }
}
