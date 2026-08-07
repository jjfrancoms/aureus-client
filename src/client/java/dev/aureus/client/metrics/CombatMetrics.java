package dev.aureus.client.metrics;

import net.minecraft.client.Minecraft;

import java.util.ArrayDeque;
import java.util.Deque;

public final class CombatMetrics {
    private static final Deque<Long> LEFT_CLICKS = new ArrayDeque<>();
    private static boolean attackWasDown;

    private CombatMetrics() {
    }

    public static void onEndTick(Minecraft client) {
        long now = System.currentTimeMillis();
        boolean attackDown = client.options.keyAttack.isDown();
        if (attackDown && !attackWasDown && client.screen == null) {
            LEFT_CLICKS.addLast(now);
        }
        attackWasDown = attackDown;
        trim(now);
    }

    public static int leftCps() {
        trim(System.currentTimeMillis());
        return LEFT_CLICKS.size();
    }

    private static void trim(long now) {
        while (!LEFT_CLICKS.isEmpty() && now - LEFT_CLICKS.peekFirst() > 1_000L) {
            LEFT_CLICKS.removeFirst();
        }
    }
}
