use futures_util::{stream, StreamExt};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::io;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

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
    let mut selected = Vec::new();
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
        selected.push(file.clone());
    }
    let wanted: HashSet<_> = selected.iter().map(|file| file.filename.as_str()).collect();
    if replace_existing {
        for entry in fs::read_dir(mods)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("jar")
                && !wanted.contains(
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or(""),
                )
            {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
    }
    let results = stream::iter(selected.into_iter().map(|file| {
        let client = client.clone();
        let destination = mods.join(&file.filename);
        let sha1 = file.hashes.get("sha1").cloned().unwrap_or_default();
        async move { verified_download(&client, &file.url, &sha1, &destination).await }
    }))
    .buffer_unordered(6)
    .collect::<Vec<_>>()
    .await;
    let mut count = 0;
    for result in results {
        result?;
        count += 1;
    }
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
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if !sha1.is_empty() && sha1_hex(&bytes) != sha1 {
        return Err(format!("Hash inválido al descargar {url}"));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = destination.with_extension("download");
    fs::write(&temporary, &bytes).map_err(|e| e.to_string())?;
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
