//! Platform boundary for explicit process launch profiles.
//! Launch argv directly; never interpret EmulationStation shell command templates.
pub mod configured;
use std::{
    io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
#[derive(Debug)]
pub struct Plan {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub rom: PathBuf,
    pub root: PathBuf,
}
impl Plan {
    pub fn validate(&self) -> io::Result<PathBuf> {
        let root = self.root.canonicalize()?;
        let rom = self.rom.canonicalize()?;
        if !self.executable.is_absolute()
            || !self.executable.is_file()
            || !root.is_dir()
            || !rom.is_file()
            || !rom.starts_with(root)
            || self.arguments.len() > 64
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid explicit launcher plan",
            ));
        }
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        let mut header = [0u8; 4];
        std::fs::File::open(&self.executable)?.read_exact(&mut header)?;
        let known = header == *b"\x7fELF"
            || header[..2] == *b"#!"
            || [
                0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe, 0xcafebabe, 0xbebafeca,
            ]
            .contains(&u32::from_be_bytes(header));
        if !known || std::fs::metadata(&self.executable)?.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::other(
                "Launcher must be a recognized executable or an explicit shebang script",
            ));
        }
        Ok(rom)
    }
    /// Caller must release graphics and controller ownership before using this boundary.
    pub fn command(&self) -> io::Result<Command> {
        let rom = self.validate()?;
        let args = self.arguments.iter().map(|a| {
            if a == "{rom}" {
                rom.as_os_str().to_os_string()
            } else {
                a.into()
            }
        });
        let mut command = Command::new(&self.executable);
        command.args(args).stdin(Stdio::null());
        Ok(command)
    }
    pub fn spawn(&self) -> io::Result<Child> {
        self.command()?.spawn()
    }
}
/// Allows adapters to inspect local art without trusting an XML-provided absolute path.
pub fn local_resource(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let p = path.canonicalize().ok()?;
    (p.is_file() && p.starts_with(root)).then_some(p)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_child_and_unavailable_rom() {
        let root = std::env::temp_dir().join(format!("emotional-launch-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let rom = root.join("fiction;$(never-run).test");
        std::fs::write(&rom, b"synthetic data, not a ROM").unwrap();
        let p = Plan {
            executable: "/usr/bin/true".into(),
            arguments: vec!["{rom}".into()],
            rom: rom.clone(),
            root: root.clone(),
        };
        assert!(p.spawn().unwrap().wait().unwrap().success());
        std::fs::remove_file(rom).unwrap();
        assert!(p.spawn().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn paths_cannot_escape_authorized_root() {
        let root = std::env::temp_dir();
        assert!(local_resource(&root, Path::new("/etc/passwd")).is_none());
        let p = Plan {
            executable: "true".into(),
            arguments: vec![],
            root: root.clone(),
            rom: root,
        };
        assert!(p.validate().is_err());
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;
    #[test]
    fn plain_text_never_falls_back_to_a_shell() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("emotional-no-shell-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("program");
        std::fs::write(&exe, b"echo should-not-run").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let rom = dir.join("fixture");
        std::fs::write(&rom, b"original fixture").unwrap();
        let p = Plan {
            executable: exe,
            arguments: vec!["{rom}".into()],
            rom,
            root: dir.clone(),
        };
        assert!(p.spawn().is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
