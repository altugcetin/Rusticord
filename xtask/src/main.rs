use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("gate"));
    match command.as_str() {
        "scan-comments" => scan_comments(),
        "gate" => gate(),
        other => {
            let _ = writeln!(io::stderr(), "unknown xtask command: {other}");
            ExitCode::from(2)
        }
    }
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.parent() {
        Some(parent) => parent.to_path_buf(),
        None => PathBuf::from("."),
    }
}

fn gate() -> ExitCode {
    if !cargo(&["fmt", "--all", "--", "--check"]) {
        return ExitCode::from(1);
    }
    if scan_comments() != ExitCode::SUCCESS {
        return ExitCode::from(1);
    }
    if !cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ]) {
        return ExitCode::from(1);
    }
    if !cargo(&["nextest", "run", "--workspace"]) {
        return ExitCode::from(1);
    }
    if !cargo(&["deny", "check"]) {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn cargo(args: &[&str]) -> bool {
    match Command::new("cargo")
        .args(args)
        .current_dir(workspace_root())
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => {
            let _ = writeln!(io::stderr(), "failed to invoke cargo");
            false
        }
    }
}

fn scan_comments() -> ExitCode {
    let root = workspace_root();
    let files = match git_tracked_files(&root) {
        Ok(files) => files,
        Err(message) => {
            let _ = writeln!(io::stderr(), "{message}");
            return ExitCode::from(1);
        }
    };
    let mut hits = 0_u32;
    for relative in files {
        if should_skip(&relative) {
            continue;
        }
        let path = root.join(&relative);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if let Some(kind) = relative.extension().and_then(|ext| ext.to_str()) {
            match kind {
                "rs" => {
                    if let Some(offset) = rust_comment_offset(&bytes) {
                        hits += 1;
                        let _ = writeln!(
                            io::stderr(),
                            "{relative} line {line}: comment token",
                            relative = relative.display(),
                            line = line_number(&bytes, offset)
                        );
                    }
                }
                "toml" | "yml" | "yaml" | "cfg" => {
                    if let Ok(text) = std::str::from_utf8(&bytes)
                        && let Some(line) = hash_comment_line(text)
                    {
                        hits += 1;
                        let _ = writeln!(
                            io::stderr(),
                            "{relative} line {line}: comment token",
                            relative = relative.display()
                        );
                    }
                }
                _ => {}
            }
        }
    }
    if hits == 0 {
        ExitCode::SUCCESS
    } else {
        let _ = writeln!(io::stderr(), "comment scan failed with {hits} hits");
        ExitCode::from(1)
    }
}

fn should_skip(path: &Path) -> bool {
    path.ends_with("Cargo.lock") || path.ends_with("LICENSE")
}

fn git_tracked_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .map_err(|_| String::from("git ls-files failed"))?;
    if !output.status.success() {
        return Err(String::from("git ls-files exited unsuccessfully"));
    }
    let mut files = Vec::new();
    for chunk in output.stdout.split(|byte| *byte == 0) {
        if chunk.is_empty() {
            continue;
        }
        let text = std::str::from_utf8(chunk).map_err(|_| String::from("path was not utf8"))?;
        files.push(PathBuf::from(text));
    }
    Ok(files)
}

fn rust_comment_offset(bytes: &[u8]) -> Option<usize> {
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes.get(index).copied();
        let second = bytes.get(index.saturating_add(1)).copied();
        match (first, second) {
            (Some(b'/'), Some(b'/')) | (Some(b'/'), Some(b'*')) | (Some(b'*'), Some(b'/')) => {
                return Some(index);
            }
            _ => index += 1,
        }
    }
    None
}

fn line_number(bytes: &[u8], offset: usize) -> usize {
    bytes
        .iter()
        .take(offset)
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn hash_comment_line(text: &str) -> Option<usize> {
    for (index, line) in text.lines().enumerate() {
        if line_has_hash_comment(line) {
            return Some(index + 1);
        }
    }
    None
}

fn line_has_hash_comment(line: &str) -> bool {
    let mut quote = None::<char>;
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if let Some(current_quote) = quote {
            if ch == '\\' {
                let _ = chars.next();
                continue;
            }
            if ch == current_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch == '#' {
            return true;
        }
    }
    false
}
