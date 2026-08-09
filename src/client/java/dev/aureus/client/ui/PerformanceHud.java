package dev.aureus.client.ui;

import dev.aureus.client.compat.CompatibilityReport;
import dev.aureus.client.config.ClientConfig;
import dev.aureus.client.metrics.CombatMetrics;
import dev.aureus.client.metrics.FrameMetrics;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.Items;

import java.util.List;

public final class PerformanceHud {
    private static final int TEXT = 0xFFF4F4F4;
    private static final int ACCENT = 0xFFF4F4F2;
    private static final int WARNING = 0xFFB8B8B8;
    private static final int INACTIVE = 0xFF777777;

    private PerformanceHud() {
    }

    public static void render(GuiGraphics graphics, DeltaTracker ignored) {
        Minecraft client = Minecraft.getInstance();
        ClientConfig config = ClientConfig.get();
        if (client.options.hideGui || client.screen != null) {
            return;
        }

        int y = config.hudY;
        int fps = Math.max(client.getFps(), 1);
        if (config.showFps) {
            draw(graphics, client, "FPS  " + fps, config.hudX, y, fps >= 60 ? ACCENT : WARNING);
            y += 11;
        }
        if (config.showFrameTime) {
            draw(graphics, client, "Frame  %.1f ms".formatted(1_000.0 / fps), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showSessionMetrics && FrameMetrics.averageFps() > 0) {
            draw(graphics, client, "Prom. " + FrameMetrics.averageFps()
                    + " | 1% low " + FrameMetrics.onePercentLow(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showCps) {
            draw(graphics, client, "CPS  " + CombatMetrics.leftCps(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showPing && client.player != null && client.getConnection() != null) {
            PlayerInfo info = client.getConnection().getPlayerInfo(client.player.getUUID());
            if (info != null) {
                int latency = info.getLatency();
                draw(graphics, client, "Ping  " + latency + " ms", config.hudX, y, latency < 100 ? ACCENT : WARNING);
                y += 11;
            }
        }
        if (config.showMemory) {
            Runtime runtime = Runtime.getRuntime();
            long used = (runtime.totalMemory() - runtime.freeMemory()) / 1_048_576L;
            long max = runtime.maxMemory() / 1_048_576L;
            draw(graphics, client, "RAM  " + used + "/" + max + " MB", config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showCoordinates && client.player != null) {
            draw(graphics, client, "XYZ  %.1f  %.1f  %.1f".formatted(
                    client.player.getX(), client.player.getY(), client.player.getZ()), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showDirection && client.player != null) {
            draw(graphics, client, "Dirección  " + client.player.getDirection().getName().toUpperCase(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showBiome && client.player != null && client.level != null) {
            String biome = client.level.getBiome(client.player.blockPosition()).unwrapKey()
                    .map(key -> key.identifier().getPath().replace('_', ' ')).orElse("desconocido");
            draw(graphics, client, "Bioma  " + biome, config.hudX, y, TEXT);
        }

        if (config.showKeystrokes) {
            int center = graphics.guiWidth() * config.keysXPercent / 100;
            int keysY = config.keysY;
            drawKey(graphics, client, "W", center - 6, keysY, client.options.keyUp.isDown());
            drawKey(graphics, client, "A", center - 18, keysY + 12, client.options.keyLeft.isDown());
            drawKey(graphics, client, "S", center - 6, keysY + 12, client.options.keyDown.isDown());
            drawKey(graphics, client, "D", center + 6, keysY + 12, client.options.keyRight.isDown());
            drawKey(graphics, client, "LMB", center - 31, keysY + 25, client.options.keyAttack.isDown());
            drawKey(graphics, client, "RMB", center + 7, keysY + 25, client.options.keyUse.isDown());
        }

        if (client.player != null) {
            renderCombatInfo(graphics, client, config);
            if (config.showItemCounters) renderItemCounters(graphics, client);
        }

        if (config.showCompatibility) {
            List<String> mods = CompatibilityReport.detectedPerformanceMods();
            if (!mods.isEmpty()) {
                draw(graphics, client, "Optimización: " + String.join(", ", mods), 6,
                        graphics.guiHeight() - 12, INACTIVE);
            }
        }
    }

    private static void renderCombatInfo(GuiGraphics graphics, Minecraft client, ClientConfig config) {
        int x = graphicsWidth(client) - 150;
        int y = 6;
        if (config.showAttackCooldown) {
            int percent = Math.round(client.player.getAttackStrengthScale(0.0F) * 100.0F);
            graphics.fill(x - 4, y - 3, x + 92, y + 10, 0x65000000);
            draw(graphics, client, "Ataque  " + percent + "%", x, y, percent == 100 ? ACCENT : TEXT);
            y += 17;
        }
        if (config.showArmor) {
            for (EquipmentSlot slot : new EquipmentSlot[]{
                    EquipmentSlot.MAINHAND, EquipmentSlot.HEAD, EquipmentSlot.CHEST,
                    EquipmentSlot.LEGS, EquipmentSlot.FEET}) {
                ItemStack stack = client.player.getItemBySlot(slot);
                if (!stack.isEmpty() && stack.isDamageableItem()) {
                    int remaining = stack.getMaxDamage() - stack.getDamageValue();
                    float ratio = remaining / (float) stack.getMaxDamage();
                    int color = ratio <= 0.2F ? 0xFFFF6666 : ratio <= 0.45F ? 0xFFFFD166 : 0xFF72E69A;
                    graphics.fill(x - 4, y - 2, x + 112, y + 18, 0x72000000);
                    graphics.renderItem(stack, x, y);
                    draw(graphics, client, remaining + " / " + stack.getMaxDamage(), x + 20, y + 2, color);
                    graphics.fill(x + 20, y + 13, x + 102, y + 15, 0xFF353A36);
                    graphics.fill(x + 20, y + 13, x + 20 + Math.round(82 * ratio), y + 15, color);
                    y += 21;
                }
            }
        }
        if (config.showEffects) {
            for (MobEffectInstance effect : client.player.getActiveEffects()) {
                String name = effect.getEffect().value().getDisplayName().getString();
                int seconds = effect.getDuration() / 20;
                graphics.fill(x - 4, y - 3, x + 112, y + 10, 0x65000000);
                draw(graphics, client, name + "  " + seconds + "s", x, y, TEXT);
                y += 15;
            }
        }
    }

    private static void renderItemCounters(GuiGraphics graphics, Minecraft client) {
        Item[] tracked = new Item[]{
                Items.GOLDEN_APPLE, Items.ENCHANTED_GOLDEN_APPLE, Items.TOTEM_OF_UNDYING,
                Items.WIND_CHARGE, Items.END_CRYSTAL, Items.ENDER_PEARL,
                Items.SPLASH_POTION, Items.LINGERING_POTION
        };
        int visible = 0;
        for (Item item : tracked) if (countItem(client, item) > 0) visible++;
        int x = graphics.guiWidth() - 27;
        int y = Math.max(42, graphics.guiHeight() / 2 - visible * 10);
        for (Item item : tracked) {
            int count = countItem(client, item);
            if (count <= 0) continue;
            ItemStack icon = new ItemStack(item);
            graphics.fill(x - 3, y - 2, x + 20, y + 19, 0x72000000);
            graphics.renderItem(icon, x, y);
            graphics.renderItemDecorations(client.font, icon, x, y, Integer.toString(count));
            y += 21;
        }
    }

    private static int countItem(Minecraft client, Item item) {
        int total = 0;
        for (int slot = 0; slot < client.player.getInventory().getContainerSize(); slot++) {
            ItemStack stack = client.player.getInventory().getItem(slot);
            if (stack.is(item)) total += stack.getCount();
        }
        return total;
    }

    private static int graphicsWidth(Minecraft client) {
        return client.getWindow().getGuiScaledWidth();
    }

    private static void draw(GuiGraphics graphics, Minecraft client, String text, int x, int y, int color) {
        graphics.drawString(client.font, text, x, y, color, true);
    }

    private static void drawKey(GuiGraphics graphics, Minecraft client, String key, int x, int y, boolean down) {
        draw(graphics, client, key, x, y, down ? ACCENT : INACTIVE);
    }
}
