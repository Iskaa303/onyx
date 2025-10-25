use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub fn read_directory(path: &Path) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();

    let dir_entries = fs::read_dir(path)
        .context(format!("Failed to read directory: {}", path.display()))?;

    for entry_result in dir_entries {
        let entry = entry_result.context("Failed to read directory entry")?;
        let metadata = entry.metadata().context("Failed to read metadata")?;

        let name = entry.file_name()
            .to_string_lossy()
            .to_string();

        if name.starts_with('.') {
            continue;
        }

        entries.push(Entry {
            name,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
