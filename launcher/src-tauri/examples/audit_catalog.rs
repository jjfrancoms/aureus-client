use futures_util::{stream, StreamExt};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct Entry {
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<Entry>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri::async_runtime::block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Aureus-Catalog-Audit/0.3.0")
            .build()?;
        let manifest: Manifest = client
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let total = manifest.versions.len();
        let checks = stream::iter(manifest.versions.into_iter().map(|entry| {
            let client = client.clone();
            async move {
                let value: serde_json::Value = client
                    .get(&entry.url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?
                    .json()
                    .await
                    .map_err(|e| e.to_string())?;
                let mut missing = Vec::new();
                if value
                    .pointer("/downloads/client/url")
                    .and_then(|v| v.as_str())
                    .is_none()
                {
                    missing.push("client");
                }
                if value.get("mainClass").and_then(|v| v.as_str()).is_none() {
                    missing.push("mainClass");
                }
                if value.get("libraries").and_then(|v| v.as_array()).is_none() {
                    missing.push("libraries");
                }
                if value
                    .get("assetIndex")
                    .and_then(|v| v.as_object())
                    .is_none()
                {
                    missing.push("assetIndex");
                }
                if value
                    .pointer("/arguments/game")
                    .and_then(|v| v.as_array())
                    .is_none()
                    && value
                        .get("minecraftArguments")
                        .and_then(|v| v.as_str())
                        .is_none()
                {
                    missing.push("arguments");
                }
                if missing.is_empty() {
                    Ok(entry.id)
                } else {
                    Err(format!("{}: {}", entry.id, missing.join(", ")))
                }
            }
        }))
        .buffer_unordered(20)
        .collect::<Vec<_>>()
        .await;
        let failures: Vec<_> = checks.into_iter().filter_map(Result::err).collect();
        println!("AUDIT_TOTAL={total}");
        println!("AUDIT_COMPATIBLE={}", total - failures.len());
        println!("AUDIT_FAILURES={}", failures.len());
        for failure in &failures {
            println!("UNSUPPORTED={failure}");
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err("el catálogo contiene formatos sin soporte".into())
        }
    })
}
