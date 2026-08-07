package dev.aureus.client.optimization;

import dev.aureus.client.config.ClientConfig;
import net.minecraft.client.Minecraft;

public final class ParticleBudget {
    private static int acceptedThisTick;

    private ParticleBudget() {
    }

    public static void reset() {
        acceptedThisTick = 0;
    }

    public static boolean accept() {
        ClientConfig config = ClientConfig.get();
        if (!config.limitParticles) {
            return true;
        }
        int budget = config.maxParticlesPerTick;
        if (config.adaptiveParticles) {
            int fps = Math.max(Minecraft.getInstance().getFps(), 1);
            double pressure = Math.clamp((double) fps / config.targetFps, 0.25, 1.0);
            budget = Math.max(20, (int) (budget * pressure));
        }
        if (acceptedThisTick >= budget) {
            return false;
        }
        acceptedThisTick++;
        return true;
    }
}
