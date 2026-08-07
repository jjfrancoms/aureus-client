package dev.aureus.client.compat;

import net.fabricmc.loader.api.FabricLoader;

import java.util.ArrayList;
import java.util.List;

public final class CompatibilityReport {
    private CompatibilityReport() {
    }

    public static List<String> detectedPerformanceMods() {
        List<String> detected = new ArrayList<>();
        addIfLoaded(detected, "sodium", "Sodium");
        addIfLoaded(detected, "lithium", "Lithium");
        addIfLoaded(detected, "ferritecore", "FerriteCore");
        addIfLoaded(detected, "immediatelyfast", "ImmediatelyFast");
        return detected;
    }

    private static void addIfLoaded(List<String> result, String id, String name) {
        if (FabricLoader.getInstance().isModLoaded(id)) {
            result.add(name);
        }
    }
}
