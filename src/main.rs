use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use regex::Regex;

const RESET: &str = "\x1b[0m";
const DIRECTORY_COLOR: &str = "\x1b[34m";
const EXECUTABLE_COLOR: &str = "\x1b[32m";
const SYMLINK_COLOR: &str = "\x1b[36m";
const HIDDEN_COLOR: &str = "\x1b[90m";
const DEFAULT_COLOR: &str = "\x1b[39m";

#[derive(Debug)]
enum TargetKind {
    Any,
    File,
    Dir,
}

struct Rule {
    re: Regex,
    color: String,
    target: TargetKind,
}

fn main() -> io::Result<()> {
    // CLI: support `--profile <application>` to open the profile file in an application
    let mut args = env::args().skip(1);
    if let Some(flag) = args.next() {
        if flag == "--profile" {
            if let Some(app) = args.next() {
                return open_profile_in_app(&app);
            } else {
                eprintln!("Usage: cols --profile <application>");
                return Ok(());
            }
        }
    }

    let rules = load_rules().unwrap_or_default();

    let cwd = env::current_dir()?;
    let mut entries = fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let color = color_for_entry(&path, &rules);
        println!("{}{}{}", color, name, RESET);
    }

    Ok(())
}

fn color_for_entry(path: &PathBuf, rules: &Vec<Rule>) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

    // Profile rules are highest priority
    if let Ok(metadata) = fs::symlink_metadata(path) {
        for rule in rules {
            let target_ok = match rule.target {
                TargetKind::Any => true,
                TargetKind::Dir => metadata.is_dir(),
                TargetKind::File => metadata.is_file(),
            };
            if target_ok {
                if rule.re.is_match(name) {
                    return rule.color.clone();
                }
            }
        }
    } else {
        // If metadata can't be read, still try matching by name
        for rule in rules {
            if rule.re.is_match(name) {
                return rule.color.clone();
            }
        }
    }

    // Fallback to existing behavior
    if name.starts_with('.') {
        return HIDDEN_COLOR.to_string();
    }

    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return SYMLINK_COLOR.to_string();
        }

        if metadata.is_dir() {
            return DIRECTORY_COLOR.to_string();
        }

        if metadata.is_file() && is_executable(&metadata) {
            return EXECUTABLE_COLOR.to_string();
        }
    }

    DEFAULT_COLOR.to_string()
}

fn color_name_to_code(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "black" => Some("\x1b[30m"),
        "red" => Some("\x1b[31m"),
        "green" => Some("\x1b[32m"),
        "yellow" => Some("\x1b[33m"),
        "blue" => Some("\x1b[34m"),
        "magenta" => Some("\x1b[35m"),
        "cyan" => Some("\x1b[36m"),
        "white" => Some("\x1b[37m"),
        "bright_black" | "grey" => Some("\x1b[90m"),
        _ => None,
    }
}

fn parse_rule_line(line: &str) -> Option<(String, String, TargetKind)> {
    // expected formats:
    // dir:.*README.* => magenta
    // file:^.*\.rs$ => \x1b[35m
    let parts: Vec<&str> = line.split("=>").collect();
    if parts.len() != 2 {
        return None;
    }
    let mut pat = parts[0].trim();
    let color_part = parts[1].trim();

    let target = if let Some(rest) = pat.strip_prefix("dir:") {
        pat = rest.trim();
        TargetKind::Dir
    } else if let Some(rest) = pat.strip_prefix("file:") {
        pat = rest.trim();
        TargetKind::File
    } else {
        TargetKind::Any
    };

    let color = if color_part.starts_with("\\x1b[") {
        color_part.to_string()
    } else if let Some(code) = color_name_to_code(color_part) {
        code.to_string()
    } else {
        // fallback to default if unknown
        DEFAULT_COLOR.to_string()
    };

    Some((pat.to_string(), color, target))
}

fn load_rules() -> io::Result<Vec<Rule>> {
    // Determine profile path: env COLOR_RULES, ./color_rules, ~/.config/colorfuls/color_rules
    let paths = vec![
        env::var("COLOR_RULES").ok().map(PathBuf::from),
        Some(PathBuf::from("color_rules")),
        env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/colorfuls/color_rules")),
    ];

    for opt in paths.into_iter().flatten() {
        if opt.exists() {
            let text = fs::read_to_string(&opt)?;
            let mut rules = Vec::new();
            for (i, raw) in text.lines().enumerate() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((pat, color, target)) = parse_rule_line(line) {
                    match Regex::new(&pat) {
                        Ok(re) => rules.push(Rule { re, color, target }),
                        Err(e) => eprintln!("Skipping invalid regex on {}:{} ({})", opt.display(), i+1, e),
                    }
                } else {
                    eprintln!("Skipping malformed rule on {}:{}", opt.display(), i+1);
                }
            }
            return Ok(rules);
        }
    }

    Ok(Vec::new())
}

fn find_profile_path() -> Option<PathBuf> {
    let paths = vec![
        env::var("COLOR_RULES").ok().map(PathBuf::from),
        Some(PathBuf::from("color_rules")),
        env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/colorfuls/color_rules")),
    ];

    for opt in paths.into_iter().flatten() {
        if opt.exists() {
            return Some(opt);
        }
    }

    None
}

fn open_profile_in_app(app: &str) -> io::Result<()> {
    if let Some(path) = find_profile_path() {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("open").arg("-a").arg(app).arg(path).status();
            match status {
                Ok(s) => {
                    if !s.success() {
                        eprintln!("Failed to open profile with {}", app);
                    }
                }
                Err(e) => eprintln!("Error launching open: {}", e),
            }
            return Ok(());
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Try to execute the given application with the profile path as argument.
            match Command::new(app).arg(path).spawn() {
                Ok(_) => return Ok(()),
                Err(e) => {
                    eprintln!("Failed to launch {}: {}", app, e);
                    return Ok(());
                }
            }
        }
    } else {
        eprintln!("No profile file found to open");
    }
    Ok(())
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
