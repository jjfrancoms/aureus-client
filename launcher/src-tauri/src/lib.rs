use serde::Serialize;
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distr::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use tauri::State;

const CLIENT_ID: &str = "4e82ebb8-bdf1-48f8-a1c2-8a62cd1be7a8";
const MINECRAFT_VERSION: &str = "1.21.11";
const MOD_FILE_NAME: &str = "aureus-client-0.3.0-minecraft-1.21.11.jar";
const MOD_BYTES: &[u8] = include_bytes!("../../../outputs/aureus-client-0.3.0-minecraft-1.21.11.jar");

struct PendingLogin {
    receiver: mpsc::Receiver<Result<String, String>>,
    verifier: String,
    redirect_uri: String,
}

#[derive(Default)]
struct AuthState {
    pending: Mutex<Option<PendingLogin>>,
    access_token: Mutex<Option<String>>,
    minecraft_session: Mutex<Option<MinecraftSession>>,
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

    LauncherStatus {
        client_id: CLIENT_ID,
        minecraft_version: MINECRAFT_VERSION,
        minecraft_directory: minecraft.map(|path| path.display().to_string()),
        mod_installed,
        java_available,
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
fn begin_microsoft_login(auth: State<'_, AuthState>) -> Result<LoginStart, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let redirect_uri = format!("http://localhost:{port}");
    let verifier: String = rand::rng().sample_iter(&Alphanumeric).take(64).map(char::from).collect();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let request_state: String = rand::rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let (sender, receiver) = mpsc::channel();
    let expected_state = request_state.clone();

    std::thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(180))).map_err(|error| error.to_string())?;
            let mut buffer = [0_u8; 8192];
            let count = stream.read(&mut buffer).map_err(|error| error.to_string())?;
            let request = String::from_utf8_lossy(&buffer[..count]);
            let path = request.lines().next().and_then(|line| line.split_whitespace().nth(1))
                .ok_or("Respuesta OAuth no válida")?;
            let callback = url::Url::parse(&format!("http://localhost{path}")).map_err(|error| error.to_string())?;
            let params: std::collections::HashMap<_, _> = callback.query_pairs().into_owned().collect();
            if params.get("state") != Some(&expected_state) {
                return Err("El estado OAuth no coincide".into());
            }
            let code = params.get("code").cloned().ok_or_else(|| params.get("error_description")
                .cloned().unwrap_or_else(|| "Microsoft no devolvió un código".into()))?;
            let body = "<html><body style='font-family:system-ui;background:#0b0e0b;color:#eef5df;padding:48px'><h1>Aureus Launcher</h1><p>Inicio de sesión recibido. Ya puedes cerrar esta pestaña.</p></body></html>";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            stream.write_all(response.as_bytes()).map_err(|error| error.to_string())?;
            Ok(code)
        })();
        let _ = sender.send(result);
    });

    *auth.pending.lock().map_err(|_| "Estado de autenticación bloqueado")? = Some(PendingLogin {
        receiver,
        verifier,
        redirect_uri: redirect_uri.clone(),
    });

    let mut authorize = url::Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
        .map_err(|error| error.to_string())?;
    authorize.query_pairs_mut()
        .append_pair("client_id", CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", "XboxLive.signin offline_access")
        .append_pair("state", &request_state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(LoginStart { authorization_url: authorize.into() })
}

#[tauri::command]
async fn complete_microsoft_login(auth: State<'_, AuthState>) -> Result<LoginResult, String> {
    let pending = auth.pending.lock().map_err(|_| "Estado de autenticación bloqueado")?
        .take().ok_or("No hay un inicio de sesión pendiente")?;
    let PendingLogin { receiver, verifier, redirect_uri } = pending;
    let code = tauri::async_runtime::spawn_blocking(move || receiver.recv_timeout(Duration::from_secs(180)))
        .await.map_err(|error| error.to_string())?
        .map_err(|_| "Se agotó el tiempo de inicio de sesión")??;
    let response = reqwest::Client::new()
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", CLIENT_ID), ("grant_type", "authorization_code"),
            ("code", code.as_str()), ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send().await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Microsoft rechazó el token: {}", response.text().await.unwrap_or_default()));
    }
    let token: TokenResponse = response.json().await.map_err(|error| error.to_string())?;
    *auth.access_token.lock().map_err(|_| "Estado de autenticación bloqueado")? = Some(token.access_token);
    Ok(LoginResult { authenticated: true, expires_in: token.expires_in })
}

#[tauri::command]
async fn connect_minecraft(auth: State<'_, AuthState>) -> Result<MinecraftSession, String> {
    let microsoft_token = auth.access_token.lock().map_err(|_| "Estado de autenticación bloqueado")?
        .clone().ok_or("Primero inicia sesión con Microsoft")?;
    let client = reqwest::Client::new();

    let xbox_response = client.post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        })).send().await.map_err(|error| error.to_string())?;
    if !xbox_response.status().is_success() {
        return Err(format!("Xbox Live rechazó la sesión: {}", xbox_response.text().await.unwrap_or_default()));
    }
    let xbox: serde_json::Value = xbox_response.json().await.map_err(|error| error.to_string())?;
    let xbox_token = xbox["Token"].as_str().ok_or("Xbox Live no devolvió un token")?;
    let user_hash = xbox["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or("Xbox Live no devolvió el usuario")?;

    let xsts_response = client.post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbox_token]},
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        })).send().await.map_err(|error| error.to_string())?;
    if !xsts_response.status().is_success() {
        return Err(format!("XSTS rechazó la cuenta: {}", xsts_response.text().await.unwrap_or_default()));
    }
    let xsts: serde_json::Value = xsts_response.json().await.map_err(|error| error.to_string())?;
    let xsts_token = xsts["Token"].as_str().ok_or("XSTS no devolvió un token")?;

    let minecraft_response = client.post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({"identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")}))
        .send().await.map_err(|error| error.to_string())?;
    if !minecraft_response.status().is_success() {
        return Err(format!("Minecraft rechazó la sesión: {}", minecraft_response.text().await.unwrap_or_default()));
    }
    let minecraft_token: MinecraftTokenResponse = minecraft_response.json().await.map_err(|error| error.to_string())?;

    let entitlements = client.get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&minecraft_token.access_token).send().await.map_err(|error| error.to_string())?;
    if !entitlements.status().is_success() {
        return Err("No se pudo verificar la propiedad de Minecraft".into());
    }
    let ownership: serde_json::Value = entitlements.json().await.map_err(|error| error.to_string())?;
    if ownership["items"].as_array().map(|items| items.is_empty()).unwrap_or(true) {
        return Err("Esta cuenta no posee Minecraft Java Edition".into());
    }

    let profile_response = client.get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft_token.access_token).send().await.map_err(|error| error.to_string())?;
    if !profile_response.status().is_success() {
        return Err("La cuenta no tiene un perfil de Minecraft Java".into());
    }
    let profile: MinecraftProfileResponse = profile_response.json().await.map_err(|error| error.to_string())?;
    let session = MinecraftSession {
        username: profile.name,
        uuid: profile.id,
        _access_token: minecraft_token.access_token,
    };
    *auth.minecraft_session.lock().map_err(|_| "Estado de autenticación bloqueado")? = Some(session.clone());
    Ok(session)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AuthState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![launcher_status, install_aureus_mod, begin_microsoft_login, complete_microsoft_login, connect_minecraft])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Aureus Launcher");
}
