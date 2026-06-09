use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

const RESET: &str = "\x1b[0m";
const DIRECTORY_COLOR: &str = "\x1b[34m";
const EXECUTABLE_COLOR: &str = "\x1b[32m";
const SYMLINK_COLOR: &str = "\x1b[36m";
const HIDDEN_COLOR: &str = "\x1b[90m";
const DEFAULT_COLOR: &str = "\x1b[39m";

fn main() -> io::Result<()> {
    let cwd = env::current_dir()?;
    let mut entries = fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let color = color_for_entry(&path);
        println!("{}{}{}", color, name, RESET);
    }

    Ok(())
}

fn color_for_entry(path: &PathBuf) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if name.starts_with('.') {
        return HIDDEN_COLOR;
    }

    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return SYMLINK_COLOR;
        }

        if metadata.is_dir() {
            return DIRECTORY_COLOR;
        }

        if metadata.is_file() && is_executable(&metadata) {
            return EXECUTABLE_COLOR;
        }
    }

    DEFAULT_COLOR
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}
