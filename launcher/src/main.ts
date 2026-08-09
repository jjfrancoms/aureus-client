import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Status = { clientId: string; minecraftVersion: string; minecraftDirectory: string | null; modInstalled: boolean; javaAvailable: boolean; fabricInstalled: boolean; fabricApiInstalled: boolean };
type LoginStart = { authorizationUrl: string };
type MinecraftSession = { username: string; uuid: string };
type Diagnostics = { operatingSystem: string; architecture: string; javaVersion: string; minecraftDirectory: string; fabricInstalled: boolean; fabricApiInstalled: boolean; aureusInstalled: boolean; latestLogTail: string };
type LaunchProgress = { percent: number; stage: string; detail: string };
type RuntimeStatus = { running: boolean; pid: number | null; startedAt: number | null };
type MinecraftVersionEntry = { id: string; versionType: string; releaseTime: string; installed: boolean };
type SelectedVersionStatus = { version: string; prepared: boolean; mode: string; modCount: number };
type ContentEntry = { name: string; kind: string; enabled: boolean; size: number; sha256: string };
type HardwareProfile = { memoryMb: number; cpuThreads: number; recommendedMemoryMb: number; profile: string; renderDistance: number; simulationDistance: number; gpuName: string; onBattery: boolean; operatingSystem: string };
type CrashAnalysis = { detected: boolean; summary: string; suspectedMods: string[]; actions: string[] };
type StoredAccount = { username: string; uuid: string; active: boolean };
type ModrinthHit = { project_id: string; slug: string; title: string; description: string; downloads: number; icon_url?: string };
type BenchmarkReport = { available: boolean; averageFps: number; onePercentLow: number; memoryUsedMb: number; samples: number; recommendation: string };
type BackupEntry = { path: string; createdAt: number; size: number };
type IntegrityReport = { valid: boolean; verifiedFiles: number; missingFiles: string[]; modifiedFiles: string[]; unmanagedFiles: string[]; summary: string };
type DownloadMetrics = { downloadedBytes: number; totalBytes: number; bytesPerSecond: number };
type SystemTelemetry = { processCpuPercent: number; processMemoryMb: number; gpuName: string; onBattery: boolean; thermalState: string };
type ModImpactReport = { modCount: number; totalSizeMb: number; likelyVisualCost: string[]; largeFiles: string[]; recommendation: string };
type ManagedInstance = { id: string; name: string; minecraftVersion: string; memoryMb: number; resolutionWidth: number; resolutionHeight: number; javaPath: string | null; jvmArgs: string[]; profile: string; createdAt: number };
type ViewName = "home" | "instances" | "performance" | "skins" | "account" | "settings";

const el = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector)!;
const all = <T extends HTMLElement>(selector: string) => [...document.querySelectorAll<T>(selector)];
const pageCopy: Record<ViewName, [string, string]> = {
  home: ["Inicio", "Prepara Minecraft para jugar con Aureus."],
  instances: ["Instancias", "Administra Aureus y su instalación de Minecraft."],
  performance: ["Rendimiento", "Prioriza fluidez, equilibrio o calidad visual."],
  skins: ["Skins", "Importa y previsualiza apariencias para Minecraft."],
  account: ["Cuenta", "Gestiona Microsoft, la licencia y el acceso sin conexión."],
  settings: ["Ajustes", "Configura tu cuenta y las preferencias del launcher."],
};
let activeInstance: ManagedInstance | null = null;
let toastTimer: number | undefined;
let selectedMinecraftVersion = "1.21.11";
let selectedInstanceId = "1.21.11";
let updateInProgress = false;
let pendingUpdate: NonNullable<Awaited<ReturnType<typeof check>>> | null = null;
let minecraftSession: MinecraftSession | null = null;
let installInProgress = false;
let launchRequestInProgress = false;
let gameLaunchInProgress = false;
let backgroundObjectUrl: string | null = null;
let skinObjectUrl: string | null = null;

async function installPendingUpdate() {
  if (!pendingUpdate || updateInProgress) return;
  const title = el<HTMLElement>("#updater-title");
  const status = el<HTMLElement>("#updater-status");
  const progress = el<HTMLElement>("#update-progress");
  const bar = el<HTMLElement>("#update-progress-bar");
  const button = el<HTMLButtonElement>("#check-update-now");
  updateInProgress = true;
  button.disabled = true;
  const globalButton = el<HTMLButtonElement>("#global-update");
  const globalLabel = el<HTMLElement>("#global-update-label");
  globalButton.hidden = false;
  globalButton.disabled = true;
  title.textContent = `Instalando Aureus ${pendingUpdate.version}`;
  status.textContent = "Descargando y verificando la firma…";
  progress.hidden = false;
  try {
    let downloaded = 0;
    let total = 0;
    await pendingUpdate.downloadAndInstall(event => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      if (event.event === "Progress") downloaded += event.data.chunkLength;
      if (event.event === "Finished") downloaded = total || downloaded;
      const percent = total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
      bar.style.width = `${percent}%`;
      status.textContent = total > 0 ? `Descargando y verificando… ${percent}%` : "Instalando actualización verificada…";
      globalLabel.textContent = total > 0 ? `Descargando ${percent}%` : "Instalando…";
    });
    title.textContent = "Actualización instalada";
    status.textContent = "Reiniciando Aureus…";
    bar.style.width = "100%";
    pendingUpdate = null;
    await relaunch();
  } catch (error) {
    title.textContent = "No se pudo actualizar";
    status.textContent = String(error);
    progress.hidden = true;
    globalLabel.textContent = "Reintentar descarga";
    showNotice(`Actualización no completada: ${String(error)}`, "error");
  } finally {
    updateInProgress = false;
    button.disabled = false;
    globalButton.disabled = false;
  }
}

async function checkForAureusUpdate(manual = false) {
  if (updateInProgress) return;
  const title = el<HTMLElement>("#updater-title");
  const status = el<HTMLElement>("#updater-status");
  const button = el<HTMLButtonElement>("#check-update-now");
  const globalButton = el<HTMLButtonElement>("#global-update");
  const globalLabel = el<HTMLElement>("#global-update-label");
  updateInProgress = true;
  button.disabled = true;
  button.textContent = "Buscando…";
  globalButton.hidden = false;
  globalButton.disabled = true;
  globalLabel.textContent = "Buscando actualización…";
  title.textContent = "Buscando actualización…";
  status.textContent = "Consultando el canal estable de Aureus.";
  try {
    pendingUpdate = await check();
    if (!pendingUpdate) {
      title.textContent = "Aureus está actualizado";
      status.textContent = "Tienes la versión estable más reciente.";
      button.textContent = "Buscar ahora";
      globalButton.hidden = true;
      if (manual) showNotice("No hay actualizaciones disponibles.", "success");
      return;
    }
    title.textContent = `Aureus ${pendingUpdate.version} está disponible`;
    status.textContent = pendingUpdate.body || "Incluye mejoras y correcciones nuevas.";
    button.textContent = "Descargar actualización";
    globalButton.hidden = false;
    globalButton.disabled = false;
    globalLabel.textContent = `Actualizar a ${pendingUpdate.version}`;
    showNotice(`Nueva versión ${pendingUpdate.version} disponible.`, "neutral");
  } catch (error) {
    const message = String(error);
    const platformPending = message.includes("fallback platforms") || message.includes("platforms` object");
    title.textContent = platformPending ? "Actualización de Windows en preparación" : "No se pudo comprobar";
    status.textContent = platformPending ? "macOS terminó primero. Aureus volverá a comprobar cuando el instalador de Windows esté publicado." : message;
    button.textContent = "Reintentar";
    globalButton.hidden = false;
    globalButton.disabled = false;
    globalLabel.textContent = platformPending ? "Windows aún se está preparando" : "Reintentar actualización";
    if (manual) showNotice(platformPending ? "La versión de Windows todavía se está generando. Intenta nuevamente en unos minutos." : `No se pudo buscar actualizaciones: ${message}`, platformPending ? "neutral" : "error");
  } finally {
    updateInProgress = false;
    button.disabled = false;
  }
}

const mediaDatabase = () => new Promise<IDBDatabase>((resolve, reject) => {
  const request = indexedDB.open("aureus-media", 1);
  request.onupgradeneeded = () => request.result.createObjectStore("assets");
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});

async function saveAsset(key: string, file: Blob) {
  const database = await mediaDatabase();
  await new Promise<void>((resolve, reject) => {
    const transaction = database.transaction("assets", "readwrite");
    transaction.objectStore("assets").put(file, key);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
  });
  database.close();
}

async function loadAsset(key: string): Promise<Blob | null> {
  const database = await mediaDatabase();
  const value = await new Promise<Blob | null>((resolve, reject) => {
    const request = database.transaction("assets").objectStore("assets").get(key);
    request.onsuccess = () => resolve(request.result ?? null);
    request.onerror = () => reject(request.error);
  });
  database.close();
  return value;
}

async function removeAsset(key: string) {
  const database = await mediaDatabase();
  await new Promise<void>((resolve, reject) => {
    const transaction = database.transaction("assets", "readwrite");
    transaction.objectStore("assets").delete(key);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
  });
  database.close();
}

function displayBackground(blob: Blob | null) {
  const image = el<HTMLImageElement>("#custom-background");
  const stage = el(".launcher-stage");
  if (backgroundObjectUrl) {
    URL.revokeObjectURL(backgroundObjectUrl);
    backgroundObjectUrl = null;
  }
  if (!blob) {
    image.removeAttribute("src");
    image.hidden = true;
    stage.classList.remove("has-media");
    return;
  }
  backgroundObjectUrl = URL.createObjectURL(blob);
  image.src = backgroundObjectUrl;
  image.hidden = false;
  stage.classList.add("has-media");
}

function displaySkin(blob: Blob | null, name = "Skin importada") {
  const canvas = el(".skin-canvas");
  if (!blob) return;
  if (skinObjectUrl) URL.revokeObjectURL(skinObjectUrl);
  skinObjectUrl = URL.createObjectURL(blob);
  canvas.style.backgroundImage = `url(${skinObjectUrl})`;
  canvas.classList.add("has-skin");
  el("#skin-name").textContent = name;
  el("#skin-library").textContent = name;
  el("#skin-library").classList.add("has-item");
}

function showNotice(message: string, tone: "neutral" | "success" | "error" = "neutral") {
  const notice = el("#notice");
  notice.textContent = message;
  notice.dataset.tone = tone;
}

function showToast(message: string, tone: "success" | "error" = "success") {
  const toast = el("#toast");
  window.clearTimeout(toastTimer);
  toast.textContent = message;
  toast.dataset.tone = tone;
  toast.hidden = false;
  requestAnimationFrame(() => toast.classList.add("visible"));
  toastTimer = window.setTimeout(() => {
    toast.classList.remove("visible");
    window.setTimeout(() => { toast.hidden = true; }, 180);
  }, 3200);
}

function switchView(view: ViewName) {
  all(".nav-item").forEach(button => {
    const active = button.dataset.view === view;
    button.classList.toggle("active", active);
    button.setAttribute("aria-current", active ? "page" : "false");
  });
  all(".view").forEach(panel => panel.classList.toggle("active", panel.dataset.viewPanel === view));
  el("#page-title").textContent = pageCopy[view][0];
  el("#page-description").textContent = pageCopy[view][1];
  localStorage.setItem("aureus.view", view);
  document.body.classList.toggle("home-active", view === "home");
  document.querySelector("main")?.scrollTo({ top: 0, behavior: "smooth" });
}

function applyStatus(status: Status) {
  el("#java-status").textContent = status.javaAvailable ? "Listo" : "No detectado";
  el("#mc-status").textContent = status.minecraftDirectory ? status.minecraftVersion : "No detectado";
  el("#mod-status").textContent = status.modInstalled ? "Instalado" : "Pendiente";
  el("#minecraft-path").textContent = status.minecraftDirectory ?? "No se encontró la carpeta de Minecraft";
  el("#client-id").textContent = status.clientId;
}

async function refresh() {
  try {
    const [status, selected] = await Promise.all([
      invoke<Status>("launcher_status"), invoke<SelectedVersionStatus>("selected_version_status")
    ]);
    applyStatus(status);
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
    const selectedCopy = selected.mode === "fabric" ? `Fabric · ${selected.modCount} mods` : selected.mode === "vanilla" ? "Vanilla preparado" : "Pendiente de preparar";
    el("#version-compatibility").textContent = selectedCopy;
    el("#instance-mode").textContent = selectedCopy;
    el("#instance-state").textContent = selected.prepared ? "Preparada" : "Pendiente";
    el("#instance-state").classList.toggle("good", selected.prepared);
    all<HTMLButtonElement>(".install-action").forEach(button => button.textContent = selected.prepared ? "Reparar" : "Preparar");
    const ready = selected.prepared && status.javaAvailable;
    el("#health-label").textContent = ready ? "LISTO" : "ACCIÓN NECESARIA";
    el("#health-detail").textContent = ready ? `Minecraft ${selected.version} está preparado` : selected.prepared ? "Java necesita reparación" : `Prepara Minecraft ${selected.version}`;
    showNotice(selected.prepared ? `Minecraft ${selected.version} está preparado.` : `Minecraft ${selected.version} aún debe prepararse.`, selected.prepared ? "success" : "neutral");
  } catch (error) {
    showNotice(`No se pudo revisar el sistema: ${String(error)}`, "error");
  }
}

async function installMod() {
  if (installInProgress) {
    showToast("La preparación ya está en curso.", "error");
    return false;
  }
  installInProgress = true;
  showNotice(`Preparando Minecraft ${selectedMinecraftVersion}…`);
  all<HTMLButtonElement>(".install-action").forEach(button => button.disabled = true);
  const metricsTimer = window.setInterval(async () => {
    try {
      const metrics = await invoke<DownloadMetrics>("download_metrics");
      if (!metrics.totalBytes) return;
      const downloaded = (metrics.downloadedBytes / 1048576).toFixed(1);
      const total = (metrics.totalBytes / 1048576).toFixed(1);
      const speed = (metrics.bytesPerSecond / 1048576).toFixed(1);
      el("#launch-detail").textContent = `${downloaded} / ${total} MB · ${speed} MB/s`;
    } catch { /* The native core may still be starting. */ }
  }, 400);
  try {
    const path = await invoke<string>("install_selected_version");
    selectedInstanceId = selectedMinecraftVersion;
    await refreshManagedInstances(selectedInstanceId);
    await refresh();
    showNotice(`Instalación completada: ${path}`, "success");
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
    el("#version-compatibility").textContent = path.includes("Vanilla") ? "Vanilla preparado" : "Fabric y mods preparados";
    return true;
  } catch (error) {
    showNotice(`No se pudo instalar: ${String(error)}`, "error");
    return false;
  } finally {
    installInProgress = false;
    window.clearInterval(metricsTimer);
    all<HTMLButtonElement>(".install-action").forEach(button => button.disabled = false);
  }
}

async function requestMinecraftLaunch() {
  if (launchRequestInProgress || installInProgress) {
    showToast("Aureus ya está procesando esta acción.", "error");
    return;
  }
  launchRequestInProgress = true;
  all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = true);
  try {
    try {
      minecraftSession = await invoke<MinecraftSession | null>("current_minecraft_session");
    } catch {
      minecraftSession = null;
    }
    if (!minecraftSession) {
      showToast("Inicia sesión con Microsoft para poder jugar.", "error");
      switchView("account");
      return;
    }
    const runtime = await invoke<RuntimeStatus>("runtime_status");
    if (runtime.running) {
      showToast("Minecraft ya está ejecutándose.", "error");
      return;
    }
    const selected = await invoke<SelectedVersionStatus>("selected_version_status");
    if (!selected.prepared) {
      showToast(`Preparando Minecraft ${selected.version} por primera vez…`, "success");
      if (!await installMod()) return;
      const prepared = await invoke<SelectedVersionStatus>("selected_version_status");
      if (!prepared.prepared) return;
    }
    const dialog = el<HTMLDialogElement>("#launch-confirmation");
    el("#confirm-version").textContent = selectedMinecraftVersion;
    el("#confirm-instance").textContent = activeInstance?.name ?? selectedInstanceId;
    el("#confirm-memory").textContent = `${Math.round((activeInstance?.memoryMb ?? Number(localStorage.getItem("aureus.memory") ?? "5") * 1024) / 1024)} GB`;
  el("#confirm-profile").textContent = activeInstance?.profile ?? localStorage.getItem("aureus.profile") ?? "Personalizado";
    el("#confirm-content").textContent = el("#version-compatibility").textContent ?? "Preparado";
    dialog.showModal();
  } catch (error) {
    showNotice(`No se pudo preparar el inicio: ${String(error)}`, "error");
    showToast("No se pudo iniciar Minecraft.", "error");
  } finally {
    launchRequestInProgress = false;
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
    void refreshRuntime();
  }
}

async function playMinecraft() {
  if (gameLaunchInProgress) return;
  gameLaunchInProgress = true;
  const confirmButton = el<HTMLButtonElement>("#confirm-launch");
  confirmButton.disabled = true;
  el<HTMLDialogElement>("#launch-confirmation").close();
  showNotice("Iniciando Minecraft directamente con Fabric…");
  el("#launch-progress").hidden = false;
  el<HTMLButtonElement>("#cancel-launch").hidden = false;
  all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = true);
  try {
    const result = await invoke<string>("launch_selected_minecraft");
    showNotice(result, "success");
  } catch (error) {
    showNotice(String(error), "error");
    el("#launch-stage").textContent = "No se pudo iniciar";
    el("#launch-detail").textContent = String(error);
    if (String(error).includes("Inicia sesión")) switchView("account");
  } finally {
    gameLaunchInProgress = false;
    confirmButton.disabled = false;
    el<HTMLButtonElement>("#cancel-launch").hidden = true;
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
  }
}

async function cancelLaunch() {
  try {
    await invoke("cancel_launch");
    el("#launch-stage").textContent = "Cancelado";
    el("#launch-detail").textContent = "El inicio fue cancelado";
    el<HTMLButtonElement>("#cancel-launch").hidden = true;
    window.setTimeout(() => {
      el("#launch-progress").hidden = true;
      el("#launch-progress-bar").style.width = "0%";
      el("#launch-percent").textContent = "0%";
      el("#launch-stage").textContent = "Preparando";
      el("#launch-detail").textContent = "Comprobando componentes…";
      all<HTMLButtonElement>(".play-action").forEach(button => { button.hidden = false; button.disabled = false; });
    }, 650);
  } catch (error) {
    showNotice(`No se pudo cancelar: ${String(error)}`, "error");
  }
}

async function killMinecraft() {
  const button = el<HTMLButtonElement>("#kill-game");
  button.disabled = true;
  try {
    showNotice(await invoke<string>("kill_minecraft"));
  } catch (error) {
    showNotice(String(error), "error");
    button.disabled = false;
  }
}

async function refreshRuntime() {
  try {
    const runtime = await invoke<RuntimeStatus>("runtime_status");
    el<HTMLButtonElement>("#kill-game").hidden = !runtime.running;
    all<HTMLButtonElement>(".play-action").forEach(button => button.hidden = runtime.running);
  } catch (error) {
    console.warn("No se pudo consultar la instancia", error);
  }
}

async function loadVersionCatalog() {
  const selector = el<HTMLSelectElement>("#minecraft-version");
  try {
    const [versions, selected] = await Promise.all([
      invoke<MinecraftVersionEntry[]>("version_catalog"), invoke<string>("selected_version")
    ]);
    const groups = new Map<string, HTMLOptGroupElement>();
    const groupNames: Record<string, string> = { release: "Versiones estables", snapshot: "Snapshots", old_beta: "Beta históricas", old_alpha: "Alpha históricas" };
    for (const version of versions) {
      let group = groups.get(version.versionType);
      if (!group) {
        group = document.createElement("optgroup");
        group.label = groupNames[version.versionType] ?? version.versionType;
        groups.set(version.versionType, group);
      }
      const option = document.createElement("option");
      option.value = version.id;
      option.textContent = `${version.id}${version.id === "1.21.11" ? " · Predeterminada" : ""}${version.installed ? " · Descargada" : ""}`;
      group.append(option);
    }
    selector.replaceChildren(...groups.values());
    selector.value = versions.some(version => version.id === selected) ? selected : "1.21.11";
    selectedMinecraftVersion = selector.value;
    selectedInstanceId = selector.value;
    all(".selected-version-label").forEach(label => label.textContent = selector.value);
    const initialStatus = await invoke<SelectedVersionStatus>("selected_version_status");
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
    el("#version-compatibility").textContent = initialStatus.mode === "aureus" ? "Aureus completo" : initialStatus.mode === "fabric" ? `Fabric · ${initialStatus.modCount} mods` : initialStatus.mode === "vanilla" ? "Vanilla preparado" : "Pendiente de preparar";
    el("#instance-mode").textContent = el("#version-compatibility").textContent ?? "Pendiente";
  } catch (error) {
    showNotice(`No se pudo cargar el catálogo oficial: ${String(error)}`, "error");
  }
  selector.addEventListener("change", async () => {
    const previous = selectedMinecraftVersion;
    selector.disabled = true;
    try {
      const version = await invoke<string>("select_version", { version: selector.value });
      selectedMinecraftVersion = version;
      selectedInstanceId = version;
      await refreshManagedInstances(version);
      all(".selected-version-label").forEach(label => label.textContent = version);
      const status = await invoke<SelectedVersionStatus>("selected_version_status");
      el("#version-compatibility").textContent = status.mode === "aureus" ? "Aureus completo" : status.mode === "fabric" ? `Fabric · ${status.modCount} mods` : status.mode === "vanilla" ? "Vanilla preparado" : "Pendiente de preparar y resolver mods";
      el("#instance-mode").textContent = el("#version-compatibility").textContent ?? "Pendiente";
      all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
      showNotice(`Versión ${version} seleccionada. Prepara la instancia antes de jugar.`);
    } catch (error) {
      selector.value = previous;
      showNotice(`No se pudo cambiar de versión: ${String(error)}`, "error");
    } finally {
      selector.disabled = false;
    }
  });
}

async function showInstanceContent() {
  const manager = el("#content-manager");
  const list = el("#content-list");
  manager.hidden = false;
  list.replaceChildren(Object.assign(document.createElement("p"), { textContent: "Revisando contenido…", className: "muted-copy" }));
  try {
    const entries = await invoke<ContentEntry[]>("list_instance_content", { instanceId: selectedInstanceId });
    if (!entries.length) {
      list.replaceChildren(Object.assign(document.createElement("p"), { textContent: "Esta instancia todavía no contiene mods, paquetes, shaders ni datapacks.", className: "muted-copy" }));
      return;
    }
    list.replaceChildren(...entries.map(entry => {
      const row = document.createElement("div"); row.className = "content-entry";
      const name = document.createElement("strong"); name.textContent = entry.name;
      const meta = document.createElement("small"); meta.textContent = `${entry.kind} · ${(entry.size / 1048576).toFixed(1)} MB`;
      const toggle = document.createElement("button"); toggle.className = "secondary"; toggle.textContent = entry.enabled ? "Desactivar" : "Activar";
      toggle.addEventListener("click", async () => { await invoke("toggle_instance_content", { instanceId: selectedInstanceId, kind: entry.kind, name: entry.name, enabled: !entry.enabled }); await showInstanceContent(); });
      row.append(name, meta, toggle); return row;
    }));
  } catch (error) { showNotice(`No se pudo leer el contenido: ${String(error)}`, "error"); }
}

async function showBackups() {
  const panel = el("#recovery-result");
  panel.hidden = false;
  panel.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "panel-title";
  heading.innerHTML = "<div><span class=\"panel-kicker\">RECUPERACIÓN</span><h3>Respaldos de la instancia</h3></div>";
  panel.append(heading);
  try {
    const backups = await invoke<BackupEntry[]>("list_instance_backups", { instanceId: selectedInstanceId });
    if (!backups.length) {
      const empty = document.createElement("p"); empty.className = "muted-copy"; empty.textContent = "Todavía no existen respaldos para esta instancia."; panel.append(empty); return;
    }
    const list = document.createElement("div"); list.className = "content-list";
    backups.forEach((backup, index) => {
      const row = document.createElement("div"); row.className = "content-entry";
      const copy = document.createElement("div");
      const title = document.createElement("strong"); title.textContent = index === 0 ? "Respaldo más reciente" : `Respaldo ${index + 1}`;
      const detail = document.createElement("small"); detail.textContent = `${new Date(backup.createdAt * 1000).toLocaleString()} · ${(backup.size / 1048576).toFixed(1)} MB`;
      copy.append(title, detail);
      const restore = document.createElement("button"); restore.className = "secondary"; restore.textContent = "Restaurar";
      restore.addEventListener("click", async () => {
        if (!window.confirm("Aureus respaldará el estado actual y restaurará esta copia. ¿Continuar?")) return;
        restore.disabled = true;
        try { showNotice(await invoke<string>("restore_instance_backup", { instanceId: selectedInstanceId, backupPath: backup.path }), "success"); }
        catch (error) { showNotice(`No se pudo restaurar: ${String(error)}`, "error"); }
        finally { restore.disabled = false; }
      });
      row.append(copy, restore); list.append(row);
    });
    panel.append(list);
  } catch (error) { panel.textContent = `No se pudieron leer los respaldos: ${String(error)}`; }
}

async function searchMods() {
  const query = el<HTMLInputElement>("#mod-search").value.trim(); if (!query) return;
  const results = el("#mod-search-results"); results.hidden = false; results.textContent = "Buscando…";
  try {
    const hits = await invoke<ModrinthHit[]>("search_modrinth", { query, version: selectedMinecraftVersion });
    if (!hits.length) {
      results.textContent = "No se encontraron mods Fabric compatibles con esta versión.";
      return;
    }
    results.replaceChildren(...hits.map(hit => {
      const row = document.createElement("div"); row.className = "content-entry";
      const title = document.createElement("strong"); title.textContent = hit.title;
      const detail = document.createElement("small"); detail.textContent = `${hit.downloads.toLocaleString()} descargas`;
      const install = document.createElement("button"); install.className = "secondary"; install.textContent = "Instalar";
      install.addEventListener("click", async () => { install.disabled = true; try { showNotice(await invoke<string>("install_modrinth", { instanceId: selectedInstanceId, version: selectedMinecraftVersion, projectId: hit.project_id }), "success"); await showInstanceContent(); } catch (error) { showNotice(String(error), "error"); } finally { install.disabled = false; } });
      row.append(title, detail, install); return row;
    }));
  } catch (error) { results.textContent = `No se pudo buscar: ${String(error)}`; }
}

async function recommendProfile() {
  try {
    const recommendation = await invoke<HardwareProfile>("recommend_hardware_profile");
    el("#hardware-summary").textContent = `${Math.round(recommendation.memoryMb / 1024)} GB RAM · ${recommendation.cpuThreads} hilos · ${recommendation.gpuName} · ${recommendation.onBattery ? "batería" : "corriente"}`;
    const memory = el<HTMLInputElement>("#memory"); memory.value = String(Math.round(recommendation.recommendedMemoryMb / 1024));
    el<HTMLOutputElement>("#memory-value").value = `${memory.value} GB`;
    localStorage.setItem("aureus.memory", memory.value);
    const targetFps = recommendation.onBattery ? 60 : recommendation.profile === "RAM_MINIMA" ? 90 : 144;
    await invoke("sync_client_config", { instanceId: selectedInstanceId, profile: recommendation.profile, renderDistance: recommendation.renderDistance, simulationDistance: recommendation.simulationDistance, targetFps });
    if (activeInstance) {
      activeInstance.memoryMb = recommendation.recommendedMemoryMb;
      activeInstance.profile = recommendation.profile;
      await invoke("upsert_managed_instance", { instance: activeInstance });
    }
    showNotice("Perfil recomendado guardado en el launcher y el mod.", "success");
  } catch (error) { showNotice(`No se pudo aplicar la recomendación: ${String(error)}`, "error"); }
}

async function refreshManagedInstances(preferred?: string) {
  const selector = el<HTMLSelectElement>("#managed-instance");
  const instances = await invoke<ManagedInstance[]>("managed_instances");
  selector.replaceChildren(...instances.map(instance => Object.assign(document.createElement("option"), { value:instance.id, textContent:`${instance.name} · ${instance.minecraftVersion}` })));
  const desired = preferred ?? selectedInstanceId;
  if (instances.some(instance => instance.id === desired)) selector.value = desired;
  else if (instances.some(instance => instance.id === selectedMinecraftVersion)) selector.value = selectedMinecraftVersion;
  selectedInstanceId = selector.value || selectedMinecraftVersion;
  activeInstance = instances.find(instance => instance.id === selectedInstanceId) ?? null;
  if (activeInstance) {
    el("#instance-name").textContent = activeInstance.name;
    el<HTMLInputElement>("#memory").value = String(activeInstance.memoryMb / 1024);
    el<HTMLOutputElement>("#memory-value").value = `${activeInstance.memoryMb / 1024} GB`;
  } else {
    el("#instance-name").textContent = `Minecraft ${selectedMinecraftVersion}`;
  }
}

function applyMinecraftSession(session: MinecraftSession) {
  minecraftSession = session;
  all<HTMLButtonElement>(".login-action").forEach(button => button.textContent = session.username);
  el(".account strong").textContent = session.username;
  el(".account small").textContent = "Minecraft Java";
  el(".avatar").textContent = session.username.slice(0, 1).toUpperCase();
  el("#account-name").textContent = session.username;
  el("#account-status-copy").textContent = "Cuenta con Minecraft Java verificada correctamente.";
  el("#microsoft-state").textContent = "Conectado";
  el("#license-state").textContent = "Minecraft Java verificado";
}

async function refreshAccounts() {
  const list = el("#account-list");
  try {
    const accounts = await invoke<StoredAccount[]>("list_minecraft_accounts");
    if (!accounts.length) { list.innerHTML = '<p class="muted-copy">No hay otras cuentas guardadas.</p>'; return; }
    list.replaceChildren(...accounts.map(account => {
      const row = document.createElement("div"); row.className = "content-entry";
      const name = document.createElement("strong"); name.textContent = account.username;
      const state = document.createElement("small"); state.textContent = account.active ? "ACTIVA" : account.uuid.slice(0, 8);
      const button = document.createElement("button"); button.className = "secondary"; button.textContent = account.active ? "Actual" : "Usar"; button.disabled = account.active;
      button.addEventListener("click", async () => { applyMinecraftSession(await invoke<MinecraftSession>("switch_minecraft_account", { uuid: account.uuid })); await refreshAccounts(); });
      row.append(name, state, button); return row;
    }));
  } catch (error) { console.warn("No se pudieron leer las cuentas", error); }
}

async function login() {
  showNotice("Abriendo el inicio de sesión seguro de Microsoft…");
  all<HTMLButtonElement>(".login-action").forEach(button => button.disabled = true);
  try {
    const start = await invoke<LoginStart>("begin_microsoft_login");
    await openUrl(start.authorizationUrl);
    await invoke("complete_microsoft_login");
    showNotice("Conectando Xbox y Minecraft…");
    const session = await invoke<MinecraftSession>("connect_minecraft");
    showNotice(`Minecraft conectado como ${session.username}.`, "success");
    applyMinecraftSession(session);
    await refreshAccounts();
  } catch (error) {
    showNotice(`No se pudo iniciar sesión: ${String(error)}`, "error");
  } finally {
    all<HTMLButtonElement>(".login-action").forEach(button => button.disabled = false);
  }
}

function updateProfile(profile: string) {
  const copy: Record<string, string> = {
    custom: "5 GB de RAM, 12 chunks visibles y 8 de simulación, conservando partículas y efectos reducidos.",
    competitive: "Prioriza latencia, 1% low y claridad PvP: partículas mínimas, sombras fuera y distancia estable.",
    memory_saver: "Reduce al mínimo seguro la distancia, efectos y memoria asignada para dejar más RAM libre al sistema.",
    battery: "Limita FPS en segundo plano, distancia y efectos para reducir consumo, temperatura y ruido del portátil.",
    max: "Reduce efectos y carga visual para priorizar la menor latencia y el máximo número de FPS.",
    balanced: "Mantiene buena visibilidad y estabilidad sin sacrificar demasiado rendimiento.",
    quality: "Conserva más detalle visual para equipos con margen de rendimiento.",
  };
  el("#profile-info").textContent = copy[profile] ?? copy.balanced;
  localStorage.setItem("aureus.profile", profile);
  const tuning: Record<string, [number, number, number]> = {
    competitive: [8, 6, 240], memory_saver: [6, 5, 90], battery: [6, 5, 60],
    max: [6, 5, 240], balanced: [10, 8, 144], quality: [16, 10, 120], custom: [12, 8, 144],
  };
  const [renderDistance, simulationDistance, targetFps] = tuning[profile] ?? tuning.balanced;
  void invoke("sync_client_config", { instanceId: selectedInstanceId, profile: profile.toUpperCase(), renderDistance, simulationDistance, targetFps });
  void invoke("apply_profile_content_policy", { instanceId: selectedInstanceId, profile }).catch(error =>
    showNotice(`No se pudo ajustar el contenido del perfil: ${String(error)}`, "error"));
  if (activeInstance) {
    activeInstance.profile = profile.toUpperCase();
    void invoke("upsert_managed_instance", { instance: activeInstance }).catch(error =>
      showNotice(`No se pudo guardar el perfil de la instancia: ${String(error)}`, "error"));
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  // Bind essential navigation before any native or storage operation. Optional
  // services may fail, but they must never leave the launcher unresponsive.
  all(".nav-item").forEach(button => button.addEventListener("click", () => switchView(button.dataset.view as ViewName)));
  all(".install-action").forEach(button => button.addEventListener("click", installMod));
  all(".play-action").forEach(button => button.addEventListener("click", requestMinecraftLaunch));
  el("#account-menu").addEventListener("click", () => switchView("account"));
  all<HTMLElement>("[data-open-view]").forEach(button => button.addEventListener("click", () => switchView(button.dataset.openView as ViewName)));
  const sidebarCollapsed = localStorage.getItem("aureus.sidebarCollapsed") === "true";
  document.body.classList.toggle("sidebar-collapsed", sidebarCollapsed);
  el("#sidebar-toggle").textContent = sidebarCollapsed ? "›" : "‹";
  el("#sidebar-toggle").addEventListener("click", () => {
    const collapsed = document.body.classList.toggle("sidebar-collapsed");
    el("#sidebar-toggle").textContent = collapsed ? "›" : "‹";
    localStorage.setItem("aureus.sidebarCollapsed", String(collapsed));
  });

  el("#refresh-content").addEventListener("click", showInstanceContent);
  el("#close-content").addEventListener("click", () => el("#content-manager").hidden = true);
  el("#search-mods").addEventListener("click", searchMods);
  el("#backup-instance").addEventListener("click", async () => {
    try { await invoke<string>("create_instance_backup", { instanceId: selectedInstanceId }); showNotice("Respaldo creado correctamente.", "success"); await showBackups(); }
    catch (error) { showNotice(`No se pudo respaldar: ${String(error)}`, "error"); }
  });
  el("#restore-instance").addEventListener("click", showBackups);
  el("#export-instance").addEventListener("click", async () => { try { showNotice(`Paquete exportado: ${await invoke<string>("export_instance", { instanceId: selectedInstanceId })}`, "success"); } catch (error) { showNotice(`No se pudo exportar: ${String(error)}`, "error"); } });
  el("#analyze-crash").addEventListener("click", async () => {
    const panel = el("#recovery-result"); panel.hidden = false;
    try {
      const report = await invoke<CrashAnalysis>("analyze_latest_crash", { instanceId: selectedInstanceId });
      const title = document.createElement("h3"); title.textContent = report.summary;
      const detail = document.createElement("p"); detail.textContent = report.detected ? (report.suspectedMods.join(" · ") || "No se identificó un mod concreto; usa Reparar para verificar archivos.") : "Juega normalmente; no hay ningún cierre que recuperar.";
      panel.replaceChildren(title, detail);
      if (report.detected) {
        const actions = document.createElement("div"); actions.className = "actions";
        const safe = document.createElement("button"); safe.className = "secondary"; safe.textContent = "Activar modo seguro";
        safe.addEventListener("click", async () => { showNotice(await invoke<string>("enable_safe_mode", { instanceId: selectedInstanceId }), "success"); });
        const rollback = document.createElement("button"); rollback.className = "secondary"; rollback.textContent = "Volver al último respaldo";
        rollback.addEventListener("click", async () => {
          const backups = await invoke<BackupEntry[]>("list_instance_backups", { instanceId: selectedInstanceId });
          if (!backups[0]) return showNotice("No existe un respaldo anterior.", "neutral");
          if (!window.confirm("Se restaurará el respaldo más reciente y se conservará el estado actual. ¿Continuar?")) return;
          showNotice(await invoke<string>("restore_instance_backup", { instanceId: selectedInstanceId, backupPath: backups[0].path }), "success");
        });
        actions.append(safe, rollback); panel.append(actions);
      }
    } catch (error) { panel.textContent = String(error); }
  });
  el("#verify-integrity").addEventListener("click", async () => {
    const panel = el("#recovery-result"); panel.hidden = false;
    try {
      const report = await invoke<IntegrityReport>("verify_instance_integrity", { instanceId: selectedInstanceId });
      const title = document.createElement("h3"); title.textContent = report.valid ? "Integridad verificada" : "La instancia necesita reparación";
      const detail = document.createElement("p");
      const extras = report.unmanagedFiles.length ? ` · ${report.unmanagedFiles.length} mods personales preservados` : "";
      detail.textContent = `${report.summary}${extras}`;
      panel.replaceChildren(title, detail);
      if (!report.valid) {
        const repair = document.createElement("button"); repair.className = "primary install-action"; repair.textContent = "Reparar archivos"; panel.append(repair);
        repair.addEventListener("click", () => all<HTMLButtonElement>(".install-action")[0]?.click());
      }
    } catch (error) { panel.textContent = `No se pudo verificar: ${String(error)}`; }
  });
  el("#recommend-profile").addEventListener("click", recommendProfile);
  el("#read-benchmark").addEventListener("click", async () => { try { const report = await invoke<BenchmarkReport>("read_benchmark", { instanceId: selectedInstanceId }); el("#benchmark-summary").textContent = report.available ? `${report.averageFps} FPS · 1% low ${report.onePercentLow} · ${report.memoryUsedMb} MB — ${report.recommendation}` : report.recommendation; } catch (error) { showNotice(String(error), "error"); } });
  el("#analyze-mod-impact").addEventListener("click", async () => {
    try { const report = await invoke<ModImpactReport>("analyze_mod_impact", { instanceId: selectedInstanceId });
      el("#mod-impact-summary").textContent = `${report.modCount} mods · ${report.totalSizeMb.toFixed(1)} MB · ${report.recommendation}`; }
    catch (error) { showNotice(`No se pudo analizar: ${String(error)}`, "error"); }
  });
  el("#save-baseline").addEventListener("click", async () => {
    try {
      const report = await invoke<BenchmarkReport>("read_benchmark", { instanceId: selectedInstanceId });
      if (!report.available) return showNotice(report.recommendation, "neutral");
      localStorage.setItem(`aureus.baseline.${selectedInstanceId}`, JSON.stringify(report));
      el("#benchmark-comparison").textContent = `Referencia: ${report.averageFps} FPS · 1% low ${report.onePercentLow} · ${report.memoryUsedMb} MB`;
      showNotice("Referencia A guardada. Cambia la configuración, juega y luego compara.", "success");
    } catch (error) { showNotice(String(error), "error"); }
  });
  el("#compare-benchmark").addEventListener("click", async () => {
    const stored = localStorage.getItem(`aureus.baseline.${selectedInstanceId}`);
    if (!stored) return showNotice("Primero guarda una referencia A.", "neutral");
    try {
      const before = JSON.parse(stored) as BenchmarkReport;
      const after = await invoke<BenchmarkReport>("read_benchmark", { instanceId: selectedInstanceId });
      if (!after.available) return showNotice(after.recommendation, "neutral");
      const fps = after.averageFps - before.averageFps;
      const low = after.onePercentLow - before.onePercentLow;
      const ram = after.memoryUsedMb - before.memoryUsedMb;
      el("#benchmark-comparison").textContent = `Resultado B: ${fps >= 0 ? "+" : ""}${fps} FPS · ${low >= 0 ? "+" : ""}${low} en 1% low · ${ram >= 0 ? "+" : ""}${ram} MB`;
    } catch (error) { showNotice(String(error), "error"); }
  });
  el("#apply-performance-mods").addEventListener("click", async () => {
    const projects = [el<HTMLInputElement>("#enable-bura").checked && "bura", el<HTMLInputElement>("#enable-rhenium").checked && "rhenium-mod"].filter(Boolean) as string[];
    if (!projects.length) return showNotice("No seleccionaste mods experimentales.", "neutral");
    const button = el<HTMLButtonElement>("#apply-performance-mods"); button.disabled = true;
    try {
      await invoke("create_instance_backup", { instanceId: selectedInstanceId });
      for (const projectId of projects) await invoke("install_modrinth", { instanceId: selectedInstanceId, version: selectedMinecraftVersion, projectId });
      showNotice(`${projects.length} optimizaciones experimentales instaladas. Se creó una selección reversible.`, "success");
      await showInstanceContent();
    } catch (error) { showNotice(`No se pudo aplicar: ${String(error)}`, "error"); }
    finally { button.disabled = false; }
  });
  el("#export-profile").addEventListener("click", async () => {
    try { showNotice(`Perfil exportado: ${await invoke<string>("export_profile", { instanceId: selectedInstanceId })}`, "success"); }
    catch (error) { showNotice(`No se pudo exportar el perfil: ${String(error)}`, "error"); }
  });
  el("#import-profile").addEventListener("click", async () => {
    const profilePath = window.prompt("Ruta del archivo .aureusprofile")?.trim(); if (!profilePath) return;
    try { await invoke<ManagedInstance>("import_profile", { instanceId: selectedInstanceId, profilePath }); showNotice("Perfil importado. El estado anterior quedó respaldado.", "success"); await refresh(); }
    catch (error) { showNotice(`No se pudo importar el perfil: ${String(error)}`, "error"); }
  });
  el<HTMLSelectElement>("#managed-instance").addEventListener("change", async event => {
    const selector = event.target as HTMLSelectElement;
    const id = selector.value;
    if (!id) return;
    const previousId = selectedInstanceId;
    selector.disabled = true;
    try {
      const version = await invoke<string>("select_managed_instance", { id });
      selectedInstanceId = id;
      selectedMinecraftVersion = version;
      await refreshManagedInstances(id);
      el<HTMLSelectElement>("#minecraft-version").value = version;
      all(".selected-version-label").forEach(label => label.textContent = version);
      await refresh();
    } catch (error) {
      selector.value = previousId;
      showNotice(`No se pudo seleccionar la instancia: ${String(error)}`, "error");
    } finally {
      selector.disabled = false;
    }
  });
  el("#duplicate-instance").addEventListener("click", async () => { const name = window.prompt("Nombre de la copia", `Copia de ${selectedInstanceId}`)?.trim(); if (!name) return; const id = `${selectedMinecraftVersion}-${Date.now()}`; try { await invoke("duplicate_managed_instance", { sourceId:selectedInstanceId, newId:id, newName:name }); await refreshManagedInstances(id); selectedInstanceId = id; await invoke("select_managed_instance", { id }); showNotice("Instancia duplicada y seleccionada.", "success"); } catch (error) { showNotice(String(error), "error"); } });
  el("#import-instance").addEventListener("click", async () => { const archivePath = window.prompt("Ruta del archivo .aureuspack")?.trim(); if (!archivePath) return; const id = `${selectedMinecraftVersion}-import-${Date.now()}`; try { await invoke("import_instance", { archivePath, newId:id, version:selectedMinecraftVersion }); await refreshManagedInstances(id); selectedInstanceId = id; await invoke("select_managed_instance", { id }); showNotice("Paquete importado y aislado correctamente.", "success"); } catch (error) { showNotice(String(error), "error"); } });
  el("#delete-instance").addEventListener("click", async () => { if (!selectedInstanceId || !window.confirm(`¿Eliminar ${selectedInstanceId} y sus archivos?`)) return; try { await invoke("delete_managed_instance", { id:selectedInstanceId, deleteFiles:true }); await invoke("select_version", { version:selectedMinecraftVersion }); selectedInstanceId = selectedMinecraftVersion; await refreshManagedInstances(); showNotice("Instancia eliminada.", "success"); } catch (error) { showNotice(String(error), "error"); } });
  el("#logout-account").addEventListener("click", async () => { try { showNotice(await invoke<string>("logout_minecraft_account"), "success"); minecraftSession = null; el(".account strong").textContent = "Sin sesión"; el(".account small").textContent = "Cuenta Microsoft"; el(".avatar").textContent = "?"; await refreshAccounts(); } catch (error) { showNotice(String(error), "error"); } });
  void refreshAccounts();
  void listen<LaunchProgress>("launch-progress", event => {
    const progress = event.payload;
    el("#launch-progress").hidden = false;
    el("#launch-stage").textContent = progress.stage;
    el("#launch-percent").textContent = `${progress.percent}%`;
    el("#launch-detail").textContent = progress.detail;
    el("#launch-progress-bar").style.width = `${progress.percent}%`;
    if (progress.percent === 100) {
      el<HTMLButtonElement>("#kill-game").hidden = false;
      el<HTMLButtonElement>("#cancel-launch").hidden = true;
      all<HTMLButtonElement>(".play-action").forEach(button => button.hidden = true);
    }
  }).catch(error => console.warn("No se pudo escuchar el progreso de inicio", error));
  window.setInterval(async () => {
    if (document.hidden) return;
    try {
      const runtime = await invoke<RuntimeStatus>("runtime_status");
      const telemetry = await invoke<SystemTelemetry>("system_telemetry", { pid: runtime.pid });
      el("#telemetry-summary").textContent = runtime.running
        ? `${telemetry.processMemoryMb} MB · CPU ${telemetry.processCpuPercent.toFixed(1)}% · ${telemetry.thermalState}`
        : `${telemetry.gpuName} · ${telemetry.onBattery ? "Batería" : "Corriente"}`;
    } catch { /* Optional telemetry must not affect launching. */ }
  }, 3000);
  void listen<string>("game-state", event => {
    const successful = event.payload.includes("correctamente");
    showToast(event.payload, successful ? "success" : "error");
    el("#launch-progress").hidden = true;
    el("#launch-progress-bar").style.width = "0%";
    el("#launch-percent").textContent = "0%";
    el("#launch-stage").textContent = "Preparando";
    el("#launch-detail").textContent = "Comprobando componentes…";
    el<HTMLButtonElement>("#kill-game").hidden = true;
    el<HTMLButtonElement>("#kill-game").disabled = false;
    all<HTMLButtonElement>(".play-action").forEach(button => button.disabled = false);
    all<HTMLButtonElement>(".play-action").forEach(button => button.hidden = false);
    void refreshRuntime();
  }).catch(error => console.warn("No se pudo escuchar el estado del juego", error));
  try {
    const restored = await invoke<MinecraftSession | null>("current_minecraft_session");
    if (restored) applyMinecraftSession(restored);
  } catch (error) {
    console.warn("No se pudo restaurar la sesión", error);
  }
  el("#confirm-launch").addEventListener("click", playMinecraft);
  el("#configure-before-launch").addEventListener("click", () => {
    el<HTMLDialogElement>("#launch-confirmation").close();
    switchView("performance");
  });
  el("#cancel-launch").addEventListener("click", cancelLaunch);
  el("#kill-game").addEventListener("click", killMinecraft);
  await Promise.allSettled([refreshRuntime(), loadVersionCatalog(), refreshManagedInstances()]);
  window.setInterval(refreshRuntime, 1000);
  el("#run-diagnostics").addEventListener("click", async () => {
    showNotice("Recopilando diagnóstico local…");
    try {
      const report = await invoke<Diagnostics>("collect_diagnostics");
      const output = el<HTMLPreElement>("#diagnostics-output");
      output.textContent = [
        `${report.operatingSystem} · ${report.architecture}`,
        report.javaVersion,
        `Fabric: ${report.fabricInstalled ? "OK" : "FALTA"} · API: ${report.fabricApiInstalled ? "OK" : "FALTA"} · Aureus: ${report.aureusInstalled ? "OK" : "FALTA"}`,
        `Directorio: ${report.minecraftDirectory}`,
        "", "Último registro:", report.latestLogTail,
      ].join("\n");
      output.hidden = false;
      showNotice("Diagnóstico completado.", "success");
    } catch (error) {
      showNotice(`Falló el diagnóstico: ${String(error)}`, "error");
    }
  });
  el("#clean-caches").addEventListener("click", async () => {
    try { showNotice(await invoke<string>("clean_safe_caches", { instanceId: selectedInstanceId }), "success"); }
    catch (error) { showNotice(`No se pudo limpiar: ${String(error)}`, "error"); }
  });

  try {
    displayBackground(await loadAsset("home-background"));
    const savedSkin = await loadAsset("local-skin");
    if (savedSkin) displaySkin(savedSkin, localStorage.getItem("aureus.skinName") ?? "Skin importada");
  } catch (error) {
    console.warn("No se pudieron restaurar los recursos visuales locales", error);
  }

  el<HTMLInputElement>("#background-file").addEventListener("change", async event => {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) return showNotice("Selecciona una imagen o GIF válido.", "error");
    if (file.size > 20 * 1024 * 1024) return showNotice("El archivo no puede superar 20 MB.", "error");
    try {
      await saveAsset("home-background", file);
      displayBackground(file);
      showNotice("Portada personalizada guardada.", "success");
    } catch (error) {
      showNotice(`No se pudo guardar la portada: ${String(error)}`, "error");
    }
  });
  el("#remove-background").addEventListener("click", async () => {
    try {
      await removeAsset("home-background");
      displayBackground(null);
      showNotice("Portada personalizada eliminada.", "success");
    } catch (error) {
      showNotice(`No se pudo quitar la portada: ${String(error)}`, "error");
    }
  });
  el<HTMLInputElement>("#skin-file").addEventListener("change", async event => {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.type !== "image/png") return showNotice("Las skins deben estar en formato PNG.", "error");
    try {
      await saveAsset("local-skin", file);
      localStorage.setItem("aureus.skinName", file.name);
      displaySkin(file, file.name);
      showNotice("Skin guardada en la biblioteca local.", "success");
    } catch (error) {
      showNotice(`No se pudo guardar la skin: ${String(error)}`, "error");
    }
  });
  all(".login-action").forEach(button => button.addEventListener("click", login));

  const profile = el<HTMLSelectElement>("#profile");
  const requestedPreset = "5gb-12-8-v1";
  if (localStorage.getItem("aureus.requestedPreset") !== requestedPreset) {
    localStorage.setItem("aureus.profile", "custom");
    localStorage.setItem("aureus.memory", "5");
    localStorage.setItem("aureus.requestedPreset", requestedPreset);
  }
  const storedProfile = localStorage.getItem("aureus.profile");
  profile.value = storedProfile === "memory_saver" ? "custom" : (storedProfile ?? "custom");
  updateProfile(profile.value);
  profile.addEventListener("change", event => updateProfile((event.target as HTMLSelectElement).value));

  const memory = el<HTMLInputElement>("#memory");
  const storedMemory = localStorage.getItem("aureus.memory");
  memory.value = storedMemory === "3" ? "5" : (storedMemory ?? "5");
  el<HTMLOutputElement>("#memory-value").value = `${memory.value} GB`;
  memory.addEventListener("input", event => {
    const value = (event.target as HTMLInputElement).value;
    el<HTMLOutputElement>("#memory-value").value = `${value} GB`;
    localStorage.setItem("aureus.memory", value);
    if (activeInstance) {
      activeInstance.memoryMb = Number(value) * 1024;
      void invoke("upsert_managed_instance", { instance: activeInstance }).catch(error =>
        showNotice(`No se pudo guardar la memoria: ${String(error)}`, "error"));
    }
  });

  const updateCheck = el<HTMLInputElement>("#update-check");
  updateCheck.checked = localStorage.getItem("aureus.updateCheck") !== "false";
  updateCheck.addEventListener("change", () => localStorage.setItem("aureus.updateCheck", String(updateCheck.checked)));
  el("#check-update-now").addEventListener("click", () => {
    if (pendingUpdate) void installPendingUpdate();
    else void checkForAureusUpdate(true);
  });
  el("#global-update").addEventListener("click", () => {
    if (pendingUpdate) void installPendingUpdate();
    else void checkForAureusUpdate(true);
  });
  if (updateCheck.checked) void checkForAureusUpdate();

  const compact = el<HTMLInputElement>("#compact-mode");
  compact.checked = localStorage.getItem("aureus.compact") === "true";
  document.body.classList.toggle("compact", compact.checked);
  compact.addEventListener("change", () => {
    document.body.classList.toggle("compact", compact.checked);
    localStorage.setItem("aureus.compact", String(compact.checked));
  });

  const savedView = (localStorage.getItem("aureus.view") as ViewName | null) ?? "home";
  switchView(pageCopy[savedView] ? savedView : "home");
  await refresh();
});
