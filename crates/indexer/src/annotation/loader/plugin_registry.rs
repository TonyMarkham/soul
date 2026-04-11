use crate::{IndexerError, IndexerResult, annotation::loader::LoadedPlugin, config::PluginEntry};

use soul_plugin_sdk::{AnnotationParser_TO, SoulPluginRef};

use abi_stable::{
    library::{RawLibrary, lib_header_from_raw_library},
    std_types::RBox,
};
use std::path::Path;

pub struct PluginRegistry {
    plugins: Vec<LoadedPlugin>,
}

impl PluginRegistry {
    pub fn load(entries: &[PluginEntry], root: &Path) -> IndexerResult<Self> {
        let mut plugins = Vec::with_capacity(entries.len());

        for entry in entries {
            let path = if entry.path.is_absolute() {
                entry.path.clone()
            } else {
                root.join(&entry.path)
            };

            // Load the raw library without going through abi_stable's global static cache.
            // `load_from_file` / `load_from` caches the first loaded module per type, so
            // calling it for a second plugin of the same type returns the first plugin.
            // We bypass the cache by using RawLibrary directly and calling init_root_module
            // on the LibHeader, which extracts the module without touching the static.
            let raw_lib = RawLibrary::load_at(&path)
                .map_err(|e| IndexerError::plugin_load(entry.language.clone(), path.clone(), e))?;
            // Leak so the library lives for the duration of the process.
            let raw_lib: &'static RawLibrary = Box::leak(Box::new(raw_lib));
            let lib_header = unsafe { lib_header_from_raw_library(raw_lib) }
                .map_err(|e| IndexerError::plugin_load(entry.language.clone(), path.clone(), e))?;
            let module: SoulPluginRef = lib_header
                .init_root_module()
                .map_err(|e| IndexerError::plugin_load(entry.language.clone(), path.clone(), e))?;

            let parser = (module.parser())();
            plugins.push(LoadedPlugin {
                language: entry.language.clone(),
                parser,
            });
        }

        validate_unique_languages(&plugins)?;
        validate_unique_extensions(&plugins)?;

        Ok(Self { plugins })
    }

    pub fn parser_for_extension(
        &self,
        ext: &str,
    ) -> Option<&AnnotationParser_TO<'static, RBox<()>>> {
        self.plugins
            .iter()
            .find(|p| p.parser.extension().as_str() == ext)
            .map(|p| &p.parser)
    }

    pub fn iter(&self) -> impl Iterator<Item = &LoadedPlugin> {
        self.plugins.iter()
    }
}

fn validate_unique_languages(plugins: &[LoadedPlugin]) -> IndexerResult<()> {
    for (i, a) in plugins.iter().enumerate() {
        for b in &plugins[..i] {
            if a.language == b.language {
                return Err(IndexerError::duplicate_plugin_language(a.language.clone()));
            }
        }
    }
    Ok(())
}

fn validate_unique_extensions(plugins: &[LoadedPlugin]) -> IndexerResult<()> {
    for (i, a) in plugins.iter().enumerate() {
        let a_ext = a.parser.extension().to_string();
        for b in &plugins[..i] {
            let b_ext = b.parser.extension().to_string();
            if a_ext == b_ext {
                return Err(IndexerError::duplicate_plugin_extension(
                    a_ext,
                    b.language.clone(),
                    a.language.clone(),
                ));
            }
        }
    }
    Ok(())
}
