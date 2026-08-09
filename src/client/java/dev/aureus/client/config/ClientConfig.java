package dev.aureus.client.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import net.fabricmc.loader.api.FabricLoader;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;

public final class ClientConfig {
    private static final int CURRENT_CONFIG_VERSION = 6;
    private static final Path PATH = FabricLoader.getInstance().getConfigDir().resolve("aureus-client.json");
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
    private static ClientConfig instance = new ClientConfig();

    public boolean showFps = true;
    public boolean showFrameTime = true;
    public boolean showSessionMetrics = true;
    public boolean showCps = true;
    public boolean showKeystrokes = true;
    public boolean showPing = true;
    public boolean showCoordinates = false;
    public boolean showAttackCooldown = true;
    public boolean showArmor = true;
    public boolean showEffects = true;
    public boolean showMemory = false;
    public boolean showCompatibility = false;
    public boolean showItemCounters = true;
    public boolean showBiome = true;
    public boolean showDirection = true;
    public boolean reduceBackgroundFps = true;
    public boolean compactHud = true;
    public boolean limitParticles = true;
    public boolean adaptiveParticles = true;
    public boolean applyVanillaOptimizations = true;
    public int backgroundFps = 30;
    public int maxParticlesPerTick = 25;
    public int targetFps = 90;
    public int renderDistance = 12;
    public int simulationDistance = 8;
    public int entityDistancePercent = 50;
    public int biomeBlendRadius = 0;
    public int mipmapLevels = 0;
    public boolean entityShadows = false;
    public boolean viewBobbing = false;
    public boolean menuCollapsed = false;
    public int hudX = 6;
    public int hudY = 6;
    public int keysXPercent = 50;
    public int keysY = 6;
    public int combatX = -150;
    public int combatY = 6;
    public int itemsX = -27;
    public int itemsYPercent = 50;
    public int hudOpacity = 78;
    public int hudScalePercent = 100;
    public boolean captureMode = false;
    public Map<String, String> serverProfiles = new HashMap<>();
    public Map<String, HudLayout> serverHudProfiles = new HashMap<>();
    public Map<String, HudElementStyle> hudElementStyles = new HashMap<>();
    public String profile = "CUSTOM";
    public int configVersion = CURRENT_CONFIG_VERSION;

    private ClientConfig() {
    }

    public static ClientConfig get() {
        return instance;
    }

    public static void load() {
        if (!Files.exists(PATH)) {
            save();
            return;
        }
        try (Reader reader = Files.newBufferedReader(PATH, StandardCharsets.UTF_8)) {
            ClientConfig loaded = GSON.fromJson(reader, ClientConfig.class);
            instance = loaded == null ? new ClientConfig() : loaded;
            if (instance.configVersion < 4) {
                instance.profile = "MAX_FPS";
                instance.renderDistance = 6;
                instance.simulationDistance = 5;
                instance.entityDistancePercent = 50;
                instance.biomeBlendRadius = 0;
                instance.mipmapLevels = 0;
                instance.entityShadows = false;
                instance.viewBobbing = false;
                instance.showCompatibility = false;
            }
            if (instance.hudOpacity <= 0) instance.hudOpacity = 78;
            if (instance.hudScalePercent <= 0) instance.hudScalePercent = 100;
            if (instance.itemsYPercent <= 0) instance.itemsYPercent = 50;
            instance.configVersion = CURRENT_CONFIG_VERSION;
            instance.backgroundFps = Math.clamp(instance.backgroundFps, 10, 120);
            instance.maxParticlesPerTick = Math.clamp(instance.maxParticlesPerTick, 20, 1000);
            instance.targetFps = Math.clamp(instance.targetFps, 30, 240);
            instance.renderDistance = Math.clamp(instance.renderDistance, 2, 32);
            instance.simulationDistance = Math.clamp(instance.simulationDistance, 5, 32);
            instance.entityDistancePercent = Math.clamp(instance.entityDistancePercent, 50, 500);
            instance.biomeBlendRadius = Math.clamp(instance.biomeBlendRadius, 0, 7);
            instance.mipmapLevels = Math.clamp(instance.mipmapLevels, 0, 4);
            instance.hudX = Math.clamp(instance.hudX, 0, 500);
            instance.hudY = Math.clamp(instance.hudY, 0, 500);
            instance.keysXPercent = Math.clamp(instance.keysXPercent, 10, 90);
            instance.keysY = Math.clamp(instance.keysY, 0, 500);
            instance.combatX = Math.clamp(instance.combatX, -500, 500);
            instance.combatY = Math.clamp(instance.combatY, 0, 500);
            instance.itemsX = Math.clamp(instance.itemsX, -500, 500);
            instance.itemsYPercent = Math.clamp(instance.itemsYPercent, 5, 95);
            instance.hudOpacity = Math.clamp(instance.hudOpacity, 20, 100);
            instance.hudScalePercent = Math.clamp(instance.hudScalePercent, 50, 150);
            if (instance.serverProfiles == null) instance.serverProfiles = new HashMap<>();
            if (instance.serverHudProfiles == null) instance.serverHudProfiles = new HashMap<>();
            if (instance.hudElementStyles == null) instance.hudElementStyles = new HashMap<>();
            for (HudElementStyle style : instance.hudElementStyles.values()) {
                style.x = Math.clamp(style.x, -1, 2000); style.y = Math.clamp(style.y, -1, 2000);
                style.scalePercent = Math.clamp(style.scalePercent, 50, 150);
                style.opacity = Math.clamp(style.opacity, 20, 100);
            }
            save();
        } catch (IOException | RuntimeException ignored) {
            instance = new ClientConfig();
        }
    }

    public static void save() {
        try {
            Files.createDirectories(PATH.getParent());
            try (Writer writer = Files.newBufferedWriter(PATH, StandardCharsets.UTF_8)) {
                GSON.toJson(instance, writer);
            }
        } catch (IOException ignored) {
            // A read-only config folder should not prevent Minecraft from starting.
        }
    }

    public HudLayout currentHudLayout() {
        HudLayout layout = new HudLayout(hudX, hudY, keysXPercent, keysY, combatX, combatY,
                itemsX, itemsYPercent, hudOpacity, hudScalePercent);
        for (Map.Entry<String, HudElementStyle> entry : hudElementStyles.entrySet()) {
            HudElementStyle copy = new HudElementStyle();
            copy.x = entry.getValue().x; copy.y = entry.getValue().y;
            copy.scalePercent = entry.getValue().scalePercent; copy.opacity = entry.getValue().opacity;
            layout.elements.put(entry.getKey(), copy);
        }
        return layout;
    }

    public void applyHudLayout(HudLayout layout) {
        if (layout == null) return;
        hudX = layout.hudX; hudY = layout.hudY; keysXPercent = layout.keysXPercent; keysY = layout.keysY;
        combatX = layout.combatX; combatY = layout.combatY; itemsX = layout.itemsX;
        itemsYPercent = layout.itemsYPercent; hudOpacity = layout.hudOpacity; hudScalePercent = layout.hudScalePercent;
        hudElementStyles = layout.elements == null ? new HashMap<>() : new HashMap<>(layout.elements);
    }

    public HudElementStyle hudStyle(String id) {
        return hudElementStyles.computeIfAbsent(id, ignored -> new HudElementStyle());
    }

    public static final class HudElementStyle {
        public int x = -1;
        public int y = -1;
        public int scalePercent = 100;
        public int opacity = 100;
    }

    public static final class HudLayout {
        public int hudX, hudY, keysXPercent, keysY, combatX, combatY, itemsX, itemsYPercent, hudOpacity, hudScalePercent;
        public Map<String, HudElementStyle> elements = new HashMap<>();
        public HudLayout(int hudX, int hudY, int keysXPercent, int keysY, int combatX, int combatY,
                         int itemsX, int itemsYPercent, int hudOpacity, int hudScalePercent) {
            this.hudX = hudX; this.hudY = hudY; this.keysXPercent = keysXPercent; this.keysY = keysY;
            this.combatX = combatX; this.combatY = combatY; this.itemsX = itemsX;
            this.itemsYPercent = itemsYPercent; this.hudOpacity = hudOpacity; this.hudScalePercent = hudScalePercent;
        }
    }
}
