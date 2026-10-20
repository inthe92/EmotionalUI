//! User-authorized profiles. XML contributes identifiers/paths, never executable code.
use super::Plan;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    process::Child,
};
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
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub handoff: String,
    #[serde(default)]
    pub clock_confirmed: bool,
    pub launchers: Vec<Profile>,
}
impl Configuration {
    pub fn validate(&self) -> Result<(), String> {
        if self.handoff != "process-exit" || self.launchers.is_empty() || self.launchers.len() > 32
        {
            return Err("Expected process-exit handoff and 1..32 explicit launchers".into());
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.launchers {
            if p.id.is_empty()
                || !ids.insert(&p.id)
                || !p.waits_for_exit
                || !p.root.is_absolute()
                || !p.executable.is_absolute()
                || !p.working_directory.is_absolute()
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
            {
                return Err("Invalid launcher profile or foreground-process contract".into());
            }
        }
        Ok(())
    }
    pub fn resolve(&self, rom: &Path) -> io::Result<Resolved> {
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
        let (depth, profile, root) = candidates
            .first()
            .ok_or_else(|| io::Error::other("No authorized launcher for this ROM"))?;
        if candidates.get(1).is_some_and(|(d, _, _)| d == depth) {
            return Err(io::Error::other("Ambiguous launcher roots"));
        }
        let plan = Plan {
            executable: profile.executable.clone(),
            arguments: profile.arguments.clone(),
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
    pub fn spawn(&self) -> io::Result<Child> {
        // The frontend's private runtime library path must not replace emulator libraries.
        self.plan
            .command()?
            .current_dir(&self.profile.working_directory)
            .env_remove("LD_LIBRARY_PATH")
            .envs(&self.profile.environment)
            .spawn()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(root: PathBuf) -> Configuration {
        Configuration {
            handoff: "process-exit".into(),
            clock_confirmed: false,
            launchers: vec![Profile {
                id: "fixture".into(),
                root: root.clone(),
                executable: "/usr/bin/true".into(),
                arguments: vec!["{rom}".into()],
                working_directory: root,
                environment: BTreeMap::new(),
                waits_for_exit: true,
            }],
        }
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
