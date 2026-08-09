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
        if (client.options.hideGui || client.screen != null || config.captureMode) {
            return;
        }

        int y = config.hudY;
        int fps = Math.max(client.getFps(), 1);
        if (config.showFps) {
            drawModule(graphics, client, "fps", "FPS  " + fps, config.hudX, y, fps >= 60 ? ACCENT : WARNING);
            y += 11;
        }
        if (config.showFrameTime) {
            drawModule(graphics, client, "frame_time", "Frame  %.1f ms".formatted(1_000.0 / fps), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showSessionMetrics && FrameMetrics.averageFps() > 0) {
            drawModule(graphics, client, "session", "Prom. " + FrameMetrics.averageFps()
                    + " | 1% low " + FrameMetrics.onePercentLow(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showCps) {
            drawModule(graphics, client, "cps", "CPS  " + CombatMetrics.leftCps(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showPing && client.player != null && client.getConnection() != null) {
            PlayerInfo info = client.getConnection().getPlayerInfo(client.player.getUUID());
            if (info != null) {
                int latency = info.getLatency();
                drawModule(graphics, client, "ping", "Ping  " + latency + " ms", config.hudX, y, latency < 100 ? ACCENT : WARNING);
                y += 11;
            }
        }
        if (config.showMemory) {
            Runtime runtime = Runtime.getRuntime();
            long used = (runtime.totalMemory() - runtime.freeMemory()) / 1_048_576L;
            long max = runtime.maxMemory() / 1_048_576L;
            drawModule(graphics, client, "memory", "RAM  " + used + "/" + max + " MB", config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showCoordinates && client.player != null) {
            drawModule(graphics, client, "coordinates", "XYZ  %.1f  %.1f  %.1f".formatted(
                    client.player.getX(), client.player.getY(), client.player.getZ()), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showDirection && client.player != null) {
            drawModule(graphics, client, "direction", "Dirección  " + client.player.getDirection().getName().toUpperCase(), config.hudX, y, TEXT);
            y += 11;
        }
        if (config.showBiome && client.player != null && client.level != null) {
            String biome = client.level.getBiome(client.player.blockPosition()).unwrapKey()
                    .map(key -> key.identifier().getPath().replace('_', ' ')).orElse("desconocido");
            drawModule(graphics, client, "biome", "Bioma  " + biome, config.hudX, y, TEXT);
        }

        if (config.showKeystrokes) {
            int center = graphics.guiWidth() * config.keysXPercent / 100;
            int keysY = config.keysY;
            ClientConfig.HudElementStyle style = config.hudStyle("keystrokes");
            int baseX = style.x >= 0 ? style.x : center - 31;
            int baseY = style.y >= 0 ? style.y : keysY;
            pushScale(graphics, baseX, baseY, combinedScale(config, style));
            drawKey(graphics, client, "W", baseX + 25, baseY, client.options.keyUp.isDown(), style);
            drawKey(graphics, client, "A", baseX + 13, baseY + 12, client.options.keyLeft.isDown(), style);
            drawKey(graphics, client, "S", baseX + 25, baseY + 12, client.options.keyDown.isDown(), style);
            drawKey(graphics, client, "D", baseX + 37, baseY + 12, client.options.keyRight.isDown(), style);
            drawKey(graphics, client, "LMB", baseX, baseY + 25, client.options.keyAttack.isDown(), style);
            drawKey(graphics, client, "RMB", baseX + 38, baseY + 25, client.options.keyUse.isDown(), style);
            graphics.pose().popMatrix();
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
        int x = config.combatX < 0 ? graphicsWidth(client) + config.combatX : config.combatX;
        int y = config.combatY;
        if (config.showAttackCooldown) {
            ClientConfig.HudElementStyle style = config.hudStyle("attack");
            int moduleX = style.x >= 0 ? style.x : x; int moduleY = style.y >= 0 ? style.y : y;
            pushScale(graphics, moduleX, moduleY, combinedScale(config, style));
            int percent = Math.round(client.player.getAttackStrengthScale(0.0F) * 100.0F);
            graphics.fill(moduleX - 4, moduleY - 3, moduleX + 92, moduleY + 10, 0x65000000);
            drawStyled(graphics, client, "Ataque  " + percent + "%", moduleX, moduleY, percent == 100 ? ACCENT : TEXT, style);
            graphics.pose().popMatrix();
            y += 17;
        }
        if (config.showArmor) {
            ClientConfig.HudElementStyle style = config.hudStyle("armor");
            int moduleX = style.x >= 0 ? style.x : x; int moduleY = style.y >= 0 ? style.y : y;
            pushScale(graphics, moduleX, moduleY, combinedScale(config, style));
            int rowY = moduleY;
            for (EquipmentSlot slot : new EquipmentSlot[]{
                    EquipmentSlot.MAINHAND, EquipmentSlot.HEAD, EquipmentSlot.CHEST,
                    EquipmentSlot.LEGS, EquipmentSlot.FEET}) {
                ItemStack stack = client.player.getItemBySlot(slot);
                if (!stack.isEmpty() && stack.isDamageableItem()) {
                    int remaining = stack.getMaxDamage() - stack.getDamageValue();
                    float ratio = remaining / (float) stack.getMaxDamage();
                    int color = ratio <= 0.2F ? 0xFFFF6666 : ratio <= 0.45F ? 0xFFFFD166 : 0xFF72E69A;
                    graphics.fill(moduleX - 4, rowY - 2, moduleX + 112, rowY + 18, 0x72000000);
                    graphics.renderItem(stack, moduleX, rowY);
                    drawStyled(graphics, client, remaining + " / " + stack.getMaxDamage(), moduleX + 20, rowY + 2, color, style);
                    graphics.fill(moduleX + 20, rowY + 13, moduleX + 102, rowY + 15, 0xFF353A36);
                    graphics.fill(moduleX + 20, rowY + 13, moduleX + 20 + Math.round(82 * ratio), rowY + 15, color);
                    rowY += 21; y += 21;
                }
            }
            graphics.pose().popMatrix();
        }
        if (config.showEffects) {
            ClientConfig.HudElementStyle style = config.hudStyle("effects");
            int moduleX = style.x >= 0 ? style.x : x; int moduleY = style.y >= 0 ? style.y : y;
            pushScale(graphics, moduleX, moduleY, combinedScale(config, style));
            int rowY = moduleY;
            for (MobEffectInstance effect : client.player.getActiveEffects()) {
                String name = effect.getEffect().value().getDisplayName().getString();
                int seconds = effect.getDuration() / 20;
                graphics.fill(moduleX - 4, rowY - 3, moduleX + 112, rowY + 10, 0x65000000);
                drawStyled(graphics, client, name + "  " + seconds + "s", moduleX, rowY, TEXT, style);
                rowY += 15;
            }
            graphics.pose().popMatrix();
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
        ClientConfig config = ClientConfig.get();
        int x = config.itemsX < 0 ? graphics.guiWidth() + config.itemsX : config.itemsX;
        int y = Math.max(20, graphics.guiHeight() * config.itemsYPercent / 100 - visible * 10);
        ClientConfig.HudElementStyle style = config.hudStyle("items");
        x = style.x >= 0 ? style.x : x; y = style.y >= 0 ? style.y : y;
        pushScale(graphics, x, y, combinedScale(config, style));
        for (Item item : tracked) {
            int count = countItem(client, item);
            if (count <= 0) continue;
            ItemStack icon = new ItemStack(item);
            graphics.fill(x - 3, y - 2, x + 20, y + 19, 0x72000000);
            graphics.renderItem(icon, x, y);
            graphics.renderItemDecorations(client.font, icon, x, y, Integer.toString(count));
            y += 21;
        }
        graphics.pose().popMatrix();
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
        int alpha = Math.clamp(Math.round(255 * ClientConfig.get().hudOpacity / 100.0F), 32, 255);
        graphics.drawString(client.font, text, x, y, (color & 0x00FFFFFF) | (alpha << 24), true);
    }

    private static void drawKey(GuiGraphics graphics, Minecraft client, String key, int x, int y, boolean down,
                                ClientConfig.HudElementStyle style) {
        drawStyled(graphics, client, key, x, y, down ? ACCENT : INACTIVE, style);
    }

    private static void drawModule(GuiGraphics graphics, Minecraft client, String id, String text,
                                   int defaultX, int defaultY, int color) {
        ClientConfig config = ClientConfig.get(); ClientConfig.HudElementStyle style = config.hudStyle(id);
        int x = style.x >= 0 ? style.x : defaultX; int y = style.y >= 0 ? style.y : defaultY;
        pushScale(graphics, x, y, combinedScale(config, style));
        drawStyled(graphics, client, text, x, y, color, style);
        graphics.pose().popMatrix();
    }

    private static void drawStyled(GuiGraphics graphics, Minecraft client, String text, int x, int y, int color,
                                   ClientConfig.HudElementStyle style) {
        int opacity = ClientConfig.get().hudOpacity * style.opacity / 100;
        int alpha = Math.clamp(Math.round(255 * opacity / 100.0F), 32, 255);
        graphics.drawString(client.font, text, x, y, (color & 0x00FFFFFF) | (alpha << 24), true);
    }

    private static int combinedScale(ClientConfig config, ClientConfig.HudElementStyle style) {
        return Math.clamp(config.hudScalePercent * style.scalePercent / 100, 50, 225);
    }

    private static void pushScale(GuiGraphics graphics, int anchorX, int anchorY, int percent) {
        float scale = Math.clamp(percent, 50, 150) / 100.0F;
        graphics.pose().pushMatrix();
        graphics.pose().translate(anchorX, anchorY);
        graphics.pose().scale(scale, scale);
        graphics.pose().translate(-anchorX, -anchorY);
    }
}
