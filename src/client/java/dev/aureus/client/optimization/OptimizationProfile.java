package dev.aureus.client.optimization;

import dev.aureus.client.config.ClientConfig;

public enum OptimizationProfile {
    MAX_FPS(80),
    BALANCED(250),
    QUALITY(700);

    private final int particlesPerTick;

    OptimizationProfile(int particlesPerTick) {
        this.particlesPerTick = particlesPerTick;
    }

    public void apply(ClientConfig config) {
        config.profile = name();
        config.limitParticles = true;
        config.maxParticlesPerTick = particlesPerTick;
        config.compactHud = this != QUALITY;
    }

    public OptimizationProfile next() {
        OptimizationProfile[] values = values();
        return values[(ordinal() + 1) % values.length];
    }

    public static OptimizationProfile selected(ClientConfig config) {
        try {
            return valueOf(config.profile);
        } catch (IllegalArgumentException exception) {
            return BALANCED;
        }
    }
}
