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

public final class ClientConfig {
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
    public boolean showCompatibility = true;
    public boolean reduceBackgroundFps = true;
    public boolean compactHud = true;
    public boolean limitParticles = true;
    public int backgroundFps = 30;
    public int maxParticlesPerTick = 250;
    public String profile = "BALANCED";

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
            instance.backgroundFps = Math.clamp(instance.backgroundFps, 10, 120);
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
}
