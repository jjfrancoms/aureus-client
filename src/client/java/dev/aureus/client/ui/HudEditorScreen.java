package dev.aureus.client.ui;

import dev.aureus.client.config.ClientConfig;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

import java.util.ArrayList;
import java.util.List;

/** Drag editor for every HUD module. Uses the pause-screen blur with no extra shader passes. */
public final class HudEditorScreen extends Screen {
    private static final int HEADER = 42;
    private final Screen parent;
    private String selected = "fps";
    private String dragging;
    private double dragOffsetX;
    private double dragOffsetY;

    public HudEditorScreen(Screen parent) {
        super(Component.literal("Editor HUD de Aureus"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        addRenderableWidget(Button.builder(Component.literal("Escala −"), button -> adjustScale(-10))
                .bounds(10, height - 28, 72, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Escala +"), button -> adjustScale(10))
                .bounds(86, height - 28, 72, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Opacidad −"), button -> adjustOpacity(-10))
                .bounds(166, height - 28, 82, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Opacidad +"), button -> adjustOpacity(10))
                .bounds(252, height - 28, 82, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Restablecer"), button -> {
            ClientConfig.get().hudElementStyles.clear();
            ClientConfig.save();
        }).bounds(width - 214, height - 28, 98, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Guardar"), button -> onClose())
                .bounds(width - 108, height - 28, 98, 20).build());
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        graphics.fill(0, 0, width, height, 0xA8080A09);
        graphics.drawCenteredString(font, "EDITOR HUD INDIVIDUAL", width / 2, 10, 0xFFF4F4F2);
        ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(selected);
        graphics.drawCenteredString(font, "Seleccionado: " + labelFor(selected) + " · escala " + style.scalePercent
                + "% · opacidad " + style.opacity + "%", width / 2, 23, 0xFF92988F);
        graphics.fill(width / 2, HEADER, width / 2 + 1, height - 36, 0x28FFFFFF);
        graphics.fill(6, (height + HEADER) / 2, width - 6, (height + HEADER) / 2 + 1, 0x28FFFFFF);
        for (Preview preview : previews()) renderCard(graphics, preview);
        super.render(graphics, mouseX, mouseY, delta);
    }

    private void renderCard(GuiGraphics graphics, Preview preview) {
        ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(preview.id);
        int x = style.x >= 0 ? style.x : preview.defaultX;
        int y = (style.y >= 0 ? style.y : preview.defaultY) + HEADER;
        int border = preview.id.equals(selected) ? 0xE8F4F4F2 : 0x704F5550;
        int backgroundAlpha = Math.clamp(style.opacity * 175 / 100, 42, 175);
        graphics.pose().pushMatrix();
        float scale = style.scalePercent / 100.0F;
        graphics.pose().translate(x, y); graphics.pose().scale(scale, scale); graphics.pose().translate(-x, -y);
        graphics.fill(x + 2, y + 3, x + preview.width + 2, y + preview.height + 3, 0x42000000);
        graphics.fill(x, y, x + preview.width, y + preview.height, border);
        graphics.fill(x + 1, y + 1, x + preview.width - 1, y + preview.height - 1,
                (backgroundAlpha << 24) | 0x001A1D1B);
        graphics.fill(x + 2, y + 2, x + preview.width - 2, y + 3, 0x45FFFFFF);
        graphics.drawString(font, preview.label, x + 6, y + Math.max(3, (preview.height - 8) / 2), 0xFFF4F4F2, false);
        graphics.pose().popMatrix();
    }

    private List<Preview> previews() {
        ClientConfig config = ClientConfig.get();
        List<Preview> output = new ArrayList<>();
        int x = config.hudX, y = config.hudY;
        output.add(new Preview("fps", "FPS 120", x, y, 58, 15)); y += 17;
        output.add(new Preview("frame_time", "Frame 8.3 ms", x, y, 86, 15)); y += 17;
        output.add(new Preview("session", "Prom. 116 · 1% low 82", x, y, 126, 15)); y += 17;
        output.add(new Preview("cps", "CPS 8", x, y, 52, 15)); y += 17;
        output.add(new Preview("ping", "Ping 42 ms", x, y, 70, 15)); y += 17;
        output.add(new Preview("memory", "RAM 2180/5120 MB", x, y, 112, 15)); y += 17;
        output.add(new Preview("coordinates", "XYZ 120 · 64 · -42", x, y, 108, 15)); y += 17;
        output.add(new Preview("direction", "Dirección NORTE", x, y, 92, 15)); y += 17;
        output.add(new Preview("biome", "Bioma plains", x, y, 78, 15));
        int keysX = width * config.keysXPercent / 100 - 31;
        output.add(new Preview("keystrokes", "W  A S D · LMB RMB", keysX, config.keysY, 108, 32));
        int combatX = config.combatX < 0 ? width + config.combatX : config.combatX;
        output.add(new Preview("attack", "Ataque 100%", combatX, config.combatY, 82, 15));
        output.add(new Preview("armor", "Armadura · herramientas", combatX, config.combatY + 20, 126, 34));
        output.add(new Preview("effects", "Efectos y pociones", combatX, config.combatY + 58, 108, 28));
        int itemsX = config.itemsX < 0 ? width + config.itemsX - 54 : config.itemsX;
        output.add(new Preview("items", "64  2  1", itemsX, height * config.itemsYPercent / 100 - HEADER, 58, 32));
        return output;
    }

    @Override
    public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
        if (super.mouseClicked(event, doubleClick)) return true;
        for (Preview preview : previews()) {
            ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(preview.id);
            int x = style.x >= 0 ? style.x : preview.defaultX;
            int y = (style.y >= 0 ? style.y : preview.defaultY) + HEADER;
            int scaledWidth = preview.width * style.scalePercent / 100;
            int scaledHeight = preview.height * style.scalePercent / 100;
            if (inside((int) event.x(), (int) event.y(), x, y, scaledWidth, scaledHeight)) {
                selected = preview.id; dragging = preview.id;
                dragOffsetX = event.x() - x; dragOffsetY = event.y() - y;
                return true;
            }
        }
        return false;
    }

    @Override
    public boolean mouseDragged(MouseButtonEvent event, double deltaX, double deltaY) {
        if (dragging == null) return super.mouseDragged(event, deltaX, deltaY);
        ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(dragging);
        style.x = Math.clamp((int) (event.x() - dragOffsetX), 0, width - 20);
        style.y = Math.clamp((int) (event.y() - dragOffsetY) - HEADER, 0, height - HEADER - 20);
        return true;
    }

    @Override
    public boolean mouseReleased(MouseButtonEvent event) {
        if (dragging != null) { dragging = null; ClientConfig.save(); return true; }
        return super.mouseReleased(event);
    }

    private void adjustScale(int change) {
        ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(selected);
        style.scalePercent = Math.clamp(style.scalePercent + change, 50, 150); ClientConfig.save();
    }

    private void adjustOpacity(int change) {
        ClientConfig.HudElementStyle style = ClientConfig.get().hudStyle(selected);
        style.opacity = Math.clamp(style.opacity + change, 20, 100); ClientConfig.save();
    }

    private static String labelFor(String id) {
        return switch (id) {
            case "frame_time" -> "Frame time"; case "session" -> "Promedio y 1% low";
            case "coordinates" -> "Coordenadas"; case "direction" -> "Dirección";
            case "keystrokes" -> "Teclas"; case "attack" -> "Ataque";
            case "armor" -> "Armadura"; case "effects" -> "Efectos";
            case "items" -> "Contadores"; default -> id.toUpperCase();
        };
    }

    private static boolean inside(int mouseX, int mouseY, int x, int y, int width, int height) {
        return mouseX >= x && mouseX < x + width && mouseY >= y && mouseY < y + height;
    }

    @Override
    public void onClose() { ClientConfig.save(); minecraft.setScreen(parent); }

    private record Preview(String id, String label, int defaultX, int defaultY, int width, int height) { }
}
