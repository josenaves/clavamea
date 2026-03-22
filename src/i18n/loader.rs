//! Fluent resource loading from filesystem.

use std::fs;
use std::path::Path;

use anyhow::Result;
use fluent::FluentResource;
use fluent_bundle::concurrent::FluentBundle;

type ConcurrentBundle = FluentBundle<FluentResource>;

/// Load a Fluent bundle for a specific language.
pub fn load_bundle(locales_dir: &Path, lang: &str) -> Result<ConcurrentBundle> {
    let mut bundle = FluentBundle::new_concurrent(vec![lang.parse()?]);

    // Load the .ftl file for this language
    let ftl_path = locales_dir.join(format!("{}.ftl", lang));
    if ftl_path.exists() {
        let content = fs::read_to_string(ftl_path)?;
        let resource = FluentResource::try_new(content)
            .map_err(|(_, errors)| anyhow::anyhow!("Fluent parsing errors: {:?}", errors))?;
        bundle
            .add_resource(resource)
            .map_err(|errors| anyhow::anyhow!("Fluent add_resource errors: {:?}", errors))?;
    }

    Ok(bundle)
}

/// Get available languages from the locales directory.
pub fn available_languages(locales_dir: &Path) -> Result<Vec<String>> {
    let mut languages = Vec::new();

    if locales_dir.exists() && locales_dir.is_dir() {
        for entry in fs::read_dir(locales_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("ftl") {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    languages.push(stem.to_string());
                }
            }
        }
    }

    Ok(languages)
}
