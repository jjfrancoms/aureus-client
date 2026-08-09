use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedInstance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub memory_mb: u32,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub java_path: Option<String>,
    pub jvm_args: Vec<String>,
    pub profile: String,
    pub created_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEntry {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub size: u64,
    pub sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareProfile {
    pub memory_mb: u64,
    pub cpu_threads: usize,
    pub recommended_memory_mb: u32,
    pub profile: String,
    pub render_distance: u32,
    pub simulation_distance: u32,
    pub gpu_name: String,
    pub on_battery: bool,
    pub operating_system: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    pub detected: bool,
    pub summary: String,
    pub suspected_mods: Vec<String>,
    pub actions: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkReport {
    pub available: bool,
    pub average_fps: u64,
    pub one_percent_low: u64,
    pub memory_used_mb: u64,
    pub samples: u64,
    pub recommendation: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub path: String,
    pub created_at: u64,
    pub size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub valid: bool,
    pub verified_files: usize,
    pub missing_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub unmanaged_files: Vec<String>,
    pub summary: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTelemetry {
    pub process_cpu_percent: f64,
    pub process_memory_mb: u64,
    pub gpu_name: String,
    pub on_battery: bool,
    pub thermal_state: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModImpactReport {
    pub mod_count: usize,
    pub total_size_mb: f64,
    pub likely_visual_cost: Vec<String>,
    pub large_files: Vec<String>,
    pub recommendation: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableProfile {
    schema_version: u32,
    name: String,
    profile: String,
    memory_mb: u32,
    resolution_width: u32,
    resolution_height: u32,
    jvm_args: Vec<String>,
    client_config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct ModrinthHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
}

#[derive(Deserialize)]
struct ModrinthSearch {
    hits: Vec<ModrinthHit>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn data_root() -> Result<PathBuf, String> {
    let path = dirs::config_dir()
        .ok_or("No se localizó la carpeta de configuración")?
        .join("Aureus Launcher");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
fn instances_file() -> Result<PathBuf, String> {
    Ok(data_root()?.join("managed-instances.json"))
}
fn instance_root(id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(data_root()?.join("instances-data").join(id))
}
fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        Err("Identificador de instancia no válido".into())
    } else {
        Ok(())
    }
}
fn read_instances() -> Result<Vec<ManagedInstance>, String> {
    let path = instances_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write_instances(items: &[ManagedInstance]) -> Result<(), String> {
    let path = instances_file()?;
    let temp = path.with_extension("tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(items).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace_file(&temp, &path)
}
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if destination.exists() {
        fs::remove_file(destination).map_err(|e| e.to_string())?;
    }
    fs::rename(source, destination).map_err(|e| e.to_string())
}
fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let target = destination.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn managed_instances() -> Result<Vec<ManagedInstance>, String> {
    read_instances()
}

pub fn register_prepared_instance(
    id: &str,
    version: &str,
    java_path: Option<String>,
) -> Result<(), String> {
    let existing = read_instances()?.into_iter().find(|item| item.id == id);
    let instance = existing.unwrap_or(ManagedInstance {
        id: id.into(),
        name: format!("Minecraft {version}"),
        minecraft_version: version.into(),
        memory_mb: 5120,
        resolution_width: 1280,
        resolution_height: 720,
        java_path: None,
        jvm_args: vec!["-XX:+UseG1GC".into(), "-XX:+ParallelRefProcEnabled".into()],
        profile: "CUSTOM".into(),
        created_at: now(),
    });
    upsert_managed_instance(ManagedInstance {
        java_path,
        ..instance
    })
    .map(|_| ())
}

pub fn launch_preferences(id: &str) -> Option<(u32, u32, u32, Vec<String>)> {
    read_instances()
        .ok()?
        .into_iter()
        .find(|item| item.id == id)
        .map(|item| {
            (
                item.memory_mb,
                item.resolution_width,
                item.resolution_height,
                item.jvm_args,
            )
        })
}

#[tauri::command]
pub fn upsert_managed_instance(mut instance: ManagedInstance) -> Result<ManagedInstance, String> {
    validate_id(&instance.id)?;
    validate_id(&instance.minecraft_version)
        .map_err(|_| "Versión de Minecraft no válida".to_string())?;
    if instance.name.trim().is_empty() {
        return Err("La instancia necesita un nombre".into());
    }
    instance.memory_mb = instance.memory_mb.clamp(1024, 32768);
    instance.resolution_width = instance.resolution_width.clamp(640, 7680);
    instance.resolution_height = instance.resolution_height.clamp(480, 4320);
    if instance.created_at == 0 {
        instance.created_at = now();
    }
    let mut items = read_instances()?;
    if let Some(existing) = items.iter_mut().find(|item| item.id == instance.id) {
        *existing = instance.clone();
    } else {
        items.push(instance.clone());
    }
    write_instances(&items)?;
    fs::create_dir_all(instance_root(&instance.id)?).map_err(|e| e.to_string())?;
    Ok(instance)
}

#[tauri::command]
pub fn duplicate_managed_instance(
    source_id: String,
    new_id: String,
    new_name: String,
) -> Result<ManagedInstance, String> {
    validate_id(&source_id)?;
    validate_id(&new_id)?;
    let mut items = read_instances()?;
    if items.iter().any(|item| item.id == new_id) {
        return Err("Ya existe una instancia con ese identificador".into());
    }
    let mut clone = items
        .iter()
        .find(|item| item.id == source_id)
        .cloned()
        .ok_or("No existe la instancia original")?;
    clone.id = new_id.clone();
    clone.name = new_name;
    clone.created_at = now();
    let source = instance_root(&source_id)?;
    let destination = instance_root(&new_id)?;
    if source.exists() {
        copy_tree(&source, &destination)?;
    } else {
        fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    }
    let descriptor = destination.join("aureus-instance.json");
    if descriptor.exists() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&descriptor).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        value["gameDirectory"] = serde_json::Value::String(destination.display().to_string());
        fs::write(
            &descriptor,
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    items.push(clone.clone());
    write_instances(&items)?;
    Ok(clone)
}

#[tauri::command]
pub fn delete_managed_instance(id: String, delete_files: bool) -> Result<String, String> {
    validate_id(&id)?;
    let mut items = read_instances()?;
    let before = items.len();
    items.retain(|item| item.id != id);
    if items.len() == before {
        return Err("No existe esa instancia".into());
    }
    write_instances(&items)?;
    if delete_files {
        let root = instance_root(&id)?;
        if root.exists() {
            fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        }
    }
    Ok("Instancia eliminada".into())
}

fn content_directory(instance_id: &str, kind: &str) -> Result<PathBuf, String> {
    let folder = match kind {
        "mod" => "mods",
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        "datapack" => "datapacks",
        _ => return Err("Tipo de contenido no válido".into()),
    };
    Ok(instance_root(instance_id)?.join(folder))
}

#[tauri::command]
pub fn list_instance_content(instance_id: String) -> Result<Vec<ContentEntry>, String> {
    let mut output = Vec::new();
    for (kind, folder) in [
        ("mod", "mods"),
        ("resourcepack", "resourcepacks"),
        ("shader", "shaderpacks"),
        ("datapack", "datapacks"),
    ] {
        let root = instance_root(&instance_id)?.join(folder);
        if !root.exists() {
            continue;
        }
        for entry in fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            if !entry.path().is_file() {
                continue;
            }
            let bytes = fs::read(entry.path()).map_err(|e| e.to_string())?;
            let raw = entry.file_name().to_string_lossy().into_owned();
            let enabled = !raw.ends_with(".disabled") && !raw.ends_with(".profile-disabled");
            output.push(ContentEntry {
                name: raw,
                kind: kind.into(),
                enabled,
                size: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            });
        }
    }
    output.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
    Ok(output)
}

#[tauri::command]
pub fn verify_instance_integrity(instance_id: String) -> Result<IntegrityReport, String> {
    let mods = instance_root(&instance_id)?.join("mods");
    let manifest_path = mods.join(".aureus-mods.json");
    if !manifest_path.exists() {
        return Ok(IntegrityReport {
            valid: true,
            verified_files: 0,
            missing_files: Vec::new(),
            modified_files: Vec::new(),
            unmanaged_files: Vec::new(),
            summary: "La instancia aún no tiene un manifiesto administrado por Aureus".into(),
        });
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Manifiesto de mods inválido: {e}"))?;
    let files = manifest["files"]
        .as_array()
        .ok_or("El manifiesto no contiene archivos")?;
    let mut expected = std::collections::HashSet::new();
    let mut missing = Vec::new();
    let mut modified = Vec::new();
    let mut verified = 0;
    for item in files {
        let Some(name) = item["filename"].as_str() else {
            continue;
        };
        expected.insert(name.to_string());
        let path = mods.join(name);
        if !path.exists() {
            missing.push(name.into());
            continue;
        }
        let expected_sha1 = item["sha1"].as_str().unwrap_or("");
        let actual = format!(
            "{:x}",
            sha1::Sha1::digest(&fs::read(path).map_err(|e| e.to_string())?)
        );
        if !expected_sha1.is_empty() && actual != expected_sha1 {
            modified.push(name.into());
        } else {
            verified += 1;
        }
    }
    let mut unmanaged = Vec::new();
    if mods.exists() {
        for entry in fs::read_dir(&mods)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().extension().and_then(|value| value.to_str()) == Some("jar")
                && !expected.contains(&name)
            {
                unmanaged.push(name);
            }
        }
    }
    let valid = missing.is_empty() && modified.is_empty();
    let summary = if valid {
        format!("{verified} archivos administrados verificados")
    } else {
        format!(
            "{} ausentes y {} modificados",
            missing.len(),
            modified.len()
        )
    };
    Ok(IntegrityReport {
        valid,
        verified_files: verified,
        missing_files: missing,
        modified_files: modified,
        unmanaged_files: unmanaged,
        summary,
    })
}

#[tauri::command]
pub fn toggle_instance_content(
    instance_id: String,
    kind: String,
    name: String,
    enabled: bool,
) -> Result<String, String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Nombre de archivo no válido".into());
    }
    let root = content_directory(&instance_id, &kind)?;
    let current = root.join(&name);
    let target = if enabled {
        root.join(
            name.strip_suffix(".disabled")
                .or_else(|| name.strip_suffix(".profile-disabled"))
                .unwrap_or(&name),
        )
    } else if name.ends_with(".disabled") || name.ends_with(".profile-disabled") {
        current.clone()
    } else {
        root.join(format!("{name}.disabled"))
    };
    if current != target {
        fs::rename(current, &target).map_err(|e| e.to_string())?;
    }
    Ok(target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub fn create_instance_backup(instance_id: String) -> Result<String, String> {
    let source = instance_root(&instance_id)?;
    if !source.exists() {
        return Err("La instancia no está preparada".into());
    }
    let backup = data_root()?
        .join("backups")
        .join(format!("{}-{}", instance_id, now()));
    copy_tree(&source, &backup)?;
    Ok(backup.display().to_string())
}

fn tree_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| tree_size(&entry.path()))
        .sum()
}

#[tauri::command]
pub fn list_instance_backups(instance_id: String) -> Result<Vec<BackupEntry>, String> {
    validate_id(&instance_id)?;
    let root = data_root()?.join("backups");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let prefix = format!("{instance_id}-");
    let mut backups = fs::read_dir(root)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let created_at = name.strip_prefix(&prefix)?.parse::<u64>().ok()?;
            Some(BackupEntry {
                path: entry.path().display().to_string(),
                created_at,
                size: tree_size(&entry.path()),
            })
        })
        .collect::<Vec<_>>();
    backups.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(backups)
}

#[tauri::command]
pub fn restore_instance_backup(instance_id: String, backup_path: String) -> Result<String, String> {
    let backup = PathBuf::from(backup_path);
    let backup_root = data_root()?.join("backups");
    if !backup.starts_with(&backup_root) || !backup.is_dir() {
        return Err("Respaldo no válido".into());
    }
    let destination = instance_root(&instance_id)?;
    if destination.exists() {
        create_instance_backup(instance_id.clone())?;
        fs::remove_dir_all(&destination).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    copy_tree(&backup, &destination)?;
    Ok("Respaldo restaurado; se guardó el estado anterior para poder deshacer".into())
}

fn zip_tree(
    writer: &mut zip::ZipWriter<fs::File>,
    root: &Path,
    directory: &Path,
) -> Result<(), String> {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for entry in fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            writer
                .add_directory(format!("{relative}/"), options)
                .map_err(|e| e.to_string())?;
            zip_tree(writer, root, &path)?;
        } else {
            writer
                .start_file(relative, options)
                .map_err(|e| e.to_string())?;
            let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, writer).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_instance(instance_id: String) -> Result<String, String> {
    let source = instance_root(&instance_id)?;
    if !source.exists() {
        return Err("La instancia no está preparada".into());
    }
    let exports = data_root()?.join("exports");
    fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    let output = exports.join(format!("{instance_id}-{}.aureuspack", now()));
    let file = fs::File::create(&output).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(file);
    zip_tree(&mut writer, &source, &source)?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(output.display().to_string())
}

#[tauri::command]
pub fn import_instance(
    archive_path: String,
    new_id: String,
    version: String,
) -> Result<ManagedInstance, String> {
    validate_id(&new_id)?;
    validate_id(&version).map_err(|_| "Versión de Minecraft no válida".to_string())?;
    let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let destination = instance_root(&new_id)?;
    if destination.exists() {
        return Err("Ya existe una instancia con ese identificador".into());
    }
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let extraction = (|| -> Result<(), String> {
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let Some(relative) = entry.enclosed_name() else {
                return Err("El paquete contiene rutas inseguras".into());
            };
            let output = destination.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(output).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut target = fs::File::create(output).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut target).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })();
    if let Err(error) = extraction {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }
    let imported = upsert_managed_instance(ManagedInstance {
        id: new_id.clone(),
        name: format!("Importada {new_id}"),
        minecraft_version: version,
        memory_mb: 5120,
        resolution_width: 1280,
        resolution_height: 720,
        java_path: None,
        jvm_args: Vec::new(),
        profile: "CUSTOM".into(),
        created_at: now(),
    });
    if imported.is_err() {
        let _ = fs::remove_dir_all(&destination);
    }
    imported
}

#[tauri::command]
pub fn export_profile(instance_id: String) -> Result<String, String> {
    let instance = read_instances()?
        .into_iter()
        .find(|item| item.id == instance_id)
        .ok_or("No existe la instancia seleccionada")?;
    let config_path = instance_root(&instance.id)?
        .join("config")
        .join("aureus-client.json");
    let client_config = fs::read(&config_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let profile = PortableProfile {
        schema_version: 1,
        name: instance.name.clone(),
        profile: instance.profile.clone(),
        memory_mb: instance.memory_mb,
        resolution_width: instance.resolution_width,
        resolution_height: instance.resolution_height,
        jvm_args: instance.jvm_args.clone(),
        client_config,
    };
    let exports = data_root()?.join("exports");
    fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    let output = exports.join(format!("{}-{}.aureusprofile", instance.id, now()));
    fs::write(
        &output,
        serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(output.display().to_string())
}

#[tauri::command]
pub fn import_profile(
    instance_id: String,
    profile_path: String,
) -> Result<ManagedInstance, String> {
    let portable: PortableProfile =
        serde_json::from_slice(&fs::read(profile_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Perfil no válido: {e}"))?;
    if portable.schema_version != 1 {
        return Err("Versión de perfil no compatible".into());
    }
    let mut instances = read_instances()?;
    let instance = instances
        .iter_mut()
        .find(|item| item.id == instance_id)
        .ok_or("No existe la instancia")?;
    instance.profile = portable.profile;
    instance.memory_mb = portable.memory_mb.clamp(1024, 32768);
    instance.resolution_width = portable.resolution_width.clamp(640, 7680);
    instance.resolution_height = portable.resolution_height.clamp(480, 4320);
    instance.jvm_args = portable.jvm_args;
    let updated = instance.clone();
    write_instances(&instances)?;
    let config = instance_root(&instance_id)?
        .join("config")
        .join("aureus-client.json");
    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if config.exists() {
        create_instance_backup(instance_id)?;
    }
    let temp = config.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&portable.client_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace_file(&temp, &config)?;
    Ok(updated)
}

#[tauri::command]
pub fn recommend_hardware_profile() -> HardwareProfile {
    let memory_mb = detect_memory_mb();
    let cpu_threads = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(4);
    let on_battery = detect_on_battery();
    let gpu_name = detect_gpu_name();
    let (profile, memory, render, simulation) = if on_battery {
        ("BATERIA", 4096, 6, 5)
    } else if memory_mb < 8192 {
        ("RAM_MINIMA", 3072, 6, 5)
    } else if memory_mb < 16384 || cpu_threads < 8 {
        ("EQUILIBRADO", 5120, 8, 6)
    } else {
        ("COMPETITIVO", 6144, 12, 8)
    };
    HardwareProfile {
        memory_mb,
        cpu_threads,
        recommended_memory_mb: memory,
        profile: profile.into(),
        render_distance: render,
        simulation_distance: simulation,
        gpu_name,
        on_battery,
        operating_system: std::env::consts::OS.into(),
    }
}

fn detect_gpu_name() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-detailLevel", "mini"])
            .output()
        {
            if let Some(value) = String::from_utf8_lossy(&out.stdout)
                .lines()
                .find_map(|line| line.trim().strip_prefix("Chipset Model:").map(str::trim))
            {
                return value.into();
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = std::process::Command::new("powershell");
        command.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "(Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name)"]).output()
        { let value = String::from_utf8_lossy(&out.stdout).trim().to_string(); if !value.is_empty() { return value; } }
    }
    "GPU detectada por el sistema".into()
}

fn detect_on_battery() -> bool {
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
    {
        return String::from_utf8_lossy(&out.stdout).contains("Battery Power");
    }
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = std::process::Command::new("powershell");
        command.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$b=Get-CimInstance Win32_Battery; if($b){[int]($b.BatteryStatus -eq 1)}else{0}",
            ])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout).trim() == "1";
        }
    }
    false
}

#[tauri::command]
pub fn system_telemetry(pid: Option<u32>) -> SystemTelemetry {
    let mut cpu = 0.0;
    let mut memory_mb = 0;
    let mut thermal_state = "Normal".to_string();
    #[cfg(target_os = "macos")]
    {
        if let Some(pid) = pid {
            if let Ok(out) = std::process::Command::new("ps")
                .args(["-o", "%cpu=,rss=", "-p", &pid.to_string()])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                let mut values = text.split_whitespace();
                cpu = values.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                memory_mb = values
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
                    / 1024;
            }
        }
        if let Ok(out) = std::process::Command::new("pmset")
            .args(["-g", "therm"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("CPU_Speed_Limit") && !text.contains("CPU_Speed_Limit = 100") {
                thermal_state = "Limitación térmica".into();
            }
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(pid) = pid {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = std::process::Command::new("powershell");
        command.creation_flags(CREATE_NO_WINDOW);
        let script = format!("$p=Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object {{$_.IDProcess -eq {pid}}} | Select-Object -First 1; if($p){{Write-Output ($p.PercentProcessorTime.ToString() + ' ' + [math]::Round($p.WorkingSetPrivate/1MB).ToString())}}");
        if let Ok(out) = command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut values = text.split_whitespace();
            cpu = values.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            memory_mb = values.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    SystemTelemetry {
        process_cpu_percent: cpu,
        process_memory_mb: memory_mb,
        gpu_name: detect_gpu_name(),
        on_battery: detect_on_battery(),
        thermal_state,
    }
}

fn remove_temporary_files(root: &Path) -> Result<u64, String> {
    if !root.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() == ".aureus-stage" {
                removed += tree_size(&path);
                fs::remove_dir_all(path).map_err(|e| e.to_string())?;
            } else {
                removed += remove_temporary_files(&path)?;
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some("download") {
            removed += path.metadata().map(|value| value.len()).unwrap_or(0);
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(removed)
}

#[tauri::command]
pub fn clean_safe_caches(instance_id: String) -> Result<String, String> {
    let mut bytes = remove_temporary_files(&instance_root(&instance_id)?)?;
    let cache = data_root()?.join("cache");
    if cache.exists() {
        bytes += tree_size(&cache);
        fs::remove_dir_all(cache).map_err(|e| e.to_string())?;
    }
    Ok(format!(
        "Caché temporal limpiada: {:.1} MB",
        bytes as f64 / 1_048_576.0
    ))
}

#[tauri::command]
pub fn apply_profile_content_policy(
    instance_id: String,
    profile: String,
) -> Result<String, String> {
    let root = instance_root(&instance_id)?;
    let mods = root.join("mods");
    let shaders = root.join("shaderpacks");
    let lightweight = matches!(
        profile.to_ascii_uppercase().as_str(),
        "COMPETITIVE"
            | "COMPETITIVO"
            | "MAX"
            | "MAX_FPS"
            | "MEMORY_SAVER"
            | "RAM_MINIMA"
            | "BATTERY"
            | "BATERIA"
    );
    let expensive = [
        "xaero",
        "minimap",
        "lambdynamiclights",
        "dynamiclights",
        "capes",
        "iris",
        "continuity",
    ];
    let mut changes: Vec<(PathBuf, PathBuf)> = Vec::new();
    for directory in [&mods, &shaders] {
        if !directory.exists() {
            continue;
        }
        for entry in fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if lightweight {
                let costly =
                    directory == &shaders || expensive.iter().any(|needle| name.contains(needle));
                if costly && !name.ends_with(".profile-disabled") {
                    changes.push((
                        path.clone(),
                        PathBuf::from(format!("{}.profile-disabled", path.display())),
                    ));
                }
            } else if name.ends_with(".profile-disabled") {
                let restored =
                    PathBuf::from(path.to_string_lossy().trim_end_matches(".profile-disabled"));
                changes.push((path, restored));
            }
        }
    }
    if changes.is_empty() {
        return Ok("El contenido ya coincide con el perfil".into());
    }
    create_instance_backup(instance_id)?;
    for (source, destination) in &changes {
        fs::rename(source, destination).map_err(|e| e.to_string())?;
    }
    Ok(format!(
        "Perfil aplicado: {} elementos visuales ajustados",
        changes.len()
    ))
}

#[tauri::command]
pub fn analyze_mod_impact(instance_id: String) -> Result<ModImpactReport, String> {
    let mods = instance_root(&instance_id)?.join("mods");
    if !mods.exists() {
        return Ok(ModImpactReport {
            mod_count: 0,
            total_size_mb: 0.0,
            likely_visual_cost: Vec::new(),
            large_files: Vec::new(),
            recommendation: "No hay mods instalados".into(),
        });
    }
    let visual_markers = [
        "shader",
        "iris",
        "minimap",
        "dynamiclight",
        "continuity",
        "capes",
        "replay",
    ];
    let mut count = 0;
    let mut total = 0;
    let mut visual = Vec::new();
    let mut large = Vec::new();
    for entry in fs::read_dir(mods)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jar") {
            continue;
        }
        count += 1;
        let size = path.metadata().map(|value| value.len()).unwrap_or(0);
        total += size;
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        if visual_markers.iter().any(|marker| lower.contains(marker)) {
            visual.push(name.clone());
        }
        if size > 12 * 1_048_576 {
            large.push(name);
        }
    }
    let recommendation = if visual.len() > 3 {
        "El perfil competitivo puede desactivar temporalmente varios mods visuales"
    } else if count > 45 {
        "La instancia contiene muchos mods; compara el benchmark con modo seguro"
    } else if !large.is_empty() {
        "Hay archivos grandes; no implican un problema, pero pueden aumentar el tiempo de inicio"
    } else {
        "No se detectó una carga evidente; confirma con el benchmark A/B"
    };
    Ok(ModImpactReport {
        mod_count: count,
        total_size_mb: total as f64 / 1_048_576.0,
        likely_visual_cost: visual,
        large_files: large,
        recommendation: recommendation.into(),
    })
}

fn detect_memory_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
                / 1_048_576;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/meminfo") {
            return text
                .lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                / 1024;
        }
    }
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = std::process::Command::new("powershell");
        command.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
                / 1_048_576;
        }
    }
    0
}

#[tauri::command]
pub fn sync_client_config(
    instance_id: String,
    profile: String,
    render_distance: u32,
    simulation_distance: u32,
    target_fps: u32,
) -> Result<String, String> {
    let config = serde_json::json!({
        "showFps":true, "showFrameTime":true, "showSessionMetrics":true, "showCps":true, "showKeystrokes":true,
        "showPing":true, "showCoordinates":false, "showAttackCooldown":true, "showArmor":true, "showEffects":true,
        "showMemory":false, "showCompatibility":false, "showItemCounters":true, "showBiome":false, "showDirection":true,
        "reduceBackgroundFps":true, "compactHud":true, "limitParticles":true,
        "adaptiveParticles":true, "applyVanillaOptimizations":true, "backgroundFps":30, "maxParticlesPerTick":25,
        "targetFps":target_fps.clamp(30,240), "renderDistance":render_distance.clamp(2,32), "simulationDistance":simulation_distance.clamp(5,32),
        "entityDistancePercent":50, "biomeBlendRadius":0, "mipmapLevels":0, "entityShadows":false, "viewBobbing":false,
        "menuCollapsed":false, "hudX":6, "hudY":6, "keysXPercent":50, "keysY":6,
        "combatX":-150, "combatY":6, "itemsX":-27, "itemsYPercent":50, "hudOpacity":78, "hudScalePercent":100, "captureMode":false,
        "serverProfiles":{}, "serverHudProfiles":{}, "hudElementStyles":{}, "profile":profile, "configVersion":6
    });
    let path = instance_root(&instance_id)?
        .join("config")
        .join("aureus-client.json");
    let parent = path.parent().ok_or("Ruta de configuración no válida")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn analyze_latest_crash(instance_id: String) -> Result<CrashAnalysis, String> {
    let root = instance_root(&instance_id)?;
    let candidates = [root.join("crash-reports"), root.join("logs")];
    let mut latest: Option<PathBuf> = None;
    for directory in candidates {
        if !directory.exists() {
            continue;
        }
        for entry in fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            if entry.path().is_file()
                && latest
                    .as_ref()
                    .map(|p| {
                        entry.metadata().ok().and_then(|m| m.modified().ok())
                            > fs::metadata(p).ok().and_then(|m| m.modified().ok())
                    })
                    .unwrap_or(true)
            {
                latest = Some(entry.path());
            }
        }
    }
    let Some(path) = latest else {
        return Ok(CrashAnalysis {
            detected: false,
            summary: "No se encontraron cierres registrados".into(),
            suspected_mods: Vec::new(),
            actions: Vec::new(),
        });
    };
    let mut text = String::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(2_000_000)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    let mut mods = Vec::new();
    for line in text.lines() {
        let lower = line.to_lowercase();
        if (lower.contains("mod") || lower.contains("mixin"))
            && (lower.contains("caused by")
                || lower.contains("failed")
                || lower.contains("exception"))
        {
            let sample = line.trim().chars().take(140).collect::<String>();
            if !mods.contains(&sample) {
                mods.push(sample);
            }
            if mods.len() == 5 {
                break;
            }
        }
    }
    let summary = if text.contains("OutOfMemoryError") {
        "Minecraft agotó la memoria asignada"
    } else if text.contains("MixinApplyError") {
        "Un mod o mixin es incompatible"
    } else if text.contains("UnsupportedClassVersionError") {
        "La versión de Java no es compatible"
    } else {
        "Minecraft terminó con un error registrado"
    };
    Ok(CrashAnalysis {
        detected: true,
        summary: summary.into(),
        suspected_mods: mods,
        actions: vec![
            "Crear respaldo".into(),
            "Desactivar contenido sospechoso".into(),
            "Reparar archivos oficiales".into(),
        ],
    })
}

#[tauri::command]
pub async fn search_modrinth(query: String, version: String) -> Result<Vec<ModrinthHit>, String> {
    let facets = serde_json::to_string(&vec![
        vec!["project_type:mod"],
        vec!["categories:fabric"],
        vec![&format!("versions:{version}")],
    ])
    .map_err(|e| e.to_string())?;
    let response: ModrinthSearch = reqwest::Client::builder()
        .user_agent(concat!("Aureus-Launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?
        .get("https://api.modrinth.com/v2/search")
        .query(&[("query", query), ("limit", "20".into()), ("facets", facets)])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.hits)
}

#[tauri::command]
pub async fn install_modrinth(
    instance_id: String,
    version: String,
    project_id: String,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Aureus-Launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let result = crate::version_manager::install_modrinth_projects(
        &version,
        &instance_root(&instance_id)?.join("mods"),
        &[project_id],
        &client,
        false,
    )
    .await?;
    Ok(format!(
        "{} archivos instalados; {} no disponibles",
        result.0,
        result.1.len()
    ))
}

#[tauri::command]
pub fn enable_safe_mode(instance_id: String) -> Result<String, String> {
    let mods = instance_root(&instance_id)?.join("mods");
    if !mods.exists() {
        return Err("La instancia no contiene mods".into());
    }
    let backup = create_instance_backup(instance_id.clone())?;
    let mut disabled = 0;
    for entry in fs::read_dir(&mods)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let essential = name.contains("fabric-api")
            || name.contains("fabricloader")
            || name.contains("aureus-client");
        if path.extension().and_then(|v| v.to_str()) == Some("jar") && !essential {
            fs::rename(&path, path.with_extension("jar.disabled")).map_err(|e| e.to_string())?;
            disabled += 1;
        }
    }
    Ok(format!(
        "Modo seguro activado: {disabled} mods desactivados. Respaldo: {backup}"
    ))
}

#[tauri::command]
pub fn read_benchmark(instance_id: String) -> Result<BenchmarkReport, String> {
    let path = instance_root(&instance_id)?.join("aureus-benchmark.json");
    if !path.exists() {
        return Ok(BenchmarkReport {
            available: false,
            average_fps: 0,
            one_percent_low: 0,
            memory_used_mb: 0,
            samples: 0,
            recommendation: "Juega al menos 20 segundos para generar mediciones reales".into(),
        });
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let average = value["averageFps"].as_u64().unwrap_or(0);
    let low = value["onePercentLow"].as_u64().unwrap_or(0);
    let memory = value["memoryUsedMb"].as_u64().unwrap_or(0);
    let recommendation = if average < 45 {
        "Usa RAM mínima, 6 chunks y desactiva shaders"
    } else if low * 2 < average {
        "Hay tirones: reduce simulación y revisa mods"
    } else if memory > 4500 {
        "El uso de RAM es alto: activa el perfil equilibrado"
    } else {
        "El rendimiento es estable para este perfil"
    };
    Ok(BenchmarkReport {
        available: true,
        average_fps: average,
        one_percent_low: low,
        memory_used_mb: memory,
        samples: value["samples"].as_u64().unwrap_or(0),
        recommendation: recommendation.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_ids_reject_paths_and_accept_versions() {
        assert!(validate_id("1.21.11-pvp").is_ok());
        assert!(validate_id("../../minecraft").is_err());
        assert!(validate_id("folder/name").is_err());
        assert!(validate_id("").is_err());
    }

    #[test]
    fn portable_profiles_round_trip_without_account_data() {
        let profile = PortableProfile {
            schema_version: 1,
            name: "PvP".into(),
            profile: "COMPETITIVO".into(),
            memory_mb: 5120,
            resolution_width: 1280,
            resolution_height: 720,
            jvm_args: vec!["-XX:+UseG1GC".into()],
            client_config: serde_json::json!({"showFps": true}),
        };
        let value = serde_json::to_value(&profile).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert!(value.get("accessToken").is_none());
        let restored: PortableProfile = serde_json::from_value(value).unwrap();
        assert_eq!(restored.memory_mb, 5120);
    }
}
