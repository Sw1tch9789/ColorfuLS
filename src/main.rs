use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use regex::Regex;
use chrono::Local;
use users::{get_user_by_uid, get_group_by_gid};

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
    let mut args_vec: Vec<String> = env::args().skip(1).collect();
    // search for --profile or -pf anywhere in args
    if let Some(pos) = args_vec.iter().position(|a| a == "--profile" || a == "-pf") {
        // if next token exists and is not another flag, treat as application name
        let app = args_vec.get(pos + 1).and_then(|s| if s.starts_with('-') { None } else { Some(s.clone()) });
        if let Some(app_name) = app {
            return open_profile_in_app(&app_name);
        } else {
            // open with system default
            return open_profile_default();
        }
    }

    let rules = load_rules().unwrap_or_default();

    // parse simple flags: -a/--all (show hidden), -l/--long (long listing)
    let mut show_all = false;
    let mut long_format = false;
    for a in &args_vec {
        match a.as_str() {
            "-a" | "--all" => show_all = true,
            "-l" | "--long" => long_format = true,
            _ => {}
        }
    }

    let cwd = env::current_dir()?;
    let mut entries = fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            if show_all {
                true
            } else {
                let name = entry.file_name().to_string_lossy().into_owned();
                !name.starts_with('.')
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let color = color_for_entry(&path, &rules);
        if long_format {
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                println!("{}{}{}", format_long_entry(&path, &metadata, &color, &name), RESET, "");
            } else {
                println!("{}{}{}", color, name, RESET);
            }
        } else {
            println!("{}{}{}", color, name, RESET);
        }
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
                    return color_spec_to_escape(&rule.color);
                }
            }
        }
    } else {
        // If metadata can't be read, still try matching by name
        for rule in rules {
            if rule.re.is_match(name) {
                return color_spec_to_escape(&rule.color);
            }
        }
    }

    // Fallback to existing behavior
    if name.starts_with('.') {
        return HIDDEN_COLOR.to_string();
    }

    // cargo handled via profile rules (color_rules)

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

    // Store the raw color spec; resolution to an ANSI escape happens at render time.
    let color = color_part.to_string();

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
                    // Always compile profile regexes case-insensitively so patterns like
                    // '.*cargo.*' match 'Cargo.lock' as well.
                    match regex::RegexBuilder::new(&pat).case_insensitive(true).build() {
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

fn supports_truecolor() -> bool {
    if let Ok(colorterm) = env::var("COLORTERM") {
        let lower = colorterm.to_lowercase();
        if lower.contains("truecolor") || lower.contains("24bit") {
            return true;
        }
    }
    if let Ok(term) = env::var("TERM") {
        if term.to_lowercase().contains("truecolor") {
            return true;
        }
    }
    false
}

fn format_long_entry(path: &PathBuf, metadata: &fs::Metadata, color: &str, name: &str) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let file_type = if metadata.is_dir() {
            'd'
        } else if metadata.file_type().is_symlink() {
            'l'
        } else {
            '-'
        };

        let perms = format_permissions(metadata);
        let nlink = metadata.nlink();
        let size = metadata.len();
        let mtime = metadata.modified().ok().map(|t| {
            let dt: chrono::DateTime<Local> = t.into();
            dt.format("%b %e %H:%M").to_string()
        }).unwrap_or_else(|| "-".to_string());

        let uid = metadata.uid();
        let gid = metadata.gid();
        let user = get_user_by_uid(uid).and_then(|u| u.name().to_str().map(|s| s.to_string())).unwrap_or(uid.to_string());
        let group = get_group_by_gid(gid).and_then(|g| g.name().to_str().map(|s| s.to_string())).unwrap_or(gid.to_string());

        return format!("{}{} {:>3} {:<8} {:<8} {:>8} {} {}{}", file_type, perms, nlink, user, group, size, mtime, color, name);
    }

    // non-unix fallback
    let file_type = if metadata.is_dir() { 'd' } else { '-' };
    let perms = format_permissions(metadata);
    let size = metadata.len();
    let mtime = metadata.modified().ok().map(|t| {
        let dt: chrono::DateTime<Local> = t.into();
        dt.format("%b %e %H:%M").to_string()
    }).unwrap_or_else(|| "-".to_string());
    format!("{}{} {:>8} {} {}{}", file_type, perms, size, mtime, color, name)
}

#[cfg(unix)]
fn format_permissions(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    let mut s = String::with_capacity(9);
    let flags = [0o400,0o200,0o100, 0o040,0o020,0o010, 0o004,0o002,0o001];
    for &f in &flags {
        s.push(if mode & f != 0 { match f {
            0o400|0o040|0o004 => 'r',
            0o200|0o020|0o002 => 'w',
            _ => 'x',
        } } else { '-'});
    }
    s
}

#[cfg(not(unix))]
fn format_permissions(_metadata: &fs::Metadata) -> String {
    "rwxrwxrwx".to_string()
}

fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // map to 6x6x6 cube (16..231)
    let r6 = (r as u16 * 5 / 255) as u8;
    let g6 = (g as u16 * 5 / 255) as u8;
    let b6 = (b as u16 * 5 / 255) as u8;
    16 + 36 * r6 + 6 * g6 + b6
}

fn color_spec_to_escape(spec: &str) -> String {
    // if it's already an escape seq
    if spec.starts_with("\x1b[") {
        return spec.to_string();
    }

    // hex: #RRGGBB
    if spec.starts_with('#') && spec.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&spec[1..3], 16),
            u8::from_str_radix(&spec[3..5], 16),
            u8::from_str_radix(&spec[5..7], 16),
        ) {
            if supports_truecolor() {
                return format!("\x1b[38;2;{};{};{}m", r, g, b);
            } else {
                let idx = rgb_to_ansi256(r, g, b);
                return format!("\x1b[38;5;{}m", idx);
            }
        }
    }

    // rgb:R,G,B
    if spec.to_lowercase().starts_with("rgb:") {
        let rest = &spec[4..];
        let parts: Vec<&str> = rest.split(',').collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (parts[0].trim().parse::<u8>(), parts[1].trim().parse::<u8>(), parts[2].trim().parse::<u8>()) {
                if supports_truecolor() {
                    return format!("\x1b[38;2;{};{};{}m", r, g, b);
                } else {
                    let idx = rgb_to_ansi256(r, g, b);
                    return format!("\x1b[38;5;{}m", idx);
                }
            }
        }
    }

    // ansi:NNN (256-color index)
    if spec.to_lowercase().starts_with("ansi:") {
        if let Ok(idx) = spec[5..].trim().parse::<u8>() {
            return format!("\x1b[38;5;{}m", idx);
        }
    }

    // named color fallback
    if let Some(code) = color_name_to_code(spec) {
        return code.to_string();
    }

    DEFAULT_COLOR.to_string()
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

fn open_profile_default() -> io::Result<()> {
    if let Some(path) = find_profile_path() {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("open").arg(path).status();
            match status {
                Ok(s) => if !s.success() { eprintln!("open returned non-zero"); },
                Err(e) => eprintln!("Error launching open: {}", e),
            }
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Prefer xdg-open when available
            if Command::new("xdg-open").arg(&path).status().is_ok() {
                return Ok(());
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            // Try $EDITOR if set
            if let Ok(editor) = env::var("EDITOR") {
                if Command::new(editor).arg(&path).spawn().is_ok() {
                    return Ok(());
                }
            }
        }

        // Final fallback: try to spawn system default by invoking `open` (mac) or xdg-open (linux)
        #[cfg(target_os = "macos")]
        { let _ = Command::new("open").arg(&path).status(); }
        #[cfg(target_os = "linux")]
        { let _ = Command::new("xdg-open").arg(&path).status(); }
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
