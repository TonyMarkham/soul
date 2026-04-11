use indexer::{IndexerError, IndexerResult, constants};

use std::path::Path;

pub async fn run(root: &Path) -> IndexerResult<()> {
    let soul_dir = root.join(constants::SOUL_DIR);
    let config_path = soul_dir.join(constants::CONFIG_FILE);

    if config_path.exists() {
        return Err(IndexerError::cli(format!(
            "already initialised: `{}` exists",
            config_path.display()
        )));
    }

    std::fs::create_dir_all(&soul_dir)
        .map_err(|e| IndexerError::config_read(config_path.clone(), e))?;

    let plugins_dir = soul_dir.join("plugins");
    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| IndexerError::config_read(plugins_dir.clone(), e))?;

    let exe_dir = std::env::current_exe()
        .map_err(|e| IndexerError::cli(format!("cannot locate indexer binary: {e}")))?;
    let exe_dir = exe_dir
        .parent()
        .ok_or_else(|| IndexerError::cli("indexer binary has no parent directory"))?;
    let src_dir = exe_dir.join("plugins-src");

    let suffix = std::env::consts::DLL_SUFFIX;
    let plugins = [
        ("rust", "libsoul_plugin_rust"),
        ("csharp", "libsoul_plugin_csharp"),
    ];

    for (lang, lib_name) in &plugins {
        let src = src_dir.join(format!("{lib_name}{suffix}"));
        let dst = plugins_dir.join(format!("{lang}{suffix}"));
        std::fs::copy(&src, &dst).map_err(|e| {
            IndexerError::cli(format!(
                "failed to copy plugin `{}` → `{}`: {e}",
                src.display(),
                dst.display()
            ))
        })?;
    }

    let config = format!(
        "{base}\n[[plugins]]\nlanguage = \"rust\"\npath = \"./.soul/plugins/rust{suffix}\"\n\n[[plugins]]\nlanguage = \"csharp\"\npath = \"./.soul/plugins/csharp{suffix}\"\n",
        base = constants::SCAN_CONFIG_BASE,
    );
    std::fs::write(&config_path, config)
        .map_err(|e| IndexerError::config_read(config_path.clone(), e))?;

    println!("Initialized → {}", config_path.display());
    Ok(())
}
