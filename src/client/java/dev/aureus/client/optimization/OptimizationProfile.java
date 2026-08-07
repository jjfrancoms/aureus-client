package dev.aureus.client.optimization;

import dev.aureus.client.config.ClientConfig;

public enum OptimizationProfile {
    CUSTOM(25, 90),
    MEMORY_SAVER(25, 90),
    MAX_FPS(60, 120),
    BALANCED(220, 75),
    QUALITY(650, 60);

    private final int particlesPerTick;
    private final int targetFps;

    OptimizationProfile(int particlesPerTick, int targetFps) {
        this.particlesPerTick = particlesPerTick;
        this.targetFps = targetFps;
    }

    public void apply(ClientConfig config) {
        config.profile = name();
        config.limitParticles = true;
        config.maxParticlesPerTick = particlesPerTick;
        config.targetFps = targetFps;
        config.adaptiveParticles = this != QUALITY;
        config.compactHud = this != QUALITY;
        switch (this) {
            case CUSTOM -> {
                config.renderDistance = 12;
                config.simulationDistance = 8;
                config.entityDistancePercent = 50;
                config.biomeBlendRadius = 0;
                config.mipmapLevels = 0;
                config.entityShadows = false;
                config.viewBobbing = false;
            }
            case MEMORY_SAVER -> {
                config.renderDistance = 4;
                config.simulationDistance = 5;
                config.entityDistancePercent = 50;
                config.biomeBlendRadius = 0;
                config.mipmapLevels = 0;
                config.entityShadows = false;
                config.viewBobbing = false;
            }
            case MAX_FPS -> {
                config.renderDistance = 6;
                config.simulationDistance = 5;
                config.entityDistancePercent = 50;
                config.biomeBlendRadius = 0;
                config.mipmapLevels = 0;
                config.entityShadows = false;
                config.viewBobbing = false;
            }
            case BALANCED -> {
                config.renderDistance = 10;
                config.simulationDistance = 6;
                config.entityDistancePercent = 75;
                config.biomeBlendRadius = 2;
                config.mipmapLevels = 2;
                config.entityShadows = false;
                config.viewBobbing = true;
            }
            case QUALITY -> {
                config.renderDistance = 16;
                config.simulationDistance = 10;
                config.entityDistancePercent = 100;
                config.biomeBlendRadius = 5;
                config.mipmapLevels = 4;
                config.entityShadows = true;
                config.viewBobbing = true;
            }
        }
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
