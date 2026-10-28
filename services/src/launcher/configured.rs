//! User-authorized profiles. XML contributes identifiers/paths, never executable code.
use super::Plan;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    process::Child,
};
pub const MAX_PROFILES: usize = 128;
pub const MAX_CONFIGURATION_BYTES: u64 = 512 * 1024;
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub root: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub waits_for_exit: bool,
    /// Empty keeps explicitly configured legacy profiles compatible. New
    /// dArkOS profiles restrict content by system root AND file extension.
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub resume_arguments: Vec<String>,
    /// Private saves/states created by the functional installer. Originals are
    /// copied once on first use, never replaced or written by this adapter.
    #[serde(default)]
    pub progress_root: Option<PathBuf>,
    #[serde(default)]
    pub save_source_directory: Option<PathBuf>,
    #[serde(default)]
    pub state_source_directory: Option<PathBuf>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub handoff: String,
    #[serde(default)]
    pub clock_confirmed: bool,
    #[serde(default)]
    pub system_options_root: Option<PathBuf>,
    pub launchers: Vec<Profile>,
}
impl Configuration {
    pub fn validate(&self) -> Result<(), String> {
        if self.handoff != "process-exit"
            || self.launchers.is_empty()
            || self.launchers.len() > MAX_PROFILES
        {
            return Err("Expected process-exit handoff and 1..128 explicit launchers".into());
        }
        if self.system_options_root.as_ref().is_some_and(|p| !p.is_absolute()) {
            return Err("Options root must be explicit and absolute".into());
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.launchers {
            if p.id.is_empty()
                || !ids.insert(&p.id)
                || !p.waits_for_exit
                || !p.root.is_absolute()
                || !p.executable.is_absolute()
                || !p.working_directory.is_absolute()
                || p.extensions.len() > 128
                || p.extensions.iter().any(|e| {
                    !e.starts_with('.')
                        || e.len() < 2
                        || e.len() > 24
                        || !e[1..].bytes().all(|b| b.is_ascii_alphanumeric())
                })
                || p.arguments.len() > 64
                || p.arguments.iter().filter(|a| a.as_str() == "{rom}").count() != 1
                || p.arguments.iter().any(|a| {
                    a.len() > 8192 || a.contains('\0') || (a.contains('{') && a != "{rom}")
                })
                || p.environment.iter().any(|(k, v)| {
                    k.is_empty()
                        || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        || v.contains('\0')
                        || v.len() > 8192
                })
                || (!p.resume_arguments.is_empty() && (p.resume_arguments.len() > 64
                    || p.resume_arguments.iter().filter(|a| a.as_str() == "{rom}").count() != 1
                    || p.resume_arguments.iter().any(|a| a.len() > 8192 || a.contains('\0') || (a.contains('{') && a != "{rom}"))))
                || [&p.progress_root, &p.save_source_directory, &p.state_source_directory].iter().any(|v| v.as_ref().is_some_and(|p| !p.is_absolute()))
            {
                return Err("Invalid launcher profile or foreground-process contract".into());
            }
        }
        Ok(())
    }
    pub fn resolve(&self, rom: &Path) -> io::Result<Resolved> {
        self.resolve_for(rom, false)
    }
    pub fn resolve_for(&self, rom: &Path, resume: bool) -> io::Result<Resolved> {
        self.validate().map_err(io::Error::other)?;
        let rom = rom.canonicalize()?;
        let mut candidates = self
            .launchers
            .iter()
            .filter_map(|p| {
                p.root
                    .canonicalize()
                    .ok()
                    .filter(|root| rom.starts_with(root))
                    .map(|root| (root.components().count(), p, root))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(depth, _, _)| std::cmp::Reverse(*depth));
        let depth = candidates
            .first()
            .map(|(depth, _, _)| *depth)
            .ok_or_else(|| io::Error::other("No authorized launcher for this ROM"))?;
        // Resolve the most specific system BEFORE testing formats: a nested
        // system must never fall back into an unrelated parent emulator.
        let mut matches = candidates.iter().filter(|(d, p, _)| {
            *d == depth
                && (p.extensions.is_empty()
                    || rom.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        p.extensions
                            .iter()
                            .any(|allowed| allowed[1..].eq_ignore_ascii_case(e))
                    }))
        });
        let (_, profile, root) = matches.next().ok_or_else(|| {
            io::Error::other("Este formato no está autorizado para el emulador de este sistema")
        })?;
        if matches.next().is_some() {
            return Err(io::Error::other("Ambiguous launcher roots/formats"));
        }
        let plan = Plan {
            executable: profile.executable.clone(),
            arguments: if resume && !profile.resume_arguments.is_empty() { profile.resume_arguments.clone() } else { profile.arguments.clone() },
            root: root.clone(),
            rom,
        };
        plan.validate()?;
        if !profile.working_directory.is_dir() {
            return Err(io::Error::other(
                "Launcher working directory is unavailable",
            ));
        }
        Ok(Resolved {
            profile: (*profile).clone(),
            plan,
        })
    }
}
pub struct Resolved {
    pub profile: Profile,
    pub plan: Plan,
}
impl Resolved {
    pub fn progress_directory(&self) -> io::Result<Option<PathBuf>> {
        let Some(base) = &self.profile.progress_root else { return Ok(None); };
        let base = base.canonicalize()?;
        let library = self.profile.root.canonicalize()?;
        if base.starts_with(&library) { return Err(io::Error::other("Progress must be outside the original library")); }
        // Stable, version-independent identity; same basenames in subfolders
        // must never share a save. The identity file also detects collisions.
        let relative = self.plan.rom.strip_prefix(&library).map_err(io::Error::other)?;
        let mut key = 0xcbf29ce484222325u64;
        for byte in relative.as_os_str().as_encoded_bytes() { key = (key ^ u64::from(*byte)).wrapping_mul(0x100000001b3); }
        Ok(Some(base.join("games").join(format!("{key:016x}"))))
    }
    pub fn seed_progress(&self) -> io::Result<()> {
        let Some(progress) = self.progress_directory()? else { return Ok(()); };
        let base = self.profile.progress_root.as_ref().unwrap().canonicalize()?;
        use std::os::unix::fs::DirBuilderExt;
        for directory in [base.join("games"), progress.clone(), progress.join("saves"), progress.join("states")] {
            if !directory.exists() { std::fs::DirBuilder::new().mode(0o700).create(&directory)?; }
            if directory.is_symlink() || !directory.canonicalize()?.starts_with(&base) { return Err(io::Error::other("Invalid private progress path")); }
        }
        owned_config(&progress.join("identity"), self.plan.rom.as_os_str().as_encoded_bytes())?;
        if let Some(index) = self.plan.arguments.iter().position(|a| a == "--appendconfig") {
            let source = Path::new(self.plan.arguments.get(index+1).ok_or_else(|| io::Error::other("Missing private config"))?);
            let source = source.canonicalize()?;
            if !source.starts_with(&base) || source.metadata()?.len() > 65536 { return Err(io::Error::other("Invalid private config template")); }
            let mut config: toml::Table = toml::from_str(&std::fs::read_to_string(&source)?).map_err(io::Error::other)?;
            for (key, folder) in [("savefile_directory","saves"),("savestate_directory","states")] {
                config.insert(key.into(), toml::Value::String(progress.join(folder).to_string_lossy().into()));
            }
            let content = toml::to_string(&config).map_err(io::Error::other)?;
            owned_config(&progress.join(source.file_name().unwrap()), content.as_bytes())?;
        }
        let stem = self.plan.rom.file_stem().ok_or_else(|| io::Error::other("No game basename"))?.to_string_lossy();
        let parent = self.plan.rom.parent().unwrap();
        for (source, target, states) in [
            (self.profile.save_source_directory.as_deref().unwrap_or(parent), progress.join("saves"), false),
            (self.profile.state_source_directory.as_deref().unwrap_or(parent), progress.join("states"), true),
        ] {
            let target = target.canonicalize()?;
            if !target.starts_with(&progress) { return Err(io::Error::other("Invalid private progress directory")); }
            let source = match source.canonicalize() { Ok(p) => p, Err(_) => continue };
            let Ok(entries) = std::fs::read_dir(source) else { continue };
            for entry in entries.take(100000) {
                let entry = entry?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                let Some(suffix) = name.strip_prefix(stem.as_ref()) else { continue };
                let accepted = if states {
                    suffix == ".state" || suffix == ".state.auto" || suffix.strip_prefix(".state").is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                } else { matches!(suffix, ".srm" | ".sav" | ".rtc") };
                if !accepted { continue; }
                let meta = entry.metadata()?;
                if entry.file_type()?.is_symlink() || !meta.is_file() || meta.len() > 64 * 1024 * 1024 { continue; }
                let destination = target.join(name);
                if destination.exists() || destination.is_symlink() { continue; }
                let mut input = std::fs::File::open(entry.path())?;
                let mut output = match std::fs::OpenOptions::new().write(true).create_new(true).open(&destination) {
                    Ok(f) => f,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(e),
                };
                if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
                    drop(output);
                    let _ = std::fs::remove_file(destination);
                    return Err(error);
                }
            }
        }
        Ok(())
    }
    pub fn spawn(&self) -> io::Result<Child> {
        // The frontend's private runtime library path must not replace emulator libraries.
        let mut arguments = self.plan.arguments.clone();
        if let Some(progress) = self.progress_directory()? {
            if let Some(index) = arguments.iter().position(|a| a == "--appendconfig") {
                let original = Path::new(&arguments[index+1]);
                let private = progress.join(original.file_name().ok_or_else(|| io::Error::other("Missing private config name"))?);
                if !private.is_file() { return Err(io::Error::other("Private progress was not prepared")); }
                arguments[index+1] = private.to_string_lossy().into();
            }
        }
        Plan { executable: self.plan.executable.clone(), root: self.plan.root.clone(), rom: self.plan.rom.clone(), arguments }
            .command()?
            .current_dir(&self.profile.working_directory)
            .env_remove("LD_LIBRARY_PATH")
            .envs(&self.profile.environment)
            .spawn()
    }
}

fn owned_config(path: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if path.is_symlink() { return Err(io::Error::other("Symlink private config refused")); }
    if path.exists() {
        if std::fs::read(path)? != content { return Err(io::Error::other("Private progress identity/config changed; preserved without overwriting")); }
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    file.write_all(content)?;
    file.sync_all()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(root: PathBuf) -> Configuration {
        Configuration {
            handoff: "process-exit".into(),
            clock_confirmed: false,
            system_options_root: None,
            launchers: vec![Profile {
                id: "fixture".into(),
                root: root.clone(),
                executable: "/usr/bin/true".into(),
                arguments: vec!["{rom}".into()],
                working_directory: root,
                environment: BTreeMap::new(),
                waits_for_exit: true,
                extensions: vec![],
                resume_arguments: vec![],
                progress_root: None,
                save_source_directory: None,
                state_source_directory: None,
            }],
        }
    }
    #[test]
    fn resume_uses_its_own_arguments_and_seed_never_overwrites_progress_or_originals() {
        let root = std::env::temp_dir().join(format!("eui-resume-seed-{}", std::process::id()));
        let library = root.join("roms");
        let progress = root.join("private");
        for directory in [&library, &progress.join("saves"), &progress.join("states")] { std::fs::create_dir_all(directory).unwrap(); }
        let rom = library.join("game.sfc");
        std::fs::write(&rom, b"synthetic content").unwrap();
        std::fs::write(library.join("game.srm"), b"original save").unwrap();
        std::fs::write(library.join("game.state.auto"), b"original state").unwrap();
        std::fs::write(library.join("other.srm"), b"unrelated").unwrap();
        let mut c = config(library.clone());
        c.launchers[0].progress_root = Some(progress.clone());
        let template = progress.join("isolation.cfg");
        let resume_template = progress.join("resume.cfg");
        std::fs::write(&template, "savefile_directory='unused'\nsavestate_directory='unused'\nsavestate_auto_load='false'\n").unwrap();
        std::fs::write(&resume_template, "savefile_directory='unused'\nsavestate_directory='unused'\nsavestate_auto_load='true'\n").unwrap();
        c.launchers[0].arguments = vec!["--appendconfig".into(), template.to_string_lossy().into(), "{rom}".into()];
        c.launchers[0].resume_arguments = vec!["--appendconfig".into(), resume_template.to_string_lossy().into(), "--resume-fixture".into(), "{rom}".into()];
        let normal = c.resolve(&rom).unwrap();
        assert_eq!(normal.plan.arguments[0], "--appendconfig");
        normal.seed_progress().unwrap();
        let game_progress = normal.progress_directory().unwrap().unwrap();
        assert_eq!(std::fs::read(game_progress.join("saves/game.srm")).unwrap(), b"original save");
        assert!(!game_progress.join("saves/other.srm").exists());
        std::fs::write(game_progress.join("saves/game.srm"), b"new private progress").unwrap();
        let resume = c.resolve_for(&rom, true).unwrap();
        assert!(resume.plan.arguments.contains(&"--resume-fixture".to_string()));
        resume.seed_progress().unwrap();
        let prepared: toml::Table = toml::from_str(&std::fs::read_to_string(game_progress.join("resume.cfg")).unwrap()).unwrap();
        assert_eq!(prepared["savefile_directory"].as_str().unwrap(), game_progress.join("saves").to_str().unwrap());
        assert_eq!(prepared["savestate_auto_load"].as_str().unwrap(), "true");
        assert_eq!(std::fs::read(game_progress.join("saves/game.srm")).unwrap(), b"new private progress");
        assert_eq!(std::fs::read(library.join("game.srm")).unwrap(), b"original save");
        assert_eq!(std::fs::read(game_progress.join("states/game.state.auto")).unwrap(), b"original state");
        std::fs::create_dir_all(library.join("another-region")).unwrap();
        let alternate = library.join("another-region/game.sfc");
        std::fs::write(&alternate, b"alternate fixture").unwrap();
        let alternate = c.resolve(&alternate).unwrap();
        alternate.seed_progress().unwrap();
        assert_ne!(alternate.progress_directory().unwrap().unwrap(), game_progress);
        assert!(!alternate.progress_directory().unwrap().unwrap().join("saves/game.srm").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn explicit_profiles_and_missing_rom() {
        let p = std::env::temp_dir();
        let c = config(p.clone());
        assert!(c.validate().is_ok());
        assert!(c.resolve(&p.join("missing-emotional-rom")).is_err());
    }
    #[test]
    fn shell_templates_and_background_contract_are_not_imported() {
        let mut c = config(std::env::temp_dir());
        c.launchers[0].arguments = vec!["%ROM%; sudo perfnorm".into()];
        assert!(c.validate().is_err());
        c.launchers[0].arguments = vec!["{rom}".into()];
        c.launchers[0].waits_for_exit = false;
        assert!(c.validate().is_err());
    }
    #[test]
    fn unknown_or_ambiguous_root_is_rejected() {
        let root = std::env::temp_dir().join(format!("emotional-resolve-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let rom = root.join("test;$(echo NO).fixture");
        std::fs::write(&rom, b"not a ROM").unwrap();
        let mut c = config(root.clone());
        assert!(c
            .resolve(&rom)
            .unwrap()
            .spawn()
            .unwrap()
            .wait()
            .unwrap()
            .success());
        let mut p = c.launchers[0].clone();
        p.id = "other".into();
        c.launchers.push(p);
        assert!(c.resolve(&rom).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn compressed_formats_route_by_system_and_not_extension_alone() {
        let root =
            std::env::temp_dir().join(format!("emotional-multisystem-{}", std::process::id()));
        let mut c = config(root.join("snes"));
        c.launchers[0].extensions = vec![".zip".into(), ".sfc".into()];
        let mut gba = c.launchers[0].clone();
        gba.id = "gba".into();
        gba.root = root.join("gba");
        gba.working_directory = gba.root.clone();
        gba.extensions = vec![".zip".into(), ".gba".into()];
        c.launchers.push(gba);
        for system in ["snes", "gba"] {
            let dir = root.join(system);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("game.ZIP"), b"synthetic archive").unwrap();
        }
        assert_eq!(
            c.resolve(&root.join("snes/game.ZIP")).unwrap().profile.id,
            "fixture"
        );
        assert_eq!(
            c.resolve(&root.join("gba/game.ZIP")).unwrap().profile.id,
            "gba"
        );
        std::fs::write(root.join("snes/not-a-game.sh"), b"untrusted script").unwrap();
        assert!(c.resolve(&root.join("snes/not-a-game.sh")).is_err());
        c.launchers[0].extensions.push(".zip;sh".into());
        assert!(c.validate().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn full_device_inventory_is_bounded_and_old_profiles_remain_compatible() {
        let mut c = config(std::env::temp_dir());
        for i in 1..MAX_PROFILES {
            let mut p = c.launchers[0].clone();
            p.id = format!("system-{i}");
            c.launchers.push(p);
        }
        assert!(c.validate().is_ok());
        let mut p = c.launchers[0].clone();
        p.id = "overflow".into();
        c.launchers.push(p);
        assert!(c.validate().is_err());
        let old = "handoff='process-exit'\n[[launchers]]\nid='old'\nroot='/tmp'\nexecutable='/usr/bin/true'\narguments=['{rom}']\nworking_directory='/tmp'\nwaits_for_exit=true";
        let parsed: Configuration = toml::from_str(old).unwrap();
        assert!(parsed.launchers[0].extensions.is_empty());
        assert!(parsed.validate().is_ok());
    }
    #[test]
    fn disc_and_cartridge_formats_can_use_disjoint_profiles_in_one_system() {
        let root =
            std::env::temp_dir().join(format!("emotional-disc-route-{}", std::process::id()));
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let mut c = config(root.clone());
        c.launchers[0].extensions = vec![".zip".into()];
        let mut disc = c.launchers[0].clone();
        disc.id = "neocd".into();
        disc.extensions = vec![".chd".into()];
        c.launchers.push(disc);
        for name in ["cart.zip", "disc.CHD", "nested/cart.zip"] {
            std::fs::write(root.join(name), b"synthetic content").unwrap();
        }
        assert_eq!(
            c.resolve(&root.join("cart.zip")).unwrap().profile.id,
            "fixture"
        );
        assert_eq!(
            c.resolve(&root.join("disc.CHD")).unwrap().profile.id,
            "neocd"
        );
        let mut child = c.launchers[1].clone();
        child.id = "nested-disc-only".into();
        child.root = root.join("nested");
        c.launchers.push(child);
        assert!(c.resolve(&root.join("nested/cart.zip")).is_err());
        c.launchers[1].extensions.push(".zip".into());
        assert!(c.resolve(&root.join("cart.zip")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LaunchRecord {
    pub id: String,
    pub game_id: String,
    pub profile: String,
    pub started_utc: Option<i64>,
    pub elapsed_ms: u64,
    pub outcome: String,
    pub exit_code: Option<i32>,
    pub session_registered: bool,
}
