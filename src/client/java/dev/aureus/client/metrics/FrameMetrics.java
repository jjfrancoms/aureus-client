package dev.aureus.client.metrics;

import net.minecraft.client.Minecraft;

import java.util.Arrays;

public final class FrameMetrics {
    private static final int SAMPLE_COUNT = 600;
    private static final int[] SAMPLES = new int[SAMPLE_COUNT];
    private static int cursor;
    private static int size;
    private static long lastSample;

    private FrameMetrics() {
    }

    public static void onEndTick(Minecraft client) {
        long now = System.currentTimeMillis();
        if (now - lastSample < 250L) {
            return;
        }
        lastSample = now;
        SAMPLES[cursor] = Math.max(client.getFps(), 0);
        cursor = (cursor + 1) % SAMPLE_COUNT;
        size = Math.min(size + 1, SAMPLE_COUNT);
    }

    public static int averageFps() {
        if (size == 0) return 0;
        long total = 0;
        for (int i = 0; i < size; i++) total += SAMPLES[i];
        return Math.round((float) total / size);
    }

    public static int onePercentLow() {
        if (size == 0) return 0;
        int[] copy = Arrays.copyOf(SAMPLES, size);
        Arrays.sort(copy);
        int lowSamples = Math.max(1, size / 100);
        long total = 0;
        for (int i = 0; i < lowSamples; i++) total += copy[i];
        return Math.round((float) total / lowSamples);
    }
}
