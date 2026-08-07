use aureus_launcher_lib::version_manager;
use std::{
    env, fs,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn main() {
    let version = env::args().nth(1).unwrap_or_else(|| "1.8.9".into());
    let minecraft = dirs::data_dir().unwrap().join("minecraft");
    let config = dirs::config_dir().unwrap().join("Aureus Launcher");
    let version_json = minecraft
        .join("versions")
        .join(&version)
        .join(format!("{version}.json"));
    let client_jar = minecraft
        .join("versions")
        .join(&version)
        .join(format!("{version}.jar"));
    let metadata = version_manager::read_launch_metadata(&version_json).unwrap();
    let java_root = config
        .join("runtimes")
        .join(format!("java-{}", metadata.java_major));
    let java = find_java(&java_root).expect("prepared java");
    let game = config.join("instances-data").join(&version);
    fs::create_dir_all(&game).unwrap();
    let natives = game.join("natives");
    version_manager::extract_natives(&version_json, &minecraft, &natives).unwrap();
    let mut classpath = version_manager::classpath(&version_json, &minecraft, &client_jar).unwrap();
    let fabric_profile = minecraft
        .join("versions")
        .join(format!("fabric-loader-0.19.3-{version}"))
        .join(format!("fabric-loader-0.19.3-{version}.json"));
    let mut main_class = metadata.main_class.clone();
    if fabric_profile.exists() {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&fabric_profile).unwrap()).unwrap();
        main_class = value["mainClass"].as_str().unwrap().into();
        for library in value["libraries"].as_array().unwrap() {
            if let Some(path) = version_manager::maven_library_path(
                &minecraft.join("libraries"),
                library["name"].as_str().unwrap(),
            ) {
                classpath.push(path);
            }
        }
    }
    let cp = classpath
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        });
    let assets = minecraft.join("assets");
    let mut args = metadata
        .game_args
        .into_iter()
        .map(|arg| {
            arg.replace("${auth_player_name}", "AureusSmoke")
                .replace("${version_name}", &version)
                .replace("${game_directory}", game.to_string_lossy().as_ref())
                .replace("${assets_root}", assets.to_string_lossy().as_ref())
                .replace("${assets_index_name}", "legacy")
                .replace("${auth_uuid}", "00000000000000000000000000000000")
                .replace("${auth_access_token}", "0")
                .replace("${user_properties}", "{}")
                .replace("${user_type}", "msa")
                .replace("${version_type}", "release")
                .replace(
                    "${auth_session}",
                    "token:0:00000000000000000000000000000000",
                )
                .replace(
                    "${game_assets}",
                    minecraft
                        .join("assets/virtual/legacy")
                        .to_string_lossy()
                        .as_ref(),
                )
        })
        .collect::<Vec<_>>();
    let log = fs::File::create(game.join("smoke.log")).unwrap();
    let err = log.try_clone().unwrap();
    let mut command = Command::new(java);
    if cfg!(target_os = "macos") {
        command.arg("-XstartOnFirstThread");
    }
    let mut child = command
        .arg("-Xmx2G")
        .arg(format!("-Djava.library.path={}", natives.display()))
        .arg("-cp")
        .arg(cp)
        .arg(main_class)
        .args(args.drain(..))
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_secs(12));
    if let Some(status) = child.try_wait().unwrap() {
        panic!("Minecraft ended early: {status}");
    }
    child.kill().unwrap();
    let _ = child.wait();
    println!("SMOKE_OK {version}");
}

fn find_java(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).ok()?.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path)
            } else if path.file_name()?.to_str()? == "java"
                && path.parent()?.file_name()?.to_str()? == "bin"
            {
                return Some(path);
            }
        }
    }
    None
}
