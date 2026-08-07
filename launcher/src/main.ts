import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

type Status = {
  clientId: string;
  minecraftVersion: string;
  minecraftDirectory: string | null;
  modInstalled: boolean;
  javaAvailable: boolean;
};
type LoginStart = { authorizationUrl: string };
type MinecraftSession = { username: string; uuid: string };

const el = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector)!;

async function refresh() {
  const status = await invoke<Status>("launcher_status");
  el("#java-status").textContent = status.javaAvailable ? "LISTO" : "NO DETECTADO";
  el("#mc-status").textContent = status.minecraftDirectory ? status.minecraftVersion : "NO DETECTADO";
  el("#mod-status").textContent = status.modInstalled ? "INSTALADO" : "PENDIENTE";
  el<HTMLButtonElement>("#install").textContent = status.modInstalled ? "Reinstalar Aureus" : "Instalar Aureus";
  el("#message").textContent = status.modInstalled
    ? "Aureus está instalado y listo para Fabric 1.21.11."
    : "Instala el mod en tu perfil de Fabric con un solo clic.";

  el<HTMLButtonElement>("#login").onclick = async () => {
    try {
      el("#message").textContent = "Abriendo Microsoft…";
      const start = await invoke<LoginStart>("begin_microsoft_login");
      await openUrl(start.authorizationUrl);
      await invoke("complete_microsoft_login");
      el("#message").textContent = "Conectando Xbox y Minecraft…";
      const session = await invoke<MinecraftSession>("connect_minecraft");
      el("#message").textContent = `Minecraft conectado como ${session.username}.`;
      el<HTMLButtonElement>("#login").textContent = session.username;
      document.querySelector(".account strong")!.textContent = session.username;
      document.querySelector(".account small")!.textContent = "Minecraft Java";
      document.querySelector(".avatar")!.textContent = session.username.slice(0, 1).toUpperCase();
    } catch (error) {
      el("#message").textContent = `Inicio de sesión cancelado: ${String(error)}`;
    }
  };
}

window.addEventListener("DOMContentLoaded", async () => {
  el<HTMLInputElement>("#memory").addEventListener("input", event => {
    el<HTMLOutputElement>("#memory-value").value = `${(event.target as HTMLInputElement).value} GB`;
  });
  el<HTMLSelectElement>("#profile").addEventListener("change", event => {
    const profile = (event.target as HTMLSelectElement).value;
    el("#profile-score").textContent = profile === "MAX FPS" ? "MAX" : profile === "CALIDAD" ? "HQ" : "BAL";
  });
  el<HTMLButtonElement>("#install").addEventListener("click", async () => {
    el("#message").textContent = "Instalando…";
    try {
      const path = await invoke<string>("install_aureus_mod");
      el("#message").textContent = `Instalado en ${path}`;
      await refresh();
    } catch (error) {
      el("#message").textContent = `No se pudo instalar: ${String(error)}`;
    }
  });
  await refresh();
});
