use crate::config::PluginEntry;
use std::path::Path;

pub fn test_plugin_entries() -> Vec<PluginEntry> {
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("target/debug");

    vec![
        PluginEntry {
            language: "rust".to_string(),
            path: target_dir.join(format!(
                "libsoul_plugin_rust{}",
                std::env::consts::DLL_SUFFIX
            )),
        },
        PluginEntry {
            language: "csharp".to_string(),
            path: target_dir.join(format!(
                "libsoul_plugin_csharp{}",
                std::env::consts::DLL_SUFFIX
            )),
        },
    ]
}
