use aureus_launcher_lib::version_manager;
use std::{env, fs};

fn main() {
    let version = env::args().nth(1).unwrap_or_else(|| "1.21.1".into());
    let minecraft = dirs::data_dir().expect("data dir").join("minecraft");
    let config = dirs::config_dir()
        .expect("config dir")
        .join("Aureus Launcher");
    let client = reqwest::Client::builder()
        .user_agent("Aureus-Launcher-Verification/0.3.0")
        .build()
        .unwrap();
    tauri::async_runtime::block_on(async {
        let installed = version_manager::install_official_version(&version, &minecraft, &client)
            .await
            .expect("official install");
        let metadata = version_manager::read_launch_metadata(&installed.version_json)
            .expect("launch metadata");
        let java = version_manager::ensure_java_runtime(
            metadata.java_major,
            &config.join("runtimes"),
            &client,
        )
        .await
        .expect("java runtime");
        let loader = version_manager::compatible_fabric_loader(&version, &client)
            .await
            .expect("fabric lookup");
        let instance = config.join("instances-data").join(&version);
        fs::create_dir_all(&instance).unwrap();
        let (mods, unavailable) = if loader.is_some() {
            version_manager::resolve_modrinth_mods(&version, &instance.join("mods"), &client)
                .await
                .expect("mod resolver")
        } else {
            (0, Vec::new())
        };
        let natives = instance.join("natives-verification");
        version_manager::extract_natives(&installed.version_json, &minecraft, &natives)
            .expect("natives");
        let classpath =
            version_manager::classpath(&installed.version_json, &minecraft, &installed.client_jar)
                .expect("classpath");
        let report = serde_json::json!({"version":version,"javaMajor":metadata.java_major,"java":java,"loader":loader,"mods":mods,"unavailable":unavailable,"classpath":classpath.len(),"natives":natives.exists(),"mainClass":metadata.main_class});
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    });
}
