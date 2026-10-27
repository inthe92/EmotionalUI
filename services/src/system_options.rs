//! Installed dArkOS Options, treated as explicit tools, never ROM metadata.
//! Discovery is read-only; execution belongs to the supervisor after UI exit.
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
};

#[derive(Clone, Debug)]
pub struct Entry {
    pub relative: String,
    pub title: String,
}

pub fn resolve(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let path = Path::new(relative);
    if relative.len() > 4096
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("sh"))
    {
        return Err(io::Error::other("Invalid Options entry"));
    }
    let root = root.canonicalize()?;
    let candidate = root.join(path).canonicalize()?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err(io::Error::other(
            "Options entry leaves the installed tools directory",
        ));
    }
    Ok(candidate)
}

pub fn discover(root: &Path) -> io::Result<Vec<Entry>> {
    let root = root.canonicalize()?;
    let mut pending = vec![(root.clone(), 0)];
    let mut result = Vec::new();
    let mut visited = 0;
    let mut labels = std::collections::HashMap::new();
    let mut hidden = Vec::new();
    while let Some((directory, depth)) = pending.pop() {
        let metadata = directory.join("gamelist.xml");
        if metadata.is_file() {
            if metadata.metadata()?.len() > 2 * 1024 * 1024 {
                return Err(io::Error::other("Options metadata exceeds its bound"));
            }
            let text = fs::read_to_string(metadata)?;
            let document = roxmltree::Document::parse_with_options(
                &text,
                roxmltree::ParsingOptions {
                    allow_dtd: false,
                    nodes_limit: 30000,
                },
            )
            .map_err(io::Error::other)?;
            for entry in document
                .root_element()
                .children()
                .filter(|n| n.has_tag_name("game") || n.has_tag_name("folder"))
            {
                let value = |name| {
                    entry
                        .children()
                        .find(|n| n.has_tag_name(name))
                        .and_then(|n| n.text())
                        .unwrap_or("")
                };
                let path = directory.join(value("path"));
                let Ok(path) = path.canonicalize() else {
                    continue;
                };
                if !path.starts_with(&root) {
                    continue;
                }
                if value("hidden").eq_ignore_ascii_case("true") {
                    hidden.push(path);
                } else if !value("name").trim().is_empty() {
                    labels.insert(
                        path,
                        value("name").trim().chars().take(256).collect::<String>(),
                    );
                }
            }
        }
        for item in fs::read_dir(directory)? {
            visited += 1;
            if visited > 4096 {
                return Err(io::Error::other("Options inventory exceeds its bound"));
            }
            let item = item?;
            if item.file_name().to_string_lossy().starts_with('.')
                || hidden.iter().any(|p| item.path().starts_with(p))
            {
                continue;
            }
            let kind = item.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() && depth < 6 {
                pending.push((item.path(), depth + 1));
            } else if kind.is_file()
                && item
                    .path()
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("sh"))
            {
                let path = item.path();
                let relative = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                resolve(&root, &relative)?;
                let fallback = Path::new(&relative)
                    .with_extension("")
                    .to_string_lossy()
                    .replace('/', " › ");
                let title = labels.get(&path).cloned().unwrap_or(fallback);
                result.push(Entry { relative, title });
            }
        }
    }
    result.sort_by_cached_key(|e| e.title.to_lowercase());
    Ok(result)
}

pub fn spawn(root: &Path, relative: &str) -> io::Result<Child> {
    if !discover(root)?
        .iter()
        .any(|entry| entry.relative == relative)
    {
        return Err(io::Error::other(
            "Options entry is hidden or no longer available",
        ));
    }
    let script = resolve(root, relative)?;
    // Preserve the original tools' Linux console interface. No shell command
    // from XML, sudo shell, permission changes or graphical owner survives here.
    let terminal = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty1")?;
    Command::new("/bin/bash")
        .arg(&script)
        .current_dir(script.parent().unwrap())
        .env_remove("LD_LIBRARY_PATH")
        .env("TERM", "linux")
        .stdin(Stdio::from(terminal.try_clone()?))
        .stdout(Stdio::from(terminal.try_clone()?))
        .stderr(Stdio::from(terminal))
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_options_include_nested_tools_but_reject_escape_and_non_scripts() {
        let root = std::env::temp_dir().join(format!("eui-options-{}", std::process::id()));
        fs::create_dir_all(root.join("Advanced")).unwrap();
        fs::write(root.join("Wifi.sh"), "exit 0").unwrap();
        fs::write(root.join("Advanced/Backup Settings.sh"), "exit 0").unwrap();
        fs::write(root.join("README.txt"), "data").unwrap();
        fs::create_dir_all(root.join("Internal")).unwrap();
        fs::write(root.join("Internal/Helper.sh"), "exit 0").unwrap();
        fs::write(root.join("gamelist.xml"), "<gameList><game><path>Wifi.sh</path><name>Red Wi-Fi</name></game><folder><path>Internal</path><hidden>true</hidden></folder></gameList>").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/bin/sh", root.join("outside.sh")).unwrap();
        let entries = discover(&root).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.title == "Red Wi-Fi"));
        assert!(entries
            .iter()
            .any(|e| e.title == "Advanced › Backup Settings"));
        for path in ["../outside.sh", "/bin/sh", "README.txt", "outside.sh", ""] {
            assert!(resolve(&root, path).is_err(), "{path}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
