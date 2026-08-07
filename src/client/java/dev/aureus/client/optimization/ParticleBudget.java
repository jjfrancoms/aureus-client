package dev.aureus.client.optimization;

import dev.aureus.client.config.ClientConfig;

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
        if (acceptedThisTick >= config.maxParticlesPerTick) {
            return false;
        }
        acceptedThisTick++;
        return true;
    }
}
