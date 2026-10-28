//! Process boundary: supervisor never initializes Slint/evdev/audio.
//! A frontend must exit successfully before a game process can be spawned.
use crate::store::Store;
use emotional_services::launcher::configured::{Configuration, LaunchRecord};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub game_id: String,
    #[serde(default)]
    pub recommended: bool,
    pub resume: bool,
    pub selected: usize,
    pub library: bool,
    pub library_index: usize,
    #[serde(default)]
    pub library_view: Option<crate::library_browser::Saved>,
    #[serde(default)]
    pub system_option: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Return {
    pub position: Request,
    pub message: String,
}
fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|v| v[0] == name).map(|v| v[1].clone())
}
pub fn frame_bridge_enabled() -> bool {
    cfg!(all(target_os = "linux", feature = "gpu-material", not(feature = "simulator")))
        && std::env::var_os("EMOTIONAL_LAUNCH_FRAMEBUFFER").as_deref() == Some(std::ffi::OsStr::new("1"))
}
fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T, Box<dyn Error>> {
    read_bounded(p, 65536)
}
fn read_bounded<T: serde::de::DeserializeOwned>(p: &Path, limit: u64) -> Result<T, Box<dyn Error>> {
    let m = fs::symlink_metadata(p)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > limit {
        return Err("Invalid bounded launch document".into());
    }
    Ok(toml::from_str(&fs::read_to_string(p)?)?)
}
pub fn write<T: Serialize>(p: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(p)?;
    f.write_all(toml::to_string(value)?.as_bytes())?;
    f.sync_all()?;
    Ok(())
}
pub fn child_directory() -> Option<PathBuf> {
    option(&std::env::args().collect::<Vec<_>>(), "--launch-child").map(PathBuf::from)
}
pub fn returned(dir: &Path) -> Option<Return> {
    read(&dir.join("return.toml")).ok()
}
struct OwnedDirectory(PathBuf);
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        for n in ["request.toml", "return.toml", "owner", "active-game", "launch.rgba"] {
            let _ = fs::remove_file(self.0.join(n));
        }
        let _ = fs::remove_dir(&self.0);
    }
}
fn directory(path: PathBuf) -> Result<OwnedDirectory, Box<dyn Error>> {
    fs::DirBuilder::new().mode(0o700).create(&path)?;
    fs::write(path.join("owner"), std::process::id().to_string())?;
    Ok(OwnedDirectory(path))
}
pub fn dispatch() -> Result<bool, Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|v| v == "--trial") && args.iter().any(|v| v == "--functional") {
        return Err("Trial mode cannot enable functional system controls".into());
    }
    if args.iter().any(|v| v == "--trial")
        && args
            .iter()
            .any(|v| v == "--launcher-config" || v == "--launch-child")
    {
        return Err("Trial preview does not authorize emulator launch".into());
    }
    if args.iter().any(|v| v == "--launch-child") {
        return Ok(false);
    }
    let Some(config) = option(&args, "--launcher-config") else {
        return Ok(false);
    };
    #[cfg(all(
        target_os = "linux",
        not(feature = "desktop"),
        not(feature = "simulator")
    ))]
    if !args.iter().any(|a| a == "--launch-physical-supervised") {
        return Err(
            "KMS launch requires supervised physical recovery; no device changes performed".into(),
        );
    }
    if !cfg!(feature = "simulator") && option(&args, "--sim-return-replay").is_some() {
        return Err("Return replays require the simulator feature".into());
    }
    if args.iter().any(|a| a == "--ui-test") {
        return Err(
            "Automatic UI self-tests cannot run with a configured launcher; use controlled replays"
                .into(),
        );
    }
    let config: Configuration = read_bounded(
        Path::new(&config),
        emotional_services::launcher::configured::MAX_CONFIGURATION_BYTES,
    )?;
    config.validate()?;
    let db = PathBuf::from(
        option(&args, "--db").ok_or("Launcher requires an explicit persistent --db path")?,
    );
    if !db.is_absolute() || !args.iter().any(|v| v == "--library-root") {
        return Err("Launcher requires absolute database and explicit library root".into());
    }
    if let Some(p) = db.parent() {
        fs::create_dir_all(p)?;
    }
    let mut store = Store::open(&db)?;
    let canonical = db.canonicalize()?;
    let lock = directory(canonical.with_extension("launch-lock"))
        .map_err(|_| "Launch supervisor already active or stale lock; inspect before removing")?;
    println!("SUPERVISOR recovery_lock={}", lock.0.display());
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let ipc = directory(
        std::env::temp_dir().join(format!("emotional-launch-{}-{nonce}", std::process::id())),
    )?;
    let executable = std::env::current_exe()?;
    let mut next: Option<Return> = None;
    loop {
        let _ = fs::remove_file(ipc.0.join("launch.rgba"));
        let returning = next.is_some();
        if let Some(value) = next.take() {
            write(&ipc.0.join("return.toml"), &value)?;
        }
        let mut frontend_args = Vec::new();
        let mut at = 0;
        while at < args.len() {
            if returning
                && (args[at] == "--sim-replay"
                    || args[at] == "--smoke-log"
                    || args[at] == "--audio-mode"
                    || args[at] == "--audio-volume")
            {
                at += 2;
                continue;
            }
            if returning && args[at] == "--demo-history" {
                at += 1;
                continue;
            }
            frontend_args.push(args[at].clone());
            at += 1;
        }
        if returning {
            if let Some(replay) = option(&args, "--sim-return-replay") {
                frontend_args.extend(["--sim-replay".into(), replay]);
            }
        }
        let mut frontend = Command::new(&executable)
            .args(&frontend_args)
            .arg("--launch-child")
            .arg(&ipc.0)
            .spawn()?;
        let frontend_pid = frontend.id();
        let status = frontend.wait()?;
        println!(
            "FRONTEND exited pid={frontend_pid} success={}",
            status.success()
        );
        let _ = fs::remove_file(ipc.0.join("return.toml"));
        let request_path = ipc.0.join("request.toml");
        if !request_path.exists() {
            if !status.success() {
                return Err("Frontend failed; independent system recovery may be required".into());
            }
            break;
        }
        let position: Request = read(&request_path)?;
        fs::remove_file(request_path)?;
        if !status.success() {
            return Err("Frontend did not exit cleanly; game launch refused".into());
        }
        if position.game_id.len() > 4096
            || position.selected > if position.system_option.is_some() { 4096 } else { 5 }
            || position.library_index > 1000000
            || position.library_view.as_ref().is_some_and(|view|!view.valid())
        {
            return Err("Invalid launch request".into());
        }
        if let Some(tool) = &position.system_option {
            if !emotional_services::system::frontend_is_stopped() {
                // A tool must not compete with ES, including while it is starting.
                break;
            }
            let console_return = config.system_options_root.as_deref()
                .and_then(|root| emotional_services::system_options::returns_to_console(root, tool).ok())
                .unwrap_or(true);
            let child = config.system_options_root.as_deref()
                .ok_or_else(|| std::io::Error::other("No installed Options root configured"))
                .and_then(|root| emotional_services::system_options::spawn(root, tool));
            let result = match child {
                Err(error) => Err(error),
                Ok(mut child) => {
                    let _ = fs::write(lock.0.join("active-game"), child.id().to_string());
                    match child.wait() {
                        Err(_) => {
                            std::mem::forget(lock);
                            return Err("Cannot confirm Options tool termination; frontend restart refused".into());
                        }
                        Ok(status) => {
                            let _ = fs::remove_file(lock.0.join("active-game"));
                            if status.success() { Ok(()) } else { Err(std::io::Error::other("The original tool reported an error")) }
                        }
                    }
                }
            };
            if console_return || !emotional_services::system::frontend_is_stopped() {
                // Some stock tools intentionally restart ES. The external
                // recovery unit owns that return; do not restart Slint over it.
                println!("OPTIONS handed back to the original frontend");
                break;
            }
            next = Some(Return { position, message: if result.is_ok() {
                "De vuelta en Ajustes.".into()
            } else {
                "No se pudo completar la herramienta. Puedes volver a intentarlo.".into()
            }});
            continue;
        }
        let attempt = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string();
        let mut record = LaunchRecord {
            id: attempt,
            game_id: position.game_id.clone(),
            profile: String::new(),
            started_utc: None,
            elapsed_ms: 0,
            outcome: "spawn-error".into(),
            exit_code: None,
            session_registered: false,
        };
        // The UI process is reaped. No graphical, input or audio owner survives in it.
        let mut local_start = None;
        let mut spawned = false;
        let mut reaped = false;
        #[cfg(target_os = "linux")]
        let frame = if frame_bridge_enabled() {
            use std::io::Read;
            let path=ipc.0.join("launch.rgba");
            let frame=(|| -> std::io::Result<emotional_services::launch_frame::Held> {
                let metadata=fs::symlink_metadata(&path)?;
                if !metadata.is_file() || metadata.len()!=emotional_services::launch_frame::FRAME_BYTES as u64 {
                    return Err(std::io::Error::other("Invalid bounded launch frame"));
                }
                let mut bytes=Vec::new();
                fs::File::open(path)?.take(emotional_services::launch_frame::FRAME_BYTES as u64+1).read_to_end(&mut bytes)?;
                emotional_services::launch_frame::Held::show(&bytes)
            })();
            match frame {Ok(frame)=>Some(frame),Err(error)=>{eprintln!("LAUNCH_FRAME unavailable: {error}");None}}
        } else {None};
        let result = (|| -> Result<(), Box<dyn Error>> {
            let rom = store
                .launch_rom(&position.game_id)?
                .ok_or("Game identifier is not in the imported library")?;
            let resolved = config.resolve_for(&rom, position.resume)?;
            resolved.seed_progress()?;
            record.profile = resolved.profile.id.clone();
            if option(&args, "--quit-file").is_some_and(|p| Path::new(&p).exists()) {
                record.outcome = "cancelled".into();
                return Err("Launch cancelled before spawn".into());
            }
            let utc = chrono::Utc::now();
            if config.clock_confirmed && (2020..=2100).contains(&chrono::Datelike::year(&utc)) {
                record.started_utc = Some(utc.timestamp());
                local_start = Some(utc.with_timezone(&chrono::Local).naive_local());
            }
            let now = Instant::now();
            let mut game = resolved.spawn()?;
            spawned = true;
            if let Err(error) = fs::write(lock.0.join("active-game"), game.id().to_string()) {
                eprintln!("Cannot write recovery PID: {error}");
            }
            println!(
                "LAUNCH started pid={} profile={}; frontend_exited=true",
                game.id(),
                record.profile
            );
            let result = game.wait()?;
            reaped = true;
            let _ = fs::remove_file(lock.0.join("active-game"));
            record.elapsed_ms = now.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            record.exit_code = result.code();
            record.outcome = if !result.success() {
                "exit-error"
            } else if record.elapsed_ms < 2000 {
                "early-exit"
            } else {
                "exited-success"
            }
            .into();
            record.session_registered =
                record.outcome == "exited-success" && record.started_utc.is_some();
            if record.outcome != "exited-success" {
                return Err(
                    "The configured process did not complete a sustained successful session".into(),
                );
            }
            Ok(())
        })();
        if spawned && !reaped {
            #[cfg(target_os = "linux")]
            std::mem::forget(frame); // Never restore over an emulator of uncertain lifetime.
            std::mem::forget(lock); // Preserve recovery PID and refuse another supervisor.
            return Err("Cannot confirm game termination; frontend restart refused, independent recovery required".into());
        }
        #[cfg(target_os = "linux")]
        drop(frame);
        // Keep attempts, actual process duration and reliable dated sessions distinct.
        let storage = store.record_launch_context(&record,position.recommended,local_start);
        println!(
            "LAUNCH result={} elapsed_ms={} session_registered={}",
            record.outcome, record.elapsed_ms, record.session_registered
        );
        next = Some(Return {
            position,
            message: if let Err(error) = storage {
                format!("El juego terminó, pero no pudimos guardar la sesión: {error}")
            } else if let Err(error) = result {
                eprintln!("Launch: {error}");
                "No pudimos iniciar o completar el juego. Puedes volver a intentarlo.".into()
            } else {
                "De vuelta en tu habitación.".into()
            },
        });
    }
    Ok(true)
}
