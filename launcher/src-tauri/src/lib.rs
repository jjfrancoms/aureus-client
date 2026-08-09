use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distr::Alphanumeric, Rng};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, State};
pub mod professional;
pub mod version_manager;

const CLIENT_ID: &str = "4e82ebb8-bdf1-48f8-a1c2-8a62cd1be7a8";
const MINECRAFT_VERSION: &str = "1.21.11";
const MOD_FILE_NAME: &str = "aureus-client-0.3.0-minecraft-1.21.11.jar";
const MOD_BYTES: &[u8] =
    include_bytes!("../../../outputs/aureus-client-0.3.0-minecraft-1.21.11.jar");
const FABRIC_API_FILE_NAME: &str = "fabric-api-0.141.6+1.21.11.jar";
const FABRIC_API_BYTES: &[u8] = include_bytes!("../../../outputs/fabric-api-0.141.6+1.21.11.jar");
const FABRIC_INSTALLER_BYTES: &[u8] = include_bytes!("../../../outputs/fabric-installer-1.1.2.jar");
const FABRIC_PROFILE: &str = "fabric-loader-0.19.3-1.21.11";

#[cfg(target_os = "windows")]
fn hidden_windows_command(program: &str) -> std::process::Command {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

struct PendingLogin {
    receiver: mpsc::Receiver<Result<String, String>>,
    verifier: String,
    redirect_uri: String,
}

struct AuthState {
    pending: Mutex<Option<PendingLogin>>,
    access_token: Mutex<Option<String>>,
    minecraft_session: Mutex<Option<MinecraftSession>>,
}

#[derive(Default)]
struct RuntimeState {
    cancel_requested: AtomicBool,
    game_pid: Mutex<Option<u32>>,
    started_at: Mutex<Option<u64>>,
}

const PERFORMANCE_MODS: &[(&str, &str)] = &[
    ("sodium-fabric-0.8.14-beta.2+mc1.21.11.jar", "https://cdn.modrinth.com/data/AANobbMI/versions/vqUoGREs/sodium-fabric-0.8.14-beta.2%2Bmc1.21.11.jar"),
    ("lithium-fabric-0.21.4+mc1.21.11.jar", "https://cdn.modrinth.com/data/gvQqBUqZ/versions/Ow7wA0kG/lithium-fabric-0.21.4%2Bmc1.21.11.jar"),
    ("ImmediatelyFast-Fabric-1.14.3+1.21.11.jar", "https://cdn.modrinth.com/data/5ZwdcRci/versions/4EwhsTu7/ImmediatelyFast-Fabric-1.14.3%2B1.21.11.jar"),
    ("ferritecore-8.2.0-fabric.jar", "https://cdn.modrinth.com/data/uXXizFIs/versions/Ii0gP3D8/ferritecore-8.2.0-fabric.jar"),
    ("entityculling-fabric-1.10.5-mc1.21.11.jar", "https://cdn.modrinth.com/data/NNAgCjsB/versions/sP0vNbeN/entityculling-fabric-1.10.5-mc1.21.11.jar"),
    ("moreculling-fabric-1.21.11-1.6.2.jar", "https://cdn.modrinth.com/data/51shyZVL/versions/wOzykoLV/moreculling-fabric-1.21.11-1.6.2.jar"),
    ("cloth-config-21.11.153-fabric.jar", "https://cdn.modrinth.com/data/9s6osm5g/versions/xuX40TN5/cloth-config-21.11.153-fabric.jar"),
    ("krypton-0.2.10.jar", "https://cdn.modrinth.com/data/fQEb0iXm/versions/O9LmWYR7/krypton-0.2.10.jar"),
    ("BadOptimizations-2.4.1-1.21.11.jar", "https://cdn.modrinth.com/data/g96Z4WVZ/versions/Q3Dusz2j/BadOptimizations-2.4.1-1.21.11.jar"),
    ("dynamic-fps-3.11.6+minecraft-1.21.11-fabric.jar", "https://cdn.modrinth.com/data/LQ3K71Q1/versions/Fab7e5Th/dynamic-fps-3.11.6%2Bminecraft-1.21.11-fabric.jar"),
    ("sodium-extra-fabric-0.9.3+mc1.21.11.jar", "https://cdn.modrinth.com/data/PtjYWJkn/versions/taHK5pw1/sodium-extra-fabric-0.9.3%2Bmc1.21.11.jar"),
    ("reeses-sodium-options-fabric-2.2.3+mc1.21.11.jar", "https://cdn.modrinth.com/data/Bh37bMuy/versions/P0MH4cn0/reeses-sodium-options-fabric-2.2.3%2Bmc1.21.11.jar"),
    ("fabric-language-kotlin-1.13.13+kotlin.2.4.10.jar", "https://cdn.modrinth.com/data/Ha28R6CL/versions/bdhiINYC/fabric-language-kotlin-1.13.13%2Bkotlin.2.4.10.jar"),
    ("yet_another_config_lib_v3-3.8.2+1.21.11-fabric.jar", "https://cdn.modrinth.com/data/1eAoo2KR/versions/pHWDw3Vc/yet_another_config_lib_v3-3.8.2%2B1.21.11-fabric.jar"),
    ("zoomify-2.15.2+1.21.11.jar", "https://cdn.modrinth.com/data/w7ThoJFB/versions/gI5KZI8V/zoomify-2.15.2%2B1.21.11.jar"),
    ("placeholder-api-2.8.2+1.21.10.jar", "https://cdn.modrinth.com/data/eXts2L7r/versions/qxjzQ9xY/placeholder-api-2.8.2%2B1.21.10.jar"),
    ("modmenu-17.0.1-beta.1.jar", "https://cdn.modrinth.com/data/mOgUt4GM/versions/j2vTurvl/modmenu-17.0.1-beta.1.jar"),
    ("BetterF3-17.0.0-Fabric-1.21.11.jar", "https://cdn.modrinth.com/data/8shC1gFX/versions/Qw1nhj7u/BetterF3-17.0.0-Fabric-1.21.11.jar"),
    ("appleskin-fabric-mc1.21.11-3.0.8.jar", "https://cdn.modrinth.com/data/EsAfCjCV/versions/59ti1rvg/appleskin-fabric-mc1.21.11-3.0.8.jar"),
    ("lambdynamiclights-4.9.1+1.21.11.jar", "https://cdn.modrinth.com/data/yBW8D80W/versions/5Tp7kdU0/lambdynamiclights-4.9.1%2B1.21.11.jar"),
    ("capes-1.5.10+1.21.11-fabric.jar", "https://cdn.modrinth.com/data/89Wsn8GD/versions/GAQAG80Q/capes-1.5.10%2B1.21.11-fabric.jar"),
    ("fast-ip-ping-v1.0.11-mc1.21.11-fabric.jar", "https://cdn.modrinth.com/data/9mtu0sUO/versions/E3Ei5xUe/fast-ip-ping-v1.0.11-mc1.21.11-fabric.jar"),
    ("HudFabric-1.0.3-1.21.11.jar", "https://cdn.modrinth.com/data/jNODUcnv/versions/5G3wTf4Q/HudFabric-1.0.3-1.21.11.jar"),
    ("shulkerboxtooltip-fabric-5.2.16+1.21.11.jar", "https://cdn.modrinth.com/data/2M01OLQq/versions/rZovgkWT/shulkerboxtooltip-fabric-5.2.16%2B1.21.11.jar"),
    ("status-effect-bars-1.0.10.jar", "https://cdn.modrinth.com/data/x02cBj9Y/versions/iY0FQLmu/status-effect-bars-1.0.10.jar"),
    ("InventoryProfilesNext-fabric-1.21.11-2.2.6.jar", "https://cdn.modrinth.com/data/O7RBXm3n/versions/YKjWPbto/InventoryProfilesNext-fabric-1.21.11-2.2.6.jar"),
    ("libIPN-fabric-1.21.11-6.6.3.jar", "https://cdn.modrinth.com/data/onSQdWhM/versions/ByG214OZ/libIPN-fabric-1.21.11-6.6.3.jar"),
    ("MouseTweaks-fabric-mc1.21.11-2.30.jar", "https://cdn.modrinth.com/data/aC3cM3Vq/versions/i1duwnJl/MouseTweaks-fabric-mc1.21.11-2.30.jar"),
    ("xaerominimap-fabric-1.21.11-26.4.2.jar", "https://cdn.modrinth.com/data/1bokaNcj/versions/8MdqDp18/xaerominimap-fabric-1.21.11-26.4.2.jar"),
    ("bettermounthud-1.2.6.jar", "https://cdn.modrinth.com/data/kqJFAPU9/versions/rXZxHSEZ/bettermounthud-1.2.6.jar"),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    running: bool,
    pid: Option<u32>,
    started_at: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftVersionEntry {
    id: String,
    version_type: String,
    release_time: String,
    installed: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedVersionStatus {
    version: String,
    prepared: bool,
    mode: String,
    mod_count: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceDescriptor {
    version: String,
    version_json: PathBuf,
    client_jar: PathBuf,
    asset_index: String,
    game_directory: PathBuf,
    state: String,
    loader_version: Option<String>,
    java_path: Option<PathBuf>,
}

#[derive(Deserialize)]
struct MojangManifest {
    versions: Vec<MojangVersion>,
}

#[derive(Deserialize)]
struct MojangVersion {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

#[derive(Serialize, Deserialize)]
struct StoredSession {
    username: String,
    uuid: String,
    access_token: String,
    expires_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAccountSummary {
    username: String,
    uuid: String,
    active: bool,
}

fn session_keyring() -> Result<keyring::Entry, String> {
    keyring::Entry::new("Aureus Launcher", "minecraft-session").map_err(|error| error.to_string())
}

fn load_stored_session() -> Option<MinecraftSession> {
    let secret = session_keyring().ok()?.get_password().ok()?;
    let stored: StoredSession = serde_json::from_str(&secret).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if stored.expires_at <= now {
        return None;
    }
    Some(MinecraftSession {
        username: stored.username,
        uuid: stored.uuid,
        _access_token: stored.access_token,
    })
}

fn save_stored_session(session: &MinecraftSession, expires_in: u64) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let stored = StoredSession {
        username: session.username.clone(),
        uuid: session.uuid.clone(),
        access_token: session._access_token.clone(),
        expires_at: now + expires_in.saturating_sub(60),
    };
    let secret = serde_json::to_string(&stored).map_err(|error| error.to_string())?;
    session_keyring()?
        .set_password(&secret)
        .map_err(|error| error.to_string())?;
    keyring::Entry::new("Aureus Launcher Account", &session.uuid)
        .map_err(|error| error.to_string())?
        .set_password(&secret)
        .map_err(|error| error.to_string())?;
    let path = aureus_data_directory()?.join("accounts.json");
    let mut accounts: Vec<StoredAccountSummary> = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    for account in &mut accounts {
        account.active = false;
    }
    if let Some(account) = accounts
        .iter_mut()
        .find(|account| account.uuid == session.uuid)
    {
        account.username = session.username.clone();
        account.active = true;
    } else {
        accounts.push(StoredAccountSummary {
            username: session.username.clone(),
            uuid: session.uuid.clone(),
            active: true,
        });
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&accounts).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_minecraft_accounts() -> Result<Vec<StoredAccountSummary>, String> {
    let path = aureus_data_directory()?.join("accounts.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn switch_minecraft_account(
    uuid: String,
    auth: State<'_, AuthState>,
) -> Result<MinecraftSession, String> {
    let secret = keyring::Entry::new("Aureus Launcher Account", &uuid)
        .map_err(|error| error.to_string())?
        .get_password()
        .map_err(|_| "La sesión guardada ya no está disponible")?;
    let stored: StoredSession = serde_json::from_str(&secret).map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    if stored.expires_at <= now {
        return Err("La sesión expiró; vuelve a iniciar con Microsoft".into());
    }
    let session = MinecraftSession {
        username: stored.username,
        uuid: stored.uuid,
        _access_token: stored.access_token,
    };
    save_stored_session(&session, stored.expires_at.saturating_sub(now))?;
    *auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")? = Some(session.clone());
    Ok(session)
}

#[tauri::command]
fn logout_minecraft_account(auth: State<'_, AuthState>) -> Result<String, String> {
    *auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")? = None;
    let _ = session_keyring()?.delete_credential();
    let path = aureus_data_directory()?.join("accounts.json");
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(mut accounts) = serde_json::from_slice::<Vec<StoredAccountSummary>>(&bytes) {
            for account in &mut accounts {
                account.active = false;
            }
            fs::write(
                path,
                serde_json::to_vec_pretty(&accounts).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok("Sesión cerrada; las otras cuentas permanecen guardadas de forma segura".into())
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
            access_token: Mutex::new(None),
            minecraft_session: Mutex::new(load_stored_session()),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftSession {
    username: String,
    uuid: String,
    #[serde(skip_serializing)]
    _access_token: String,
}

#[derive(Deserialize)]
struct MinecraftTokenResponse {
    access_token: String,
    #[serde(default = "default_minecraft_token_lifetime")]
    expires_in: u64,
}

fn default_minecraft_token_lifetime() -> u64 {
    86_400
}

#[derive(Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginStart {
    authorization_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResult {
    authenticated: bool,
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherStatus {
    client_id: &'static str,
    minecraft_version: &'static str,
    minecraft_directory: Option<String>,
    mod_installed: bool,
    java_available: bool,
    fabric_installed: bool,
    fabric_api_installed: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameInstance {
    id: String,
    name: String,
    minecraft_version: String,
    loader_version: String,
    memory_mb: u32,
    jvm_args: Vec<String>,
    game_directory: String,
    performance_profile: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsReport {
    operating_system: String,
    architecture: String,
    java_version: String,
    minecraft_directory: String,
    fabric_installed: bool,
    fabric_api_installed: bool,
    aureus_installed: bool,
    latest_log_tail: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchProgress {
    percent: u8,
    stage: &'static str,
    detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateStatus {
    current_version: String,
    latest_version: String,
    available: bool,
    release_url: String,
    signed_install_required: bool,
}

#[tauri::command]
async fn check_launcher_update() -> Result<UpdateStatus, String> {
    let response: serde_json::Value = reqwest::Client::builder()
        .user_agent("Aureus-Launcher/0.3.0")
        .build()
        .map_err(|e| e.to_string())?
        .get("https://api.github.com/repos/jjfrancoms/aureus-client/releases/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let latest = response
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("0.3.0")
        .trim_start_matches('v')
        .to_string();
    let current = env!("CARGO_PKG_VERSION").to_string();
    Ok(UpdateStatus {
        available: latest != current,
        current_version: current,
        latest_version: latest,
        release_url: response
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/jjfrancoms/aureus-client/releases")
            .into(),
        signed_install_required: true,
    })
}

fn emit_launch(
    app: &tauri::AppHandle,
    percent: u8,
    stage: &'static str,
    detail: impl Into<String>,
) {
    let _ = app.emit(
        "launch-progress",
        LaunchProgress {
            percent,
            stage,
            detail: detail.into(),
        },
    );
}

fn ensure_not_cancelled(runtime: &RuntimeState) -> Result<(), String> {
    if runtime.cancel_requested.load(Ordering::SeqCst) {
        Err("Inicio cancelado por el usuario".into())
    } else {
        Ok(())
    }
}

fn aureus_data_directory() -> Result<PathBuf, String> {
    let base = dirs::config_dir().ok_or("No se pudo localizar la carpeta de configuración")?;
    let path = base.join("Aureus Launcher");
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn instances_path() -> Result<PathBuf, String> {
    Ok(aureus_data_directory()?.join("instances.json"))
}

fn selected_version_path() -> Result<PathBuf, String> {
    Ok(aureus_data_directory()?.join("selected-version.txt"))
}
fn selected_instance_path() -> Result<PathBuf, String> {
    Ok(aureus_data_directory()?.join("selected-instance.txt"))
}

fn read_selected_version() -> String {
    selected_version_path()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| MINECRAFT_VERSION.into())
}
fn read_selected_instance() -> String {
    selected_instance_path()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(read_selected_version)
}

#[tauri::command]
async fn version_catalog() -> Result<Vec<MinecraftVersionEntry>, String> {
    let manifest: MojangManifest = reqwest::Client::builder()
        .user_agent("Aureus-Launcher/0.3.0")
        .build()
        .map_err(|error| error.to_string())?
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let minecraft = minecraft_directory();
    Ok(manifest
        .versions
        .into_iter()
        .map(|version| {
            let installed = minecraft
                .as_ref()
                .map(|root| {
                    root.join("versions")
                        .join(&version.id)
                        .join(format!("{}.json", version.id))
                        .exists()
                })
                .unwrap_or(false);
            MinecraftVersionEntry {
                id: version.id,
                version_type: version.version_type,
                release_time: version.release_time,
                installed,
            }
        })
        .collect())
}

#[tauri::command]
fn selected_version() -> String {
    read_selected_version()
}

#[tauri::command]
fn selected_version_status() -> Result<SelectedVersionStatus, String> {
    let version = read_selected_version();
    let path = aureus_data_directory()?
        .join("instances-data")
        .join(read_selected_instance())
        .join("aureus-instance.json");
    if !path.exists() {
        return Ok(SelectedVersionStatus {
            version,
            prepared: false,
            mode: "pending".into(),
            mod_count: 0,
        });
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    Ok(SelectedVersionStatus {
        version,
        prepared: true,
        mode: if value["state"] == "fabric-ready" {
            "fabric"
        } else {
            "vanilla"
        }
        .into(),
        mod_count: value["modCount"].as_u64().unwrap_or(0) as usize,
    })
}

#[tauri::command]
fn select_version(version: String) -> Result<String, String> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
    {
        return Err("Versión de Minecraft no válida".into());
    }
    fs::write(selected_version_path()?, &version).map_err(|error| error.to_string())?;
    fs::write(selected_instance_path()?, &version).map_err(|error| error.to_string())?;
    Ok(version)
}

#[tauri::command]
fn select_managed_instance(id: String) -> Result<String, String> {
    let instance = professional::managed_instances()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("No existe esa instancia")?;
    fs::write(selected_instance_path()?, &instance.id).map_err(|error| error.to_string())?;
    fs::write(selected_version_path()?, &instance.minecraft_version)
        .map_err(|error| error.to_string())?;
    Ok(instance.minecraft_version)
}

#[tauri::command]
async fn install_selected_version(
    app: tauri::AppHandle,
    runtime: State<'_, RuntimeState>,
) -> Result<String, String> {
    runtime.cancel_requested.store(false, Ordering::SeqCst);
    let version = read_selected_version();
    emit_launch(
        &app,
        4,
        "Catálogo",
        format!("Verificando Minecraft {version}"),
    );
    let minecraft =
        minecraft_directory().ok_or("No se pudo localizar la carpeta compartida de Minecraft")?;
    let client = reqwest::Client::builder()
        .user_agent("Aureus-Launcher/0.3.0")
        .build()
        .map_err(|error| error.to_string())?;
    emit_launch(
        &app,
        12,
        "Minecraft",
        "Descargando cliente, librerías y recursos oficiales",
    );
    let installed =
        version_manager::install_official_version(&version, &minecraft, &client).await?;
    ensure_not_cancelled(&runtime)?;
    let instance_dir = aureus_data_directory()?
        .join("instances-data")
        .join(&version);
    fs::write(selected_instance_path()?, &version).map_err(|error| error.to_string())?;
    fs::create_dir_all(instance_dir.join("mods")).map_err(|error| error.to_string())?;
    fs::create_dir_all(instance_dir.join("resourcepacks")).map_err(|error| error.to_string())?;
    emit_launch(
        &app,
        55,
        "Minecraft",
        format!(
            "{} archivos descargados y verificados",
            installed.downloaded_files
        ),
    );
    emit_launch(
        &app,
        62,
        "Instancia",
        "Minecraft Vanilla está listo; comprobando compatibilidad con Fabric",
    );
    let loader = version_manager::compatible_fabric_loader(&version, &client).await?;
    let launch_meta = version_manager::read_launch_metadata(&installed.version_json)?;
    emit_launch(
        &app,
        66,
        "Java",
        format!("Preparando Java {}", launch_meta.java_major),
    );
    let java_path = version_manager::ensure_java_runtime(
        launch_meta.java_major,
        &aureus_data_directory()?.join("runtimes"),
        &client,
    )
    .await?;
    ensure_not_cancelled(&runtime)?;
    let mut state = "vanilla-ready";
    let mut mod_count = 0;
    let mut unavailable: Vec<String> = Vec::new();
    if let Some(loader_version) = &loader {
        emit_launch(
            &app,
            70,
            "Fabric",
            format!("Instalando Fabric Loader {loader_version}"),
        );
        let installer = std::env::temp_dir().join("aureus-fabric-installer-general.jar");
        fs::write(&installer, FABRIC_INSTALLER_BYTES).map_err(|error| error.to_string())?;
        let output = std::process::Command::new(&java_path)
            .arg("-jar")
            .arg(&installer)
            .arg("client")
            .arg("-dir")
            .arg(&minecraft)
            .arg("-mcversion")
            .arg(&version)
            .arg("-loader")
            .arg(loader_version)
            .output()
            .map_err(|error| format!("No se pudo ejecutar Fabric Installer: {error}"))?;
        let _ = fs::remove_file(installer);
        if output.status.success() {
            ensure_not_cancelled(&runtime)?;
            emit_launch(
                &app,
                78,
                "Mods",
                "Buscando versiones compatibles y dependencias",
            );
            let resolved = version_manager::resolve_modrinth_mods(
                &version,
                &instance_dir.join("mods"),
                &client,
            )
            .await?;
            mod_count = resolved.0;
            unavailable = resolved.1;
            state = "fabric-ready";
            if version == MINECRAFT_VERSION {
                fs::write(instance_dir.join("mods").join(MOD_FILE_NAME), MOD_BYTES)
                    .map_err(|error| error.to_string())?;
                mod_count += 1;
                let source_pack = minecraft
                    .join("resourcepacks")
                    .join("The Better Default Pack");
                let destination_pack = instance_dir
                    .join("resourcepacks")
                    .join("The Better Default Pack");
                if source_pack.exists() {
                    copy_directory(&source_pack, &destination_pack)?;
                    fs::write(instance_dir.join("options.txt"), b"resourcePacks:[\"file/The Better Default Pack\"]\nrenderDistance:8\nsimulationDistance:12\n").map_err(|error| error.to_string())?;
                }
            }
        }
    }
    let descriptor = serde_json::json!({
        "version": version, "versionJson": installed.version_json, "clientJar": installed.client_jar,
        "assetIndex": installed.asset_index_id, "gameDirectory": instance_dir,
        "state": state, "loaderVersion": loader, "javaPath": java_path, "modCount": mod_count, "unavailableMods": unavailable
    });
    fs::write(
        instance_dir.join("aureus-instance.json"),
        serde_json::to_vec_pretty(&descriptor).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    professional::register_prepared_instance(
        &version,
        &version,
        Some(java_path.display().to_string()),
    )?;
    emit_launch(
        &app,
        100,
        "Lista",
        if state == "fabric-ready" {
            format!("Minecraft {version} · Fabric · {mod_count} mods")
        } else {
            format!("Minecraft {version} · Vanilla")
        },
    );
    Ok(if state == "fabric-ready" {
        format!("Minecraft {version} preparado con Fabric y {mod_count} mods compatibles")
    } else {
        format!("Minecraft {version} preparado en modo Vanilla; Fabric no está disponible")
    })
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_directory(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn default_instance() -> Result<GameInstance, String> {
    let game_directory = minecraft_directory().ok_or("No se pudo localizar Minecraft")?;
    Ok(GameInstance {
        id: "aureus-1-21-11".into(),
        name: "Aureus 1.21.11".into(),
        minecraft_version: MINECRAFT_VERSION.into(),
        loader_version: "0.19.3".into(),
        memory_mb: 5120,
        jvm_args: vec!["-XX:+UseG1GC".into(), "-XX:+ParallelRefProcEnabled".into()],
        game_directory: game_directory.display().to_string(),
        performance_profile: "CUSTOM".into(),
    })
}

fn read_instances() -> Result<Vec<GameInstance>, String> {
    let path = instances_path()?;
    if !path.exists() {
        let instances = vec![default_instance()?];
        write_instances(&instances)?;
        return Ok(instances);
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let instances: Vec<GameInstance> =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if instances.is_empty() {
        Ok(vec![default_instance()?])
    } else {
        Ok(instances)
    }
}

fn write_instances(instances: &[GameInstance]) -> Result<(), String> {
    let path = instances_path()?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(instances).map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_instances() -> Result<Vec<GameInstance>, String> {
    read_instances()
}

#[tauri::command]
fn save_instance(instance: GameInstance) -> Result<GameInstance, String> {
    if instance.name.trim().is_empty() {
        return Err("La instancia necesita un nombre".into());
    }
    if !(2048..=16384).contains(&instance.memory_mb) {
        return Err("La memoria debe estar entre 2 y 16 GB".into());
    }
    if instance.minecraft_version != MINECRAFT_VERSION {
        return Err("Esta versión de Aureus solo admite Minecraft 1.21.11".into());
    }
    let mut instances = read_instances()?;
    if let Some(existing) = instances.iter_mut().find(|item| item.id == instance.id) {
        *existing = instance.clone();
    } else {
        instances.push(instance.clone());
    }
    write_instances(&instances)?;
    Ok(instance)
}

#[tauri::command]
fn collect_diagnostics() -> Result<DiagnosticsReport, String> {
    let minecraft = minecraft_directory().ok_or("No se pudo localizar Minecraft")?;
    let java_version = std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or("Java detectado")
                .to_string()
        })
        .unwrap_or_else(|_| "Java no detectado".into());
    let latest_log = minecraft.join("logs").join("latest.log");
    let latest_log_tail = fs::read_to_string(latest_log)
        .ok()
        .map(|text| {
            let lines: Vec<_> = text.lines().rev().take(120).collect();
            lines.into_iter().rev().collect::<Vec<_>>().join("\n")
        })
        .unwrap_or_else(|| "No existe un registro reciente".into());
    Ok(DiagnosticsReport {
        operating_system: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        java_version,
        minecraft_directory: minecraft.display().to_string(),
        fabric_installed: minecraft
            .join("versions")
            .join(FABRIC_PROFILE)
            .join(format!("{FABRIC_PROFILE}.json"))
            .exists(),
        fabric_api_installed: minecraft.join("mods").join(FABRIC_API_FILE_NAME).exists(),
        aureus_installed: minecraft.join("mods").join(MOD_FILE_NAME).exists(),
        latest_log_tail,
    })
}

fn minecraft_directory() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    #[cfg(target_os = "macos")]
    return Some(home.join("Library/Application Support/minecraft"));
    #[cfg(target_os = "windows")]
    return Some(home.join("AppData/Roaming/.minecraft"));
    #[cfg(target_os = "linux")]
    return Some(home.join(".minecraft"));
}

#[tauri::command]
fn launcher_status() -> LauncherStatus {
    let minecraft = minecraft_directory();
    let mod_installed = minecraft
        .as_ref()
        .map(|path| path.join("mods").join(MOD_FILE_NAME).exists())
        .unwrap_or(false);
    let java_available = std::process::Command::new("java")
        .arg("-version")
        .output()
        .is_ok();
    let fabric_installed = minecraft
        .as_ref()
        .map(|path| {
            path.join("versions")
                .join(FABRIC_PROFILE)
                .join(format!("{FABRIC_PROFILE}.json"))
                .exists()
        })
        .unwrap_or(false);
    let fabric_api_installed = minecraft
        .as_ref()
        .map(|path| path.join("mods").join(FABRIC_API_FILE_NAME).exists())
        .unwrap_or(false);

    LauncherStatus {
        client_id: CLIENT_ID,
        minecraft_version: MINECRAFT_VERSION,
        minecraft_directory: minecraft.map(|path| path.display().to_string()),
        mod_installed,
        java_available,
        fabric_installed,
        fabric_api_installed,
    }
}

#[tauri::command]
fn install_aureus_mod() -> Result<String, String> {
    let minecraft = minecraft_directory().ok_or("No se pudo localizar la carpeta de Minecraft")?;
    let mods = minecraft.join("mods");
    fs::create_dir_all(&mods).map_err(|error| error.to_string())?;
    let destination = mods.join(MOD_FILE_NAME);
    fs::write(&destination, MOD_BYTES).map_err(|error| error.to_string())?;
    Ok(destination.display().to_string())
}

#[tauri::command]
async fn prepare_minecraft() -> Result<String, String> {
    let minecraft = minecraft_directory().ok_or("No se pudo localizar la carpeta de Minecraft")?;
    fs::create_dir_all(&minecraft).map_err(|error| error.to_string())?;

    let installer = std::env::temp_dir().join("aureus-fabric-installer-1.1.2.jar");
    fs::write(&installer, FABRIC_INSTALLER_BYTES).map_err(|error| error.to_string())?;
    let install_result = std::process::Command::new("java")
        .arg("-jar")
        .arg(&installer)
        .arg("client")
        .arg("-dir")
        .arg(&minecraft)
        .arg("-mcversion")
        .arg(MINECRAFT_VERSION)
        .arg("-loader")
        .arg("0.19.3")
        .output()
        .map_err(|error| format!("No se pudo ejecutar Java: {error}"))?;
    let _ = fs::remove_file(&installer);
    if !install_result.status.success() {
        return Err(format!(
            "Fabric no pudo instalarse: {}",
            String::from_utf8_lossy(&install_result.stderr)
        ));
    }

    let mods = minecraft.join("mods");
    fs::create_dir_all(&mods).map_err(|error| error.to_string())?;
    fs::write(mods.join(FABRIC_API_FILE_NAME), FABRIC_API_BYTES)
        .map_err(|error| error.to_string())?;
    fs::write(mods.join(MOD_FILE_NAME), MOD_BYTES).map_err(|error| error.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent("Aureus-Launcher/0.3.0")
        .build()
        .map_err(|error| error.to_string())?;
    for (file_name, url) in PERFORMANCE_MODS {
        let destination = mods.join(file_name);
        if destination.exists() {
            continue;
        }
        let response = client
            .get(*url)
            .send()
            .await
            .map_err(|error| format!("No se pudo descargar {file_name}: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "No se pudo descargar {file_name}: HTTP {}",
                response.status()
            ));
        }
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        let temporary = destination.with_extension("jar.part");
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        fs::rename(temporary, destination).map_err(|error| error.to_string())?;
    }
    Ok(format!(
        "Minecraft {MINECRAFT_VERSION}, Aureus y {} optimizadores están listos",
        PERFORMANCE_MODS.len()
    ))
}

fn library_path_from_name(root: &std::path::Path, name: &str) -> Option<PathBuf> {
    let parts: Vec<_> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    Some(
        root.join(group)
            .join(artifact)
            .join(version)
            .join(format!("{artifact}-{version}{classifier}.jar")),
    )
}

fn newest_natives_directory(minecraft: &std::path::Path) -> Result<PathBuf, String> {
    let bin = minecraft.join("bin");
    fs::read_dir(&bin).map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .max_by_key(|entry| entry.metadata().and_then(|value| value.modified()).ok())
        .map(|entry| entry.path())
        .ok_or("No se encontraron los componentes nativos. Inicia Minecraft una vez desde el launcher oficial.".into())
}

fn ensure_default_resource_pack(minecraft: &std::path::Path) -> Result<(), String> {
    let pack = minecraft
        .join("resourcepacks")
        .join("The Better Default Pack");
    if !pack.join("pack.mcmeta").exists() {
        return Ok(());
    }
    let options_path = minecraft.join("options.txt");
    let contents = fs::read_to_string(&options_path).unwrap_or_default();
    let desired = "resourcePacks:[\"file/The Better Default Pack\"]";
    let mut found = false;
    let mut lines: Vec<String> = contents
        .lines()
        .map(|line| {
            if line.starts_with("resourcePacks:") {
                found = true;
                desired.to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(desired.to_string());
    }
    fs::write(options_path, format!("{}\n", lines.join("\n"))).map_err(|error| error.to_string())
}

fn ensure_memory_saver_options(minecraft: &std::path::Path) -> Result<(), String> {
    let options_path = minecraft.join("options.txt");
    let contents = fs::read_to_string(&options_path).unwrap_or_default();
    let settings = [
        ("ao", "false"),
        ("enableVsync", "false"),
        ("entityDistanceScaling", "0.5"),
        ("entityShadows", "false"),
        ("graphicsPreset", "\"fast\""),
        ("maxFps", "90"),
        ("mipmapLevels", "0"),
        ("particles", "2"),
        ("renderClouds", "\"false\""),
        ("renderDistance", "12"),
        ("simulationDistance", "8"),
        ("biomeBlendRadius", "0"),
        ("menuBackgroundBlurriness", "0"),
        ("improvedTransparency", "false"),
    ];
    let mut found = std::collections::HashSet::new();
    let mut lines: Vec<String> = contents
        .lines()
        .map(|line| {
            let Some((key, _)) = line.split_once(':') else {
                return line.to_string();
            };
            if let Some((_, value)) = settings.iter().find(|(name, _)| *name == key) {
                found.insert(key.to_string());
                format!("{key}:{value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    for (key, value) in settings {
        if !found.contains(key) {
            lines.push(format!("{key}:{value}"));
        }
    }
    fs::write(options_path, format!("{}\n", lines.join("\n"))).map_err(|error| error.to_string())
}

#[tauri::command]
fn launch_minecraft(
    app: tauri::AppHandle,
    auth: State<'_, AuthState>,
    runtime: State<'_, RuntimeState>,
) -> Result<String, String> {
    runtime.cancel_requested.store(false, Ordering::SeqCst);
    if runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")?
        .is_some()
    {
        return Err("Minecraft ya está ejecutándose".into());
    }
    emit_launch(&app, 8, "Sesión", "Comprobando la cuenta de Minecraft");
    ensure_not_cancelled(&runtime)?;
    let session = auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")?
        .clone()
        .ok_or("Inicia sesión con Microsoft desde Cuenta antes de jugar")?;
    emit_launch(
        &app,
        18,
        "Sesión",
        format!("Cuenta verificada: {}", session.username),
    );
    let minecraft = minecraft_directory().ok_or("No se pudo localizar Minecraft")?;
    ensure_default_resource_pack(&minecraft)?;
    if matches!(
        read_instances()?
            .first()
            .map(|item| item.performance_profile.as_str()),
        Some("MEMORY_SAVER" | "CUSTOM")
    ) {
        ensure_memory_saver_options(&minecraft)?;
    }
    emit_launch(
        &app,
        28,
        "Archivos",
        "Leyendo Minecraft 1.21.11 y Fabric 0.19.3",
    );
    ensure_not_cancelled(&runtime)?;
    let base_json: serde_json::Value = serde_json::from_slice(
        &fs::read(minecraft.join("versions/1.21.11/1.21.11.json"))
            .map_err(|_| "Faltan los archivos base de Minecraft 1.21.11")?,
    )
    .map_err(|error| error.to_string())?;
    let fabric_json: serde_json::Value = serde_json::from_slice(
        &fs::read(
            minecraft
                .join("versions")
                .join(FABRIC_PROFILE)
                .join(format!("{FABRIC_PROFILE}.json")),
        )
        .map_err(|_| "Falta el perfil de Fabric")?,
    )
    .map_err(|error| error.to_string())?;

    let libraries_root = minecraft.join("libraries");
    let mut classpath: Vec<PathBuf> = Vec::new();
    for library in base_json["libraries"].as_array().into_iter().flatten() {
        let Some(path) = library["downloads"]["artifact"]["path"].as_str() else {
            continue;
        };
        if path.contains("natives-linux")
            || path.contains("natives-windows")
            || path.contains("linux-")
            || path.contains("windows-")
        {
            continue;
        }
        let full = libraries_root.join(path);
        if full.exists() {
            classpath.push(full);
        }
    }
    for library in fabric_json["libraries"].as_array().into_iter().flatten() {
        if let Some(name) = library["name"].as_str() {
            if let Some(path) = library_path_from_name(&libraries_root, name) {
                if path.exists() {
                    classpath.push(path);
                }
            }
        }
    }
    let client_jar = minecraft
        .join("versions")
        .join(FABRIC_PROFILE)
        .join(format!("{FABRIC_PROFILE}.jar"));
    if !client_jar.exists() {
        return Err("Falta el cliente de Minecraft preparado por Fabric".into());
    }
    classpath.push(client_jar);
    emit_launch(
        &app,
        48,
        "Librerías",
        format!("{} componentes preparados", classpath.len()),
    );
    ensure_not_cancelled(&runtime)?;
    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let classpath = classpath
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(separator);
    let natives = newest_natives_directory(&minecraft)?;
    emit_launch(
        &app,
        62,
        "Sistema",
        "Componentes nativos de macOS preparados",
    );
    let instance = read_instances()?
        .into_iter()
        .next()
        .unwrap_or(default_instance()?);
    let memory = instance.memory_mb.clamp(2048, 16384);
    let log_config = minecraft.join("assets/log_configs/client-1.21.2.xml");

    #[cfg(target_os = "macos")]
    let bundled_java = minecraft.join(
        "runtime/java-runtime-delta/mac-os/java-runtime-delta/jre.bundle/Contents/Home/bin/java",
    );
    #[cfg(target_os = "windows")]
    let bundled_java =
        minecraft.join("runtime/java-runtime-delta/windows-x64/java-runtime-delta/bin/javaw.exe");
    #[cfg(target_os = "linux")]
    let bundled_java =
        minecraft.join("runtime/java-runtime-delta/linux/java-runtime-delta/bin/java");
    let java: PathBuf = if bundled_java.exists() {
        bundled_java
    } else {
        PathBuf::from("java")
    };
    emit_launch(
        &app,
        74,
        "Java",
        format!(
            "Memoria: {} GB · Perfil: {}",
            memory / 1024,
            instance.performance_profile
        ),
    );
    ensure_not_cancelled(&runtime)?;

    let log_dir = aureus_data_directory()?.join("logs");
    fs::create_dir_all(&log_dir).map_err(|error| error.to_string())?;
    let stdout =
        File::create(log_dir.join("minecraft-latest.log")).map_err(|error| error.to_string())?;
    let stderr = stdout.try_clone().map_err(|error| error.to_string())?;
    let mut command = std::process::Command::new(java);
    command
        .current_dir(&minecraft)
        .arg("-XstartOnFirstThread")
        .arg("-Xss1M")
        .arg("-Xms512M")
        .arg(format!("-Xmx{memory}M"))
        .arg("-XX:+UseG1GC")
        .arg("-XX:+ParallelRefProcEnabled")
        .arg("-XX:+UseStringDeduplication")
        .arg("-XX:MinHeapFreeRatio=10")
        .arg("-XX:MaxHeapFreeRatio=30")
        .arg(format!("-Djava.library.path={}", natives.display()))
        .arg(format!("-Djna.tmpdir={}", natives.display()))
        .arg(format!(
            "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
            natives.display()
        ))
        .arg(format!("-Dio.netty.native.workdir={}", natives.display()))
        .arg("-Dminecraft.launcher.brand=aureus-launcher")
        .arg("-Dminecraft.launcher.version=0.3.0");
    if log_config.exists() {
        command.arg(format!(
            "-Dlog4j.configurationFile={}",
            log_config.display()
        ));
    }
    emit_launch(&app, 88, "Fabric", "Cargando Fabric API y Aureus Client");
    ensure_not_cancelled(&runtime)?;
    let mut child = command
        .arg("-cp")
        .arg(classpath)
        .arg("-DFabricMcEmu= net.minecraft.client.main.Main ")
        .arg("net.fabricmc.loader.impl.launch.knot.KnotClient")
        .arg("--username")
        .arg(&session.username)
        .arg("--version")
        .arg(FABRIC_PROFILE)
        .arg("--gameDir")
        .arg(&minecraft)
        .arg("--assetsDir")
        .arg(minecraft.join("assets"))
        .arg("--assetIndex")
        .arg(base_json["assetIndex"]["id"].as_str().unwrap_or("29"))
        .arg("--uuid")
        .arg(&session.uuid)
        .arg("--accessToken")
        .arg(&session._access_token)
        .arg("--clientId")
        .arg(CLIENT_ID)
        .arg("--xuid")
        .arg("")
        .arg("--versionType")
        .arg("release")
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| format!("No se pudo iniciar Minecraft: {error}"))?;
    let pid = child.id();
    *runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")? = Some(pid);
    *runtime
        .started_at
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")? = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs(),
    );
    emit_launch(
        &app,
        100,
        "Jugando",
        format!("Minecraft iniciado como {}", session.username),
    );
    let monitor_app = app.clone();
    std::thread::spawn(move || {
        let message = match child.wait() {
            Ok(status) if status.success() => "Minecraft se cerró correctamente".to_string(),
            Ok(status) => format!(
                "Minecraft se cerró con código {}",
                status.code().unwrap_or(-1)
            ),
            Err(error) => format!("No se pudo supervisar Minecraft: {error}"),
        };
        if let Ok(mut running_pid) = monitor_app.state::<RuntimeState>().game_pid.lock() {
            if *running_pid == Some(pid) {
                *running_pid = None;
            }
        }
        if let Ok(mut started_at) = monitor_app.state::<RuntimeState>().started_at.lock() {
            *started_at = None;
        }
        let _ = monitor_app.emit("game-state", message);
    });
    Ok(format!(
        "Minecraft iniciado directamente como {} (PID {})",
        session.username, pid
    ))
}

fn replace_launch_tokens(
    value: &str,
    descriptor: &InstanceDescriptor,
    minecraft: &std::path::Path,
    session: &MinecraftSession,
) -> String {
    let replacements = vec![
        ("${auth_player_name}", session.username.clone()),
        ("${version_name}", descriptor.version.clone()),
        (
            "${game_directory}",
            descriptor.game_directory.to_string_lossy().into_owned(),
        ),
        (
            "${assets_root}",
            minecraft.join("assets").to_string_lossy().into_owned(),
        ),
        ("${assets_index_name}", descriptor.asset_index.clone()),
        ("${auth_uuid}", session.uuid.clone()),
        ("${auth_access_token}", session._access_token.clone()),
        ("${user_type}", "msa".into()),
        ("${version_type}", "release".into()),
        ("${user_properties}", "{}".into()),
        (
            "${auth_session}",
            format!("token:{}:{}", session._access_token, session.uuid),
        ),
        (
            "${game_assets}",
            minecraft
                .join("assets/virtual")
                .join(&descriptor.asset_index)
                .to_string_lossy()
                .into_owned(),
        ),
        ("${clientid}", CLIENT_ID.into()),
        ("${auth_xuid}", String::new()),
        ("${launcher_name}", "Aureus".into()),
        ("${launcher_version}", "0.3.0".into()),
        (
            "${library_directory}",
            minecraft.join("libraries").to_string_lossy().into_owned(),
        ),
        (
            "${classpath_separator}",
            if cfg!(target_os = "windows") {
                ";".into()
            } else {
                ":".into()
            },
        ),
    ];
    replacements
        .into_iter()
        .fold(value.to_string(), |text, (from, to)| {
            text.replace(from, &to)
        })
}

#[tauri::command]
fn launch_selected_minecraft(
    app: tauri::AppHandle,
    auth: State<'_, AuthState>,
    runtime: State<'_, RuntimeState>,
) -> Result<String, String> {
    let version = read_selected_version();
    let instance_id = read_selected_instance();
    if runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")?
        .is_some()
    {
        return Err("Minecraft ya está ejecutándose".into());
    }
    let session = auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")?
        .clone()
        .ok_or("Inicia sesión con Microsoft desde Cuenta antes de jugar")?;
    let instance_dir = aureus_data_directory()?
        .join("instances-data")
        .join(&instance_id);
    let descriptor: InstanceDescriptor = serde_json::from_slice(
        &fs::read(instance_dir.join("aureus-instance.json"))
            .map_err(|_| "Prepara esta versión antes de jugar")?,
    )
    .map_err(|error| error.to_string())?;
    let minecraft = minecraft_directory().ok_or("No se pudo localizar Minecraft")?;
    emit_launch(
        &app,
        18,
        "Instancia",
        format!("Cargando {} ({})", version, descriptor.state),
    );
    let base_meta = version_manager::read_launch_metadata(&descriptor.version_json)?;
    let natives = instance_dir.join("natives");
    version_manager::extract_natives(&descriptor.version_json, &minecraft, &natives)?;
    let mut classpath =
        version_manager::classpath(&descriptor.version_json, &minecraft, &descriptor.client_jar)?;
    let mut main_class = base_meta.main_class;
    if descriptor.state == "fabric-ready" {
        if let Some(loader) = &descriptor.loader_version {
            let profile = format!("fabric-loader-{loader}-{version}");
            let profile_json = minecraft
                .join("versions")
                .join(&profile)
                .join(format!("{profile}.json"));
            let value: serde_json::Value = serde_json::from_slice(
                &fs::read(&profile_json).map_err(|_| "El perfil de Fabric está incompleto")?,
            )
            .map_err(|error| error.to_string())?;
            main_class = value
                .get("mainClass")
                .and_then(|item| item.as_str())
                .unwrap_or("net.fabricmc.loader.impl.launch.knot.KnotClient")
                .into();
            for library in value
                .get("libraries")
                .and_then(|item| item.as_array())
                .into_iter()
                .flatten()
            {
                if let Some(name) = library.get("name").and_then(|item| item.as_str()) {
                    if let Some(path) = library_path_from_name(&minecraft.join("libraries"), name) {
                        if path.exists() {
                            classpath.push(path);
                        }
                    }
                }
            }
            let profile_jar = minecraft
                .join("versions")
                .join(&profile)
                .join(format!("{profile}.jar"));
            if profile_jar.exists() {
                classpath.pop();
                classpath.push(profile_jar);
            }
        }
    }
    let java = descriptor
        .java_path
        .clone()
        .ok_or("La instancia no tiene un runtime Java preparado")?;
    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let cp = classpath
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(separator);
    fs::create_dir_all(&descriptor.game_directory).map_err(|error| error.to_string())?;
    let log_dir = aureus_data_directory()?.join("logs");
    fs::create_dir_all(&log_dir).map_err(|error| error.to_string())?;
    let stdout = File::create(log_dir.join(format!("minecraft-{version}-latest.log")))
        .map_err(|error| error.to_string())?;
    let stderr = stdout.try_clone().map_err(|error| error.to_string())?;
    emit_launch(
        &app,
        70,
        "Java",
        format!("Java {} · preparando proceso", base_meta.java_major),
    );
    let mut command = std::process::Command::new(java);
    let has_version_jvm_args = !base_meta.jvm_args.is_empty();
    let (memory_mb, width, height, custom_jvm) =
        professional::launch_preferences(&instance_id).unwrap_or((5120, 1280, 720, Vec::new()));
    command
        .current_dir(&descriptor.game_directory)
        .arg("-Xms512M")
        .arg(format!("-Xmx{memory_mb}M"));
    for argument in custom_jvm {
        command.arg(argument);
    }
    if cfg!(target_os = "macos") && !has_version_jvm_args {
        command.arg("-XstartOnFirstThread");
    }
    for argument in base_meta.jvm_args {
        if argument == "-cp"
            || argument.contains("${classpath}")
            || argument.contains("${natives_directory}")
            || argument.starts_with("-Djava.library.path")
        {
            continue;
        }
        command.arg(replace_launch_tokens(
            &argument,
            &descriptor,
            &minecraft,
            &session,
        ));
    }
    let supports_resolution =
        main_class.contains("KnotClient") || main_class.contains("client.main.Main");
    command
        .arg(format!("-Djava.library.path={}", natives.display()))
        .arg("-cp")
        .arg(cp)
        .arg(main_class);
    for argument in base_meta.game_args {
        command.arg(replace_launch_tokens(
            &argument,
            &descriptor,
            &minecraft,
            &session,
        ));
    }
    if supports_resolution {
        command.args([
            "--width",
            &width.to_string(),
            "--height",
            &height.to_string(),
        ]);
    }
    let mut child = command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| format!("No se pudo iniciar Minecraft {version}: {error}"))?;
    let pid = child.id();
    *runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")? = Some(pid);
    *runtime
        .started_at
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")? = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs(),
    );
    emit_launch(
        &app,
        100,
        "Jugando",
        format!("Minecraft {version} iniciado"),
    );
    let monitor = app.clone();
    let monitored_version = version.clone();
    std::thread::spawn(move || {
        let message = child
            .wait()
            .map(|status| {
                if status.success() {
                    format!("Minecraft {monitored_version} se cerró correctamente")
                } else {
                    format!("Minecraft {monitored_version} se cerró con error")
                }
            })
            .unwrap_or_else(|error| error.to_string());
        if let Ok(mut value) = monitor.state::<RuntimeState>().game_pid.lock() {
            *value = None;
        }
        if let Ok(mut value) = monitor.state::<RuntimeState>().started_at.lock() {
            *value = None;
        }
        let _ = monitor.emit("game-state", message);
    });
    Ok(format!("Minecraft {version} iniciado (PID {pid})"))
}

#[tauri::command]
fn cancel_launch(
    app: tauri::AppHandle,
    runtime: State<'_, RuntimeState>,
) -> Result<String, String> {
    runtime.cancel_requested.store(true, Ordering::SeqCst);
    emit_launch(&app, 0, "Cancelado", "El inicio fue cancelado");
    Ok("Inicio cancelado".into())
}

#[tauri::command]
fn kill_minecraft(runtime: State<'_, RuntimeState>) -> Result<String, String> {
    let pid = runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")?
        .ok_or("No hay una instancia de Minecraft ejecutándose")?;
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status();
    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
    match status {
        Ok(value) if value.success() => Ok(format!("Se solicitó cerrar Minecraft (PID {pid})")),
        Ok(value) => Err(format!("El sistema no pudo cerrar Minecraft: {value}")),
        Err(error) => Err(format!("No se pudo cerrar Minecraft: {error}")),
    }
}

#[tauri::command]
fn runtime_status(runtime: State<'_, RuntimeState>) -> Result<RuntimeStatus, String> {
    let stored_pid = runtime
        .game_pid
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")?;
    let pid = *stored_pid;
    let started_at = *runtime
        .started_at
        .lock()
        .map_err(|_| "Estado del proceso bloqueado")?;
    Ok(RuntimeStatus {
        running: pid.is_some(),
        pid,
        started_at,
    })
}

#[tauri::command]
fn begin_microsoft_login(auth: State<'_, AuthState>) -> Result<LoginStart, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let redirect_uri = format!("http://localhost:{port}");
    let verifier: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let request_state: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let (sender, receiver) = mpsc::channel();
    let expected_state = request_state.clone();

    std::thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(180)))
                .map_err(|error| error.to_string())?;
            let mut buffer = [0_u8; 8192];
            let count = stream
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            let request = String::from_utf8_lossy(&buffer[..count]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .ok_or("Respuesta OAuth no válida")?;
            let callback = url::Url::parse(&format!("http://localhost{path}"))
                .map_err(|error| error.to_string())?;
            let params: std::collections::HashMap<_, _> =
                callback.query_pairs().into_owned().collect();
            if params.get("state") != Some(&expected_state) {
                return Err("El estado OAuth no coincide".into());
            }
            let code = params.get("code").cloned().ok_or_else(|| {
                params
                    .get("error_description")
                    .cloned()
                    .unwrap_or_else(|| "Microsoft no devolvió un código".into())
            })?;
            let body = "<html><body style='font-family:system-ui;background:#0b0e0b;color:#eef5df;padding:48px'><h1>Aureus Launcher</h1><p>Inicio de sesión recibido. Ya puedes cerrar esta pestaña.</p></body></html>";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            stream
                .write_all(response.as_bytes())
                .map_err(|error| error.to_string())?;
            Ok(code)
        })();
        let _ = sender.send(result);
    });

    *auth
        .pending
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")? = Some(PendingLogin {
        receiver,
        verifier,
        redirect_uri: redirect_uri.clone(),
    });

    let mut authorize =
        url::Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
            .map_err(|error| error.to_string())?;
    authorize
        .query_pairs_mut()
        .append_pair("client_id", CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", "XboxLive.signin offline_access")
        .append_pair("state", &request_state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(LoginStart {
        authorization_url: authorize.into(),
    })
}

#[tauri::command]
async fn complete_microsoft_login(auth: State<'_, AuthState>) -> Result<LoginResult, String> {
    let pending = auth
        .pending
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")?
        .take()
        .ok_or("No hay un inicio de sesión pendiente")?;
    let PendingLogin {
        receiver,
        verifier,
        redirect_uri,
    } = pending;
    let code = tauri::async_runtime::spawn_blocking(move || {
        receiver.recv_timeout(Duration::from_secs(180))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|_| "Se agotó el tiempo de inicio de sesión")??;
    let response = reqwest::Client::new()
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Microsoft rechazó el token: {}",
            response.text().await.unwrap_or_default()
        ));
    }
    let token: TokenResponse = response.json().await.map_err(|error| error.to_string())?;
    *auth
        .access_token
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")? = Some(token.access_token);
    Ok(LoginResult {
        authenticated: true,
        expires_in: token.expires_in,
    })
}

#[tauri::command]
async fn connect_minecraft(auth: State<'_, AuthState>) -> Result<MinecraftSession, String> {
    let microsoft_token = auth
        .access_token
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")?
        .clone()
        .ok_or("Primero inicia sesión con Microsoft")?;
    let client = reqwest::Client::new();

    let xbox_response = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !xbox_response.status().is_success() {
        return Err(format!(
            "Xbox Live rechazó la sesión: {}",
            xbox_response.text().await.unwrap_or_default()
        ));
    }
    let xbox: serde_json::Value = xbox_response
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let xbox_token = xbox["Token"]
        .as_str()
        .ok_or("Xbox Live no devolvió un token")?;
    let user_hash = xbox["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("Xbox Live no devolvió el usuario")?;

    let xsts_response = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbox_token]},
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !xsts_response.status().is_success() {
        return Err(format!(
            "XSTS rechazó la cuenta: {}",
            xsts_response.text().await.unwrap_or_default()
        ));
    }
    let xsts: serde_json::Value = xsts_response
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let xsts_token = xsts["Token"].as_str().ok_or("XSTS no devolvió un token")?;

    let minecraft_response = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({"identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")}))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !minecraft_response.status().is_success() {
        return Err(format!(
            "Minecraft rechazó la sesión: {}",
            minecraft_response.text().await.unwrap_or_default()
        ));
    }
    let minecraft_token: MinecraftTokenResponse = minecraft_response
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let entitlements = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&minecraft_token.access_token)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !entitlements.status().is_success() {
        return Err("No se pudo verificar la propiedad de Minecraft".into());
    }
    let ownership: serde_json::Value = entitlements
        .json()
        .await
        .map_err(|error| error.to_string())?;
    if ownership["items"]
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        return Err("Esta cuenta no posee Minecraft Java Edition".into());
    }

    let profile_response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft_token.access_token)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !profile_response.status().is_success() {
        return Err("La cuenta no tiene un perfil de Minecraft Java".into());
    }
    let profile: MinecraftProfileResponse = profile_response
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let session = MinecraftSession {
        username: profile.name,
        uuid: profile.id,
        _access_token: minecraft_token.access_token,
    };
    save_stored_session(&session, minecraft_token.expires_in)?;
    *auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")? = Some(session.clone());
    Ok(session)
}

#[tauri::command]
fn current_minecraft_session(
    auth: State<'_, AuthState>,
) -> Result<Option<MinecraftSession>, String> {
    Ok(auth
        .minecraft_session
        .lock()
        .map_err(|_| "Estado de autenticación bloqueado")?
        .clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AuthState::default())
        .manage(RuntimeState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            launcher_status,
            version_catalog,
            selected_version,
            selected_version_status,
            select_version,
            install_selected_version,
            list_instances,
            save_instance,
            collect_diagnostics,
            install_aureus_mod,
            prepare_minecraft,
            launch_minecraft,
            launch_selected_minecraft,
            cancel_launch,
            kill_minecraft,
            runtime_status,
            begin_microsoft_login,
            complete_microsoft_login,
            connect_minecraft,
            current_minecraft_session,
            professional::managed_instances,
            professional::upsert_managed_instance,
            professional::duplicate_managed_instance,
            professional::delete_managed_instance,
            professional::list_instance_content,
            professional::toggle_instance_content,
            professional::create_instance_backup,
            professional::restore_instance_backup,
            professional::export_instance,
            professional::import_instance,
            professional::recommend_hardware_profile,
            professional::sync_client_config,
            professional::analyze_latest_crash,
            professional::search_modrinth,
            professional::install_modrinth,
            professional::enable_safe_mode,
            professional::read_benchmark,
            list_minecraft_accounts,
            switch_minecraft_account,
            logout_minecraft_account,
            check_launcher_update,
            select_managed_instance
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Aureus Launcher");
}
