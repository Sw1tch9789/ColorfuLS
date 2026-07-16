//! カレントディレクトリの一覧を表示する軽量な `cols` バイナリです
//!
//! - プロファイル (`color_rules`) による色付け
//! - `-a/--all` で隠しファイル表示
//! - `-l/--long` で `ls -l` 風の詳細表示
//!
//! ドキュメントは `cargo doc` で生成できます。

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use chrono::Local;
use regex::Regex;
use users::{get_group_by_gid, get_user_by_uid};

/// リセット用 ANSI シーケンス
const RESET: &str = "\x1b[0m";

/// ディレクトリ表示の色（ANSI シーケンス）
const DIRECTORY_COLOR: &str = "\x1b[34m";

/// 実行可能ファイル表示の色
const EXECUTABLE_COLOR: &str = "\x1b[32m";

/// シンボリックリンク表示の色
const SYMLINK_COLOR: &str = "\x1b[36m";

/// 隠しファイル表示の色
const HIDDEN_COLOR: &str = "\x1b[90m";

/// デフォルト色
const DEFAULT_COLOR: &str = "\x1b[39m";

/// プロファイルルールの対象種別
#[derive(Debug)]
enum TargetKind {
    /// ファイル・ディレクトリ問わず
    Any,
    /// ファイルのみ
    File,
    /// ディレクトリのみ
    Dir,
}

/// ルール構造体：正規表現、色指定、ターゲット種別
struct Rule {
    re: Regex,
    color: String,
    target: TargetKind,
}

/// エントリ表示のエントリポイント
///
/// - `-a/--all` で隠しファイルを表示
/// - `-l/--long` で詳細表示
fn main() -> io::Result<()> {
    // 引数収集
        let args_vec: Vec<String> = env::args().skip(1).collect();

    // プロファイルを開くオプション（優先）
    if let Some(pos) = args_vec.iter().position(|a| a == "--profile" || a == "-pf") {
        let app = args_vec.get(pos + 1).and_then(|s| if s.starts_with('-') { None } else { Some(s.clone()) });
        if let Some(app_name) = app {
            return open_profile_in_app(&app_name);
        } else {
            return open_profile_default();
        }
    }

    // プロファイル読み込み
    let rules = load_rules().unwrap_or_default();

    // フラグ解析
    let mut show_all = false;
    let mut long_format = false;
    for a in &args_vec {
        match a.as_str() {
            "-a" | "--all" => show_all = true,
            "-l" | "--long" => long_format = true,
            _ => {}
        }
    }

    // ディレクトリ読み取り
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

    // 名前でケースインセンシティブにソート（ls 風）
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    // -l の場合は列幅を事前計算
    let (mut max_user_w, mut max_group_w, mut max_size_w) = (0usize, 0usize, 0usize);
    if long_format {
        for entry in &entries {
            let path = entry.path();
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let uid = metadata.uid();
                    let gid = metadata.gid();
                    let user = get_user_by_uid(uid).and_then(|u| u.name().to_str().map(|s| s.to_string())).unwrap_or(uid.to_string());
                    let group = get_group_by_gid(gid).and_then(|g| g.name().to_str().map(|s| s.to_string())).unwrap_or(gid.to_string());
                    let size = metadata.len();
                    max_user_w = max_user_w.max(user.len());
                    max_group_w = max_group_w.max(group.len());
                    max_size_w = max_size_w.max(size.to_string().len());
                }
                #[cfg(not(unix))]
                {
                    let size = metadata.len();
                    max_size_w = max_size_w.max(size.to_string().len());
                }
            }
        }
    }

    // 表示ループ
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let color = color_for_entry(&path, &rules);

        if long_format {
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                println!("{}{}", format_long_entry_with_widths(&path, &metadata, &color, &name, max_user_w, max_group_w, max_size_w), RESET);
            } else {
                println!("{}{}{}", color, name, RESET);
            }
        } else {
            println!("{}{}{}", color, name, RESET);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// プロファイル（color_rules）関連ヘルパー群
// ---------------------------------------------------------------------------

/// ルール行を解析して (pattern, color_spec, target) を返す
fn parse_rule_line(line: &str) -> Option<(String, String, TargetKind)> {
    // 期待される形式:
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

    let color = color_part.to_string();
    Some((pat.to_string(), color, target))
}

/// プロファイルファイルのパス候補を探す
fn find_profile_path() -> Option<PathBuf> {
    let paths = vec![
        env::var("COLOR_RULES").ok().map(PathBuf::from),
        Some(PathBuf::from("color_rules")),
        env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/colorfuls/color_rules")),
    ];

    paths.into_iter().flatten().find(|opt| opt.exists())
}

/// プロファイルを読み込み、正規表現とルールリストを返す
fn load_rules() -> io::Result<Vec<Rule>> {
    let paths = vec![
        env::var("COLOR_RULES").ok().map(PathBuf::from),
        Some(PathBuf::from("color_rules")),
        env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/colorfuls/color_rules")),
    ];
    if let Some(opt) = paths.into_iter().flatten().find(|opt| opt.exists()) {
        let text = fs::read_to_string(&opt)?;
        let mut rules = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((pat, color, target)) = parse_rule_line(line) {
                match regex::RegexBuilder::new(&pat).case_insensitive(true).build() {
                    Ok(re) => rules.push(Rule { re, color, target }),
                    Err(e) => eprintln!("Skipping invalid regex on {}:{} ({})", opt.display(), i+1, e),
                }
            } else {
                eprintln!("Skipping malformed rule on {}:{}", opt.display(), i+1);
            }
        }
        Ok(rules)
    } else {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// 色関連ユーティリティ
// ---------------------------------------------------------------------------

/// 名前から短い ANSI カラーコードを返す（既存の色名対応）
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

/// RGB を 256 色インデックスに近似変換する
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // 6x6x6 キューブへマップ（16..231）
    let r6 = (r as u16 * 5 / 255) as u8;
    let g6 = (g as u16 * 5 / 255) as u8;
    let b6 = (b as u16 * 5 / 255) as u8;
    16 + 36 * r6 + 6 * g6 + b6
}

/// ターミナルが truecolor (24bit) をサポートしているか簡易判定
fn supports_truecolor() -> bool {
    if env::var("COLORTERM").map(|c| { let lower = c.to_lowercase(); lower.contains("truecolor") || lower.contains("24bit") }).unwrap_or(false) {
        return true;
    }
    if env::var("TERM").map(|t| t.to_lowercase().contains("truecolor")).unwrap_or(false) {
        return true;
    }
    false
}

/// color spec (#RRGGBB, rgb:R,G,B, ansi:N, 名前, または既に ANSI エスケープ) を
/// 実際の ANSI エスケープ列に変換して返す
fn color_spec_to_escape(spec: &str) -> String {
    if spec.starts_with("\x1b[") {
        return spec.to_string();
    }
    if spec.starts_with('#') && spec.len() == 7
        && let Ok(r) = u8::from_str_radix(&spec[1..3], 16)
        && let Ok(g) = u8::from_str_radix(&spec[3..5], 16)
        && let Ok(b) = u8::from_str_radix(&spec[5..7], 16) {
        if supports_truecolor() {
            return format!("\x1b[38;2;{};{};{}m", r, g, b);
        } else {
            let idx = rgb_to_ansi256(r, g, b);
            return format!("\x1b[38;5;{}m", idx);
        }
    }

    if spec.to_lowercase().starts_with("rgb:") {
        let rest = &spec[4..];
        let parts: Vec<&str> = rest.split(',').collect();
        if parts.len() == 3
            && let Ok(r) = parts[0].trim().parse::<u8>()
            && let Ok(g) = parts[1].trim().parse::<u8>()
            && let Ok(b) = parts[2].trim().parse::<u8>() {
            if supports_truecolor() {
                return format!("\x1b[38;2;{};{};{}m", r, g, b);
            } else {
                let idx = rgb_to_ansi256(r, g, b);
                return format!("\x1b[38;5;{}m", idx);
            }
        }
    }

    if spec.to_lowercase().starts_with("ansi:") && let Ok(idx) = spec[5..].trim().parse::<u8>() {
        return format!("\x1b[38;5;{}m", idx);
    }

    if let Some(code) = color_name_to_code(spec) {
        return code.to_string();
    }

    DEFAULT_COLOR.to_string()
}

/// エントリに対する色を決定する（プロファイル優先）
fn color_for_entry(path: &PathBuf, rules: &Vec<Rule>) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

    // 1) プロファイルルール（優先）
    if let Ok(metadata) = fs::symlink_metadata(path) {
        for rule in rules {
            let target_ok = match rule.target {
                TargetKind::Any => true,
                TargetKind::Dir => metadata.is_dir(),
                TargetKind::File => metadata.is_file(),
            };
            if target_ok && rule.re.is_match(name) {
                return color_spec_to_escape(&rule.color);
            }
        }
    } else {
        for rule in rules {
            if rule.re.is_match(name) {
                return color_spec_to_escape(&rule.color);
            }
        }
    }

    // 2) デフォルト挙動
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

// ---------------------------------------------------------------------------
// ロング表示とファイル情報関連
// ---------------------------------------------------------------------------

/// パーミッションを rwx 形式の文字列に変換する
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

/// ロング表示用の整形（owner/group 幅を受け取り揃えて出力）
fn format_long_entry_with_widths(path: &PathBuf, metadata: &fs::Metadata, color: &str, name: &str, user_w: usize, group_w: usize, size_w: usize) -> String {
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

        // シンボリックリンクは "name -> target" で表示
        let display_name = if metadata.file_type().is_symlink() {
            match fs::read_link(path) {
                Ok(target) => format!("{}{}{} -> {}", color, name, RESET, target.display()),
                Err(_) => format!("{}{}{}", color, name, RESET),
            }
        } else {
            format!("{}{}{}", color, name, RESET)
        };

        format!("{}{} {:>3} {:<user_w$} {:<group_w$} {:>size_w$} {} {}", file_type, perms, nlink, user, group, size, mtime, display_name, user_w=user_w, group_w=group_w, size_w=size_w)
    }

    #[cfg(not(unix))]
    {
        // 非 unix フォールバック
        let file_type = if metadata.is_dir() { 'd' } else { '-' };
        let perms = format_permissions(metadata);
        let size = metadata.len();
        let mtime = metadata.modified().ok().map(|t| {
            let dt: chrono::DateTime<Local> = t.into();
            dt.format("%b %e %H:%M").to_string()
        }).unwrap_or_else(|| "-".to_string());
        let display_name = format!("{}{}{}", color, name, RESET);
        return format!("{}{} {:>size_w$} {} {}", file_type, perms, size, mtime, display_name, size_w=size_w);
    }
}

/// 実行可能ビット判定
#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}

// ---------------------------------------------------------------------------
// プロファイルを開くためのユーティリティ（open/xdg-open 等）
// ---------------------------------------------------------------------------

/// 指定アプリでプロファイルファイルを開く
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

/// システムデフォルトでプロファイルを開く
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
            if Command::new("xdg-open").arg(&path).status().is_ok() {
                return Ok(());
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            if let Ok(editor) = env::var("EDITOR") {
                if Command::new(editor).arg(&path).spawn().is_ok() {
                    return Ok(());
                }
            }
        }

        // フォールバック呼び出しは上で環境別に既に処理しているためここでは不要
    } else {
        eprintln!("No profile file found to open");
    }
    Ok(())
}
