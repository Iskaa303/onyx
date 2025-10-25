use crate::fs::Entry;
use std::path::Path;

pub fn show_entries(entries: &[Entry], path: &Path) {
    if entries.is_empty() {
        println!("Empty directory");
        return;
    }

    let path_str = path.display().to_string();
    let display_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);

    println!("\nDirectory: {}\n", display_path);
    println!("{:<40} {:<10} {}", "Name", "Type", "Size");
    println!("{}", "-".repeat(60));

    for entry in entries {
        let entry_type = if entry.is_dir { "DIR" } else { "FILE" };
        let size = format_size(entry.size);
        println!("{:<40} {:<10} {}", entry.name, entry_type, size);
    }

    let total_files = entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = entries.iter().filter(|e| e.is_dir).count();

    println!("\n{} directories, {} files", total_dirs, total_files);
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
