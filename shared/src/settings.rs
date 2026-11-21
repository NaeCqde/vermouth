//! Where the settings files are read from: the current directory first; only when it has no such file, the directory of the
//! executable. (The one that is found is the only one that is read.)

use std::path::PathBuf;

/// The directory of the running executable.
fn exe_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    exe.parent().map(|d| d.to_path_buf())
}

/// The path of the settings file `name`: `./name` when it exists, else `<executable directory>/name` when that exists, else
/// `./name` (so that an error names the usual place).
pub fn find(name: &str) -> PathBuf {
    let cwd = PathBuf::from(name);
    if cwd.is_file() {
        return cwd;
    }
    match exe_dir().map(|d| d.join(name)) {
        Some(p) if p.is_file() => p,
        _ => cwd,
    }
}

/// Reads `.env` (found as `find` does) into the environment; variables that are already set are kept.
pub fn load_env() {
    let path = find(".env");
    if path.is_file() {
        let _ = dotenvy::from_path(path);
    }
}
