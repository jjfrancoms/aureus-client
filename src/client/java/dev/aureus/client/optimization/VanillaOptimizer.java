package dev.aureus.client.optimization;

import dev.aureus.client.config.ClientConfig;
import net.minecraft.client.Minecraft;

/** Applies conservative vanilla settings with stable, measurable GPU/CPU impact. */
public final class VanillaOptimizer {
    private VanillaOptimizer() {
    }

    public static void apply(Minecraft client) {
        ClientConfig config = ClientConfig.get();
        if (!config.applyVanillaOptimizations) {
            return;
        }

        client.options.renderDistance().set(config.renderDistance);
        client.options.simulationDistance().set(config.simulationDistance);
        client.options.entityDistanceScaling().set(config.entityDistancePercent / 100.0D);
        client.options.biomeBlendRadius().set(config.biomeBlendRadius);
        client.options.mipmapLevels().set(config.mipmapLevels);
        client.options.entityShadows().set(config.entityShadows);
        client.options.bobView().set(config.viewBobbing);
        client.options.save();
    }
}
