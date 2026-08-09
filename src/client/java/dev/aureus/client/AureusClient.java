package dev.aureus.client;

import dev.aureus.client.config.ClientConfig;
import dev.aureus.client.metrics.CombatMetrics;
import dev.aureus.client.metrics.FrameMetrics;
import dev.aureus.client.optimization.ParticleBudget;
import dev.aureus.client.optimization.VanillaOptimizer;
import dev.aureus.client.optimization.OptimizationProfile;
import dev.aureus.client.ui.PerformanceHud;
import dev.aureus.client.ui.ConfigScreen;
import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.minecraft.client.KeyMapping;
import net.minecraft.resources.Identifier;
import org.lwjgl.glfw.GLFW;

public final class AureusClient implements ClientModInitializer {
    public static final String MOD_ID = "aureus_client";
    private boolean initialProfileApplied;
    private String activeServer = "";

    @Override
    public void onInitializeClient() {
        ClientConfig.load();
        KeyMapping.Category category = KeyMapping.Category.register(
                Identifier.fromNamespaceAndPath(MOD_ID, "controls")
        );
        KeyMapping openMenu = KeyBindingHelper.registerKeyBinding(new KeyMapping(
                "key.aureus_client.open_menu", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_RIGHT_SHIFT, category
        ));
        KeyMapping captureMode = KeyBindingHelper.registerKeyBinding(new KeyMapping(
                "key.aureus_client.capture_mode", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_F8, category
        ));
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            if (!initialProfileApplied && client.options != null) {
                VanillaOptimizer.apply(client);
                initialProfileApplied = true;
            }
            String server = client.getCurrentServer() == null ? "" : client.getCurrentServer().ip;
            if (!server.equals(activeServer)) {
                activeServer = server;
                String profile = ClientConfig.get().serverProfiles.get(server);
                if (profile != null) {
                    try { OptimizationProfile.valueOf(profile).apply(ClientConfig.get()); VanillaOptimizer.apply(client); ClientConfig.save(); }
                    catch (IllegalArgumentException ignored) { }
                }
                ClientConfig.get().applyHudLayout(ClientConfig.get().serverHudProfiles.get(server));
                ClientConfig.save();
            }
            CombatMetrics.onEndTick(client);
            FrameMetrics.onEndTick(client);
            VanillaOptimizer.applyAdaptiveFps(client);
            ParticleBudget.reset();
            while (openMenu.consumeClick()) {
                client.setScreen(new ConfigScreen(client.screen));
            }
            while (captureMode.consumeClick()) {
                ClientConfig.get().captureMode = !ClientConfig.get().captureMode;
                ClientConfig.save();
                if (client.player != null) {
                    client.player.displayClientMessage(net.minecraft.network.chat.Component.literal(
                            ClientConfig.get().captureMode ? "Aureus: HUD oculto" : "Aureus: HUD visible"), true);
                }
            }
        });
        HudElementRegistry.addLast(
                Identifier.fromNamespaceAndPath(MOD_ID, "performance_hud"),
                PerformanceHud::render
        );
    }
}
