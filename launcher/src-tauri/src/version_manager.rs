use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::io::{self, Write};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

struct DownloadTracker {
    downloaded: AtomicU64,
    total: AtomicU64,
    started_ms: AtomicU64,
}
static DOWNLOAD_TRACKER: OnceLock<DownloadTracker> = OnceLock::new();
fn tracker() -> &'static DownloadTracker {
    DOWNLOAD_TRACKER.get_or_init(|| DownloadTracker {
        downloaded: AtomicU64::new(0),
        total: AtomicU64::new(0),
        started_ms: AtomicU64::new(0),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadMetrics {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
}

pub fn reset_download_metrics() {
    let current = tracker();
    current.downloaded.store(0, Ordering::Relaxed);
    current.total.store(0, Ordering::Relaxed);
    current.started_ms.store(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        Ordering::Relaxed,
    );
}

#[tauri::command]
pub fn download_metrics() -> DownloadMetrics {
    let current = tracker();
    let downloaded = current.downloaded.load(Ordering::Relaxed);
    let started = current.started_ms.load(Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let elapsed_ms = now.saturating_sub(started).max(1);
    DownloadMetrics {
        downloaded_bytes: downloaded,
        total_bytes: current.total.load(Ordering::Relaxed),
        bytes_per_second: downloaded.saturating_mul(1000) / elapsed_ms,
    }
}

#[derive(Clone, Deserialize)]
pub struct DownloadFile {
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<ManifestVersion>,
}

#[derive(Deserialize)]
struct ManifestVersion {
    id: String,
    url: String,
    sha1: String,
}

#[derive(Deserialize)]
struct VersionDownloads {
    client: DownloadFile,
}

#[derive(Deserialize)]
struct Artifact {
    path: String,
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct LibraryDownloads {
    artifact: Option<Artifact>,
    classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize)]
struct Library {
    downloads: Option<LibraryDownloads>,
}

#[derive(Deserialize)]
struct AssetIndexRef {
    id: String,
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct VersionMeta {
    downloads: VersionDownloads,
    libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexRef,
}

#[derive(Clone, Deserialize)]
struct AssetObject {
    hash: String,
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
    #[serde(default)]
    r#virtual: bool,
    #[serde(default)]
    map_to_resources: bool,
}

pub struct InstalledVersion {
    pub version_json: PathBuf,
    pub client_jar: PathBuf,
    pub asset_index_id: String,
    pub downloaded_files: usize,
}

pub struct LaunchMetadata {
    pub main_class: String,
    pub game_args: Vec<String>,
    pub jvm_args: Vec<String>,
    pub java_major: u32,
}

#[derive(Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
}
#[derive(Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}
#[derive(Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: String,
    name: String,
}

pub(crate) fn find_java(root: &Path) -> Option<PathBuf> {
    let executable = if cfg!(target_os = "windows") {
        "javaw.exe"
    } else {
        "java"
    };
    let mut queue = VecDeque::from([root.to_path_buf()]);
    while let Some(directory) = queue.pop_front() {
        for entry in fs::read_dir(directory).ok()?.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                queue.push_back(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(executable)
                && path.parent()?.file_name()?.to_str() == Some("bin")
            {
                return Some(path);
            }
        }
    }
    None
}

pub async fn ensure_java_runtime(
    major: u32,
    runtime_root: &Path,
    client: &reqwest::Client,
) -> Result<PathBuf, String> {
    let destination = runtime_root.join(format!("java-{major}"));
    if let Some(java) = find_java(&destination) {
        return Ok(java);
    }
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };
    let url = format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={arch}&image_type=jre&os={os}&vendor=eclipse");
    let assets: Vec<AdoptiumAsset> = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let package = &assets
        .first()
        .ok_or_else(|| format!("No existe un runtime Java {major} para este sistema"))?
        .binary
        .package;
    let bytes = client
        .get(&package.link)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&bytes)) != package.checksum {
        return Err(format!("Hash inválido para Java {major}"));
    }
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let archive_path = runtime_root.join(&package.name);
    fs::write(&archive_path, &bytes).map_err(|e| e.to_string())?;
    if cfg!(target_os = "windows") {
        let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        archive.extract(&destination).map_err(|e| e.to_string())?;
    } else {
        let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&destination).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_file(archive_path);
    find_java(&destination)
        .ok_or_else(|| format!("Java {major} se descargó pero no contiene un ejecutable"))
}

fn rule_allows(value: &serde_json::Value) -> bool {
    let Some(rules) = value.get("rules").and_then(|item| item.as_array()) else {
        return true;
    };
    let current_os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let mut allowed = false;
    for rule in rules {
        let os_matches = rule
            .get("os")
            .and_then(|os| os.get("name"))
            .and_then(|name| name.as_str())
            .map(|name| name == current_os)
            .unwrap_or(true);
        let feature_matches = rule
            .get("features")
            .map(|features| {
                features
                    .as_object()
                    .map(|map| map.values().all(|value| value == false))
                    .unwrap_or(false)
            })
            .unwrap_or(true);
        if os_matches && feature_matches {
            allowed = rule.get("action").and_then(|item| item.as_str()) == Some("allow");
        }
    }
    allowed
}

fn argument_values(value: &serde_json::Value) -> Vec<String> {
    if let Some(text) = value.as_str() {
        return vec![text.to_string()];
    }
    if !rule_allows(value) {
        return Vec::new();
    }
    match value.get("value") {
        Some(item) if item.is_string() => vec![item.as_str().unwrap().to_string()],
        Some(item) => item
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    }
}

pub fn read_launch_metadata(version_json: &Path) -> Result<LaunchMetadata, String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(version_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let game_args = if let Some(args) = value
        .pointer("/arguments/game")
        .and_then(|item| item.as_array())
    {
        args.iter().flat_map(argument_values).collect()
    } else {
        shell_words::split(
            value
                .get("minecraftArguments")
                .and_then(|item| item.as_str())
                .unwrap_or(""),
        )
        .map_err(|e| e.to_string())?
    };
    let jvm_args = value
        .pointer("/arguments/jvm")
        .and_then(|item| item.as_array())
        .map(|args| args.iter().flat_map(argument_values).collect())
        .unwrap_or_default();
    Ok(LaunchMetadata {
        main_class: value
            .get("mainClass")
            .and_then(|item| item.as_str())
            .ok_or("La versión no define mainClass")?
            .to_string(),
        game_args,
        jvm_args,
        java_major: value
            .pointer("/javaVersion/majorVersion")
            .and_then(|item| item.as_u64())
            .unwrap_or(8) as u32,
    })
}

pub fn classpath(
    version_json: &Path,
    minecraft: &Path,
    client_jar: &Path,
) -> Result<Vec<PathBuf>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(version_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for library in value
        .get("libraries")
        .and_then(|item| item.as_array())
        .into_iter()
        .flatten()
    {
        if !rule_allows(library) {
            continue;
        }
        if let Some(path) = library
            .pointer("/downloads/artifact/path")
            .and_then(|item| item.as_str())
        {
            let full = minecraft.join("libraries").join(path);
            if full.exists() {
                entries.push(full);
            }
        }
    }
    entries.push(client_jar.to_path_buf());
    Ok(entries)
}

pub fn maven_library_path(root: &Path, name: &str) -> Option<PathBuf> {
    let parts: Vec<_> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let classifier = parts
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    Some(
        root.join(parts[0].replace('.', "/"))
            .join(parts[1])
            .join(parts[2])
            .join(format!("{}-{}{}.jar", parts[1], parts[2], classifier)),
    )
}

pub fn extract_natives(
    version_json: &Path,
    minecraft: &Path,
    destination: &Path,
) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(version_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let target = if cfg!(target_os = "windows") {
        "natives-windows"
    } else if cfg!(target_os = "macos") {
        "natives-macos"
    } else {
        "natives-linux"
    };
    let legacy_target = if cfg!(target_os = "macos") {
        "natives-osx"
    } else {
        target
    };
    for library in value
        .get("libraries")
        .and_then(|item| item.as_array())
        .into_iter()
        .flatten()
    {
        if !rule_allows(library) {
            continue;
        }
        let classifiers = library
            .pointer("/downloads/classifiers")
            .and_then(|item| item.as_object());
        let classified = classifiers
            .and_then(|items| {
                items
                    .iter()
                    .find(|(key, _)| key.as_str() == legacy_target)
                    .or_else(|| {
                        items.iter().find(|(key, _)| {
                            key.starts_with(legacy_target) && !key.contains("arm64")
                        })
                    })
            })
            .map(|(_, value)| value);
        let name = library
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or("");
        let modern_native = name.contains(&format!(":{target}"))
            && if cfg!(target_arch = "aarch64") {
                name.contains("arm64")
            } else {
                !name.contains("arm64") && !name.ends_with("-x86")
            };
        let artifact = classified.or_else(|| {
            if modern_native {
                library.pointer("/downloads/artifact")
            } else {
                None
            }
        });
        let Some(artifact) = artifact else { continue };
        let Some(path) = artifact.get("path").and_then(|item| item.as_str()) else {
            continue;
        };
        let file =
            fs::File::open(minecraft.join("libraries").join(path)).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            if entry.is_dir() || entry.name().starts_with("META-INF/") {
                continue;
            }
            let Some(name) = Path::new(entry.name()).file_name() else {
                continue;
            };
            let mut output = fs::File::create(destination.join(name)).map_err(|e| e.to_string())?;
            io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    loader: FabricLoaderVersion,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

#[derive(Clone, Deserialize)]
struct ModrinthFile {
    filename: String,
    url: String,
    hashes: HashMap<String, String>,
    primary: bool,
}

#[derive(Clone, Deserialize)]
struct ModrinthDependency {
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Clone, Deserialize)]
struct ModrinthVersion {
    version_type: String,
    files: Vec<ModrinthFile>,
    dependencies: Vec<ModrinthDependency>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ManagedModsManifest {
    minecraft_version: String,
    generated_at: u64,
    files: Vec<ManagedModEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedModEntry {
    project_id: String,
    filename: String,
    sha1: String,
}

pub async fn compatible_fabric_loader(
    version: &str,
    client: &reqwest::Client,
) -> Result<Option<String>, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{version}");
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let loaders: Vec<FabricLoaderInfo> = response.json().await.map_err(|e| e.to_string())?;
    Ok(loaders
        .iter()
        .find(|item| item.loader.stable)
        .or_else(|| loaders.first())
        .map(|item| item.loader.version.clone()))
}

pub async fn resolve_modrinth_mods(
    version: &str,
    mods: &Path,
    client: &reqwest::Client,
) -> Result<(usize, Vec<String>), String> {
    const PROJECTS: &[&str] = &[
        "fabric-api",
        "sodium",
        "lithium",
        "immediatelyfast",
        "ferrite-core",
        "entityculling",
        "moreculling",
        "krypton",
        "badoptimizations",
        "dynamic-fps",
        "sodium-extra",
        "reeses-sodium-options",
        "zoomify",
        "modmenu",
        "betterf3",
        "appleskin",
        "fast-ip-ping",
        "debugify",
        "fpsflow",
        "hudfabric",
        "shulkerboxtooltip",
        "status-effect-bars",
        "inventory-profiles-next",
        "mouse-tweaks",
        "xaeros-minimap",
        "better-mount-hud",
        "item-counter-fx",
        "simple-voice-chat",
    ];
    install_modrinth_projects(
        version,
        mods,
        &PROJECTS
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        client,
        true,
    )
    .await
}

pub async fn install_modrinth_projects(
    version: &str,
    mods: &Path,
    projects: &[String],
    client: &reqwest::Client,
    replace_existing: bool,
) -> Result<(usize, Vec<String>), String> {
    fs::create_dir_all(mods).map_err(|e| e.to_string())?;
    let mut queue: VecDeque<String> = projects.iter().cloned().collect();
    let mut visited = HashSet::new();
    let mut selected: Vec<(String, ModrinthFile)> = Vec::new();
    let mut unavailable = Vec::new();
    while let Some(project) = queue.pop_front() {
        if !visited.insert(project.clone()) {
            continue;
        }
        let url = format!("https://api.modrinth.com/v2/project/{project}/version?loaders=%5B%22fabric%22%5D&game_versions=%5B%22{version}%22%5D&include_changelog=false");
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            unavailable.push(project);
            continue;
        }
        let versions: Vec<ModrinthVersion> = response.json().await.map_err(|e| e.to_string())?;
        let chosen = versions
            .iter()
            .find(|item| item.version_type == "release")
            .or_else(|| versions.iter().find(|item| item.version_type == "beta"))
            .or_else(|| versions.first());
        let Some(chosen) = chosen else {
            unavailable.push(project);
            continue;
        };
        for dependency in &chosen.dependencies {
            if dependency.dependency_type == "required" {
                if let Some(id) = &dependency.project_id {
                    queue.push_back(id.clone());
                }
            }
        }
        let file = chosen
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| chosen.files.first())
            .ok_or_else(|| format!("{project} no publicó un archivo descargable"))?;
        validate_download_filename(&file.filename)?;
        selected.push((project, file.clone()));
    }
    let wanted: HashSet<_> = selected
        .iter()
        .map(|(_, file)| file.filename.as_str())
        .collect();
    let manifest_path = mods.join(".aureus-mods.json");
    let previous = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ManagedModsManifest>(&bytes).ok())
        .unwrap_or_default();
    let previously_managed: HashSet<String> = previous
        .files
        .iter()
        .map(|entry| entry.filename.clone())
        .collect();
    for (_, file) in &selected {
        let destination = mods.join(&file.filename);
        if destination.exists() && !previously_managed.contains(&file.filename) {
            return Err(format!(
                "El mod {} ya existe y no es administrado por Aureus; se conservó sin cambios",
                file.filename
            ));
        }
    }
    let stage = mods.join(".aureus-stage");
    if stage.exists() {
        fs::remove_dir_all(&stage).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let downloads = selected.clone();
    let results = stream::iter(downloads.into_iter().map(|(_, file)| {
        let client = client.clone();
        let destination = stage.join(&file.filename);
        let sha1 = file.hashes.get("sha1").cloned().unwrap_or_default();
        let url = file.url.clone();
        async move { verified_download(&client, &url, &sha1, &destination).await }
    }))
    .buffer_unordered(6)
    .collect::<Vec<_>>()
    .await;
    let mut count = 0;
    for result in results {
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
        count += 1;
    }
    let rollback = mods.join(".aureus-rollback");
    if rollback.exists() {
        fs::remove_dir_all(&rollback).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&rollback).map_err(|e| e.to_string())?;
    let mut affected = previously_managed.clone();
    affected.extend(selected.iter().map(|(_, file)| file.filename.clone()));
    for filename in affected {
        let should_remove = replace_existing || wanted.contains(filename.as_str());
        let path = mods.join(&filename);
        if should_remove && path.exists() {
            if let Err(error) = replace_file(&path, &rollback.join(&filename)) {
                let rollback_error = restore_transaction(mods, &rollback, &[]).err();
                let _ = fs::remove_dir_all(&stage);
                return Err(match rollback_error {
                    Some(rollback_error) => {
                        format!("{error}; además falló la reversión: {rollback_error}")
                    }
                    None => error,
                });
            }
        }
    }
    let mut installed = Vec::new();
    for (_, file) in &selected {
        if let Err(error) = replace_file(&stage.join(&file.filename), &mods.join(&file.filename)) {
            let rollback_error = restore_transaction(mods, &rollback, &installed).err();
            let _ = fs::remove_dir_all(&stage);
            return Err(match rollback_error {
                Some(rollback_error) => {
                    format!("{error}; además falló la reversión: {rollback_error}")
                }
                None => error,
            });
        }
        installed.push(file.filename.clone());
    }
    let manifest = ManagedModsManifest {
        minecraft_version: version.into(),
        generated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        files: selected
            .into_iter()
            .map(|(project_id, file)| ManagedModEntry {
                project_id,
                filename: file.filename,
                sha1: file.hashes.get("sha1").cloned().unwrap_or_default(),
            })
            .collect(),
    };
    let manifest_temp = manifest_path.with_extension("json.tmp");
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    if let Err(error) = fs::write(&manifest_temp, manifest_bytes)
        .map_err(|e| e.to_string())
        .and_then(|_| replace_file(&manifest_temp, &manifest_path))
    {
        let rollback_error = restore_transaction(mods, &rollback, &installed).err();
        let _ = fs::remove_dir_all(&stage);
        return Err(match rollback_error {
            Some(rollback_error) => format!("{error}; además falló la reversión: {rollback_error}"),
            None => error,
        });
    }
    let _ = fs::remove_dir_all(&stage);
    let _ = fs::remove_dir_all(&rollback);
    Ok((count, unavailable))
}

fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

async fn verified_download(
    client: &reqwest::Client,
    url: &str,
    sha1: &str,
    destination: &Path,
) -> Result<bool, String> {
    if let Ok(existing) = fs::read(destination) {
        if sha1.is_empty() || sha1_hex(&existing) == sha1 {
            return Ok(false);
        }
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = destination.with_extension("download");
    let partial_size = fs::metadata(&temporary)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut request = client.get(url);
    if partial_size > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={partial_size}-"));
    }
    let response = request
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let resumed = partial_size > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let remaining = response.content_length().unwrap_or(0);
    tracker().total.fetch_add(
        if resumed {
            partial_size + remaining
        } else {
            remaining
        },
        Ordering::Relaxed,
    );
    if resumed {
        tracker()
            .downloaded
            .fetch_add(partial_size, Ordering::Relaxed);
    }
    let mut output = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        output.write_all(&chunk).map_err(|e| e.to_string())?;
        tracker()
            .downloaded
            .fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    output.flush().map_err(|e| e.to_string())?;
    let bytes = fs::read(&temporary).map_err(|e| e.to_string())?;
    if !sha1.is_empty() && sha1_hex(&bytes) != sha1 {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Hash inválido al descargar {url}"));
    }
    replace_file(&temporary, destination)?;
    Ok(true)
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if destination.exists() {
        fs::remove_file(destination).map_err(|e| e.to_string())?;
    }
    fs::rename(source, destination).map_err(|e| e.to_string())
}

fn validate_download_filename(filename: &str) -> Result<(), String> {
    let mut components = Path::new(filename).components();
    let valid = matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
        && !filename.contains(['/', '\\'])
        && !filename.starts_with('.')
        && filename.to_ascii_lowercase().ends_with(".jar");
    if valid {
        Ok(())
    } else {
        Err(format!("Nombre de archivo de mod no válido: {filename}"))
    }
}

fn restore_transaction(mods: &Path, rollback: &Path, installed: &[String]) -> Result<(), String> {
    for filename in installed {
        let path = mods.join(filename);
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    if rollback.exists() {
        for entry in fs::read_dir(rollback)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            replace_file(&entry.path(), &mods.join(entry.file_name()))?;
        }
        fs::remove_dir_all(rollback).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn install_official_version(
    version: &str,
    minecraft: &Path,
    client: &reqwest::Client,
) -> Result<InstalledVersion, String> {
    let manifest: Manifest = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|item| item.id == version)
        .ok_or_else(|| format!("Minecraft {version} no existe en el catálogo oficial"))?;
    let version_dir = minecraft.join("versions").join(version);
    let version_json = version_dir.join(format!("{version}.json"));
    verified_download(client, &entry.url, &entry.sha1, &version_json).await?;
    let meta: VersionMeta =
        serde_json::from_slice(&fs::read(&version_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let client_jar = version_dir.join(format!("{version}.jar"));
    let mut downloaded = usize::from(
        verified_download(
            client,
            &meta.downloads.client.url,
            &meta.downloads.client.sha1,
            &client_jar,
        )
        .await?,
    );

    let mut library_files = Vec::new();
    for library in meta.libraries {
        let Some(downloads) = library.downloads else {
            continue;
        };
        if let Some(file) = downloads.artifact {
            library_files.push(file);
        }
        if let Some(classifiers) = downloads.classifiers {
            library_files.extend(classifiers.into_values());
        }
    }
    let library_root = minecraft.join("libraries");
    let results = stream::iter(library_files.into_iter().map(|file| {
        let client = client.clone();
        let destination = library_root.join(&file.path);
        async move { verified_download(&client, &file.url, &file.sha1, &destination).await }
    }))
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;
    for result in results {
        downloaded += usize::from(result?);
    }

    let index_path = minecraft
        .join("assets/indexes")
        .join(format!("{}.json", meta.asset_index.id));
    downloaded += usize::from(
        verified_download(
            client,
            &meta.asset_index.url,
            &meta.asset_index.sha1,
            &index_path,
        )
        .await?,
    );
    let index: AssetIndex =
        serde_json::from_slice(&fs::read(&index_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let virtual_assets = index.r#virtual;
    let map_to_resources = index.map_to_resources;
    let logical_objects = index.objects.clone();
    let object_root = minecraft.join("assets/objects");
    let results = stream::iter(index.objects.into_values().map(|object| {
        let client = client.clone();
        let prefix = &object.hash[..2];
        let url = format!(
            "https://resources.download.minecraft.net/{prefix}/{}",
            object.hash
        );
        let destination = object_root.join(prefix).join(&object.hash);
        let hash = object.hash;
        async move { verified_download(&client, &url, &hash, &destination).await }
    }))
    .buffer_unordered(16)
    .collect::<Vec<_>>()
    .await;
    for result in results {
        downloaded += usize::from(result?);
    }
    if virtual_assets || map_to_resources {
        for (logical_name, object) in logical_objects {
            let source = object_root.join(&object.hash[..2]).join(&object.hash);
            let destination = if virtual_assets {
                minecraft
                    .join("assets/virtual")
                    .join(&meta.asset_index.id)
                    .join(&logical_name)
            } else {
                minecraft.join("resources").join(&logical_name)
            };
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            if !destination.exists() {
                fs::copy(source, destination).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(InstalledVersion {
        version_json,
        client_jar,
        asset_index_id: meta.asset_index.id,
        downloaded_files: downloaded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aureus-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn mod_filenames_cannot_escape_the_instance() {
        assert!(validate_download_filename("sodium-1.0.jar").is_ok());
        assert!(validate_download_filename("../outside.jar").is_err());
        assert!(validate_download_filename("folder/mod.jar").is_err());
        assert!(validate_download_filename("folder\\mod.jar").is_err());
        assert!(validate_download_filename(".hidden.jar").is_err());
        assert!(validate_download_filename("readme.txt").is_err());
    }

    #[test]
    fn failed_mod_transaction_restores_previous_files() {
        let root = test_directory("rollback");
        let mods = root.join("mods");
        let rollback = mods.join(".aureus-rollback");
        fs::create_dir_all(&rollback).unwrap();
        fs::write(mods.join("new.jar"), b"new").unwrap();
        fs::write(rollback.join("old.jar"), b"old").unwrap();

        restore_transaction(&mods, &rollback, &["new.jar".into()]).unwrap();

        assert!(!mods.join("new.jar").exists());
        assert_eq!(fs::read(mods.join("old.jar")).unwrap(), b"old");
        assert!(!rollback.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maven_coordinates_use_platform_neutral_paths() {
        let root = Path::new("libraries");
        let path = maven_library_path(root, "org.example:demo:1.2.3:all").unwrap();
        assert_eq!(path, root.join("org/example/demo/1.2.3/demo-1.2.3-all.jar"));
        assert!(maven_library_path(root, "invalid").is_none());
    }
}
