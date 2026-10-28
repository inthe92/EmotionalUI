//! Small Linux system adapter. Work and commands stay off the graphical thread.
//!
//! No network credentials, SSIDs or Bluetooth addresses are retained or logged.
//! Missing capabilities are reported as unknown/unavailable, never simulated.
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_secs(10);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_OUTPUT: u64 = 32 * 1024;
/// A test action cannot turn off the LCD. This is not a global OS setting.
const TEST_BRIGHTNESS_MIN_PERCENT: u8 = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BatteryState {
    #[default]
    Unknown,
    Charging,
    Discharging,
    Full,
    NotCharging,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Battery {
    pub percent: Option<u8>,
    pub state: BatteryState,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WifiState {
    #[default]
    Unknown,
    Connected,
    Disconnected,
    Disabled,
    Unavailable,
    Error,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wifi {
    pub state: WifiState,
    pub signal: Option<u8>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BluetoothState {
    #[default]
    Unknown,
    /// Adapter exists, but its BlueZ service is inactive; radio power is unknown.
    ServiceInactive,
    Enabled,
    Disabled,
    Unavailable,
    Error,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bluetooth {
    pub state: BluetoothState,
    pub connected: Option<bool>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Brightness {
    pub requested_percent: Option<u8>,
    pub applied_percent: Option<u8>,
    pub maximum: Option<u32>,
    pub writable: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub battery: Battery,
    pub wifi: Wifi,
    pub bluetooth: Bluetooth,
    pub volume: Option<u8>,
    pub volume_muted: Option<bool>,
    pub brightness: Brightness,
    pub allow_changes: bool,
    /// An established local SSH connection, or an unreadable Linux TCP table.
    /// Only a boolean is retained; addresses never leave this worker.
    pub remote_session: bool,
    /// Fixed, user-readable feedback. Never command output or private metadata.
    pub message: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Refresh,
    Volume(u8),
    Brightness { percent: u8, safe_test: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestError {
    ReadOnly,
    InvalidPercent,
    Busy,
    Unavailable,
}
impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ReadOnly => "Modo prueba: los controles del sistema son de solo lectura.",
            Self::InvalidPercent => "El porcentaje debe estar entre 0 y 100.",
            Self::Busy => "El sistema sigue aplicando una operación. Inténtalo de nuevo.",
            Self::Unavailable => "Los controles del sistema no están disponibles.",
        })
    }
}
impl std::error::Error for RequestError {}

/// Sampling is asynchronous. `latest` and `request` never wait for a command.
pub struct Runtime {
    sender: SyncSender<Action>,
    snapshot: Arc<Mutex<Option<Snapshot>>>,
    stopped: Arc<AtomicBool>,
    allow_changes: bool,
    available: bool,
}
impl Runtime {
    pub fn start(allow_changes: bool) -> Self {
        Self::start_impl(allow_changes, None)
    }
    /// Notify only after publishing a changed snapshot; callback must not block.
    pub fn start_with_notify(
        allow_changes: bool,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self::start_impl(allow_changes, Some(Arc::new(notify)))
    }
    fn start_impl(allow_changes: bool, notify: Option<Arc<dyn Fn() + Send + Sync>>) -> Self {
        let worker_notify = notify.clone();
        let (sender, receiver) = mpsc::sync_channel(4);
        let snapshot = Arc::new(Mutex::new(None));
        let stopped = Arc::new(AtomicBool::new(false));
        let available = cfg!(target_os = "linux");
        let mut runtime = Self {
            sender,
            snapshot: snapshot.clone(),
            stopped: stopped.clone(),
            allow_changes,
            available,
        };
        if available {
            if thread::Builder::new()
                .name("system-state".into())
                .spawn(move || {
                    worker(
                        Probe::new(Paths::linux(), SystemRunner),
                        receiver,
                        snapshot,
                        stopped,
                        allow_changes,
                        worker_notify,
                    )
                })
                .is_err()
            {
                runtime.available = false;
            }
        }
        if !runtime.available {
            *runtime.snapshot.lock().unwrap_or_else(|e| e.into_inner()) = Some(Snapshot {
                wifi: Wifi {
                    state: WifiState::Unavailable,
                    signal: None,
                },
                bluetooth: Bluetooth {
                    state: BluetoothState::Unavailable,
                    connected: None,
                },
                // Desktop simulation never modifies the host's mixer or LCD.
                allow_changes: false,
                ..Default::default()
            });
        }
        if !runtime.available {
            if let Some(notify) = notify {
                notify();
            }
        }
        runtime
    }
    pub fn latest(&self) -> Option<Snapshot> {
        self.snapshot.try_lock().ok().and_then(|v| v.clone())
    }
    pub fn request(&self, action: Action) -> Result<(), RequestError> {
        validate_action(action, self.allow_changes)?;
        if !self.available {
            return Err(RequestError::Unavailable);
        }
        self.sender.try_send(action).map_err(|e| match e {
            TrySendError::Full(_) => RequestError::Busy,
            TrySendError::Disconnected(_) => RequestError::Unavailable,
        })
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        // Wake the worker without joining (and blocking) the graphical thread.
        let _ = self.sender.try_send(Action::Refresh);
    }
}
fn validate_action(action: Action, allow_changes: bool) -> Result<(), RequestError> {
    match action {
        Action::Refresh => Ok(()),
        Action::Volume(p) | Action::Brightness { percent: p, .. } if p > 100 => {
            Err(RequestError::InvalidPercent)
        }
        _ if !allow_changes => Err(RequestError::ReadOnly),
        _ => Ok(()),
    }
}
fn worker<R: Runner>(
    mut probe: Probe<R>,
    receiver: Receiver<Action>,
    shared: Arc<Mutex<Option<Snapshot>>>,
    stopped: Arc<AtomicBool>,
    allow_changes: bool,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    let mut message = None;
    loop {
        if stopped.load(Ordering::Relaxed) {
            break;
        }
        let mut next = probe.snapshot(allow_changes);
        next.message = message.clone();
        let changed = {
            let mut current = shared.lock().unwrap_or_else(|e| e.into_inner());
            if current.as_ref() != Some(&next) {
                *current = Some(next);
                true
            } else {
                false
            }
        };
        if changed {
            if let Some(notify) = &notify {
                notify();
            }
        }
        let action = match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(action) => action,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if stopped.load(Ordering::Relaxed) {
            break;
        }
        if validate_action(action, allow_changes).is_err() {
            continue;
        }
        message = probe.apply(action);
    }
}

#[derive(Clone)]
struct Paths {
    power: PathBuf,
    backlight: PathBuf,
    bluetooth: PathBuf,
    tcp: [PathBuf; 2],
}
impl Paths {
    fn linux() -> Self {
        Self {
            power: "/sys/class/power_supply".into(),
            backlight: "/sys/class/backlight".into(),
            bluetooth: "/sys/class/bluetooth".into(),
            tcp: ["/proc/net/tcp".into(), "/proc/net/tcp6".into()],
        }
    }
}
fn number(path: &Path) -> Option<u32> {
    let value = fs::read_to_string(path).ok()?;
    value.trim().parse().ok()
}
fn ssh_connection(table: &str) -> bool {
    table.lines().skip(1).any(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        fields.get(3) == Some(&"01")
            && fields
                .get(1)
                .and_then(|v| v.rsplit(':').next())
                .is_some_and(|port| u16::from_str_radix(port, 16).ok() == Some(22))
    })
}
fn remote_session(paths: &Paths) -> bool {
    paths
        .tcp
        .iter()
        .any(|path| fs::read_to_string(path).map_or(true, |v| ssh_connection(&v)))
}
fn percent(value: u32, maximum: u32) -> Option<u8> {
    (maximum > 0 && value <= maximum)
        .then(|| ((u64::from(value) * 100 + u64::from(maximum) / 2) / u64::from(maximum)) as u8)
}
fn entries(path: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
fn battery(paths: &Paths) -> Battery {
    // Ambiguous batteries are not averaged into an invented combined reading.
    let batteries = entries(&paths.power)
        .into_iter()
        .filter(|p| fs::read_to_string(p.join("type")).is_ok_and(|t| t.trim() == "Battery"))
        .collect::<Vec<_>>();
    let [path] = batteries.as_slice() else {
        return Battery::default();
    };
    Battery {
        percent: number(&path.join("capacity"))
            .filter(|v| *v <= 100)
            .map(|v| v as u8),
        state: match fs::read_to_string(path.join("status"))
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("Charging") => BatteryState::Charging,
            Some("Discharging") => BatteryState::Discharging,
            Some("Full") => BatteryState::Full,
            Some("Not charging") => BatteryState::NotCharging,
            _ => BatteryState::Unknown,
        },
    }
}
fn backlight_path(paths: &Paths) -> Option<PathBuf> {
    let valid = entries(&paths.backlight)
        .into_iter()
        .filter(|p| number(&p.join("max_brightness")).is_some_and(|v| v > 0))
        .collect::<Vec<_>>();
    let [path] = valid.as_slice() else {
        return None;
    };
    Some(path.clone())
}
fn can_write(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // access checks the current real user/group permissions without opening
        // or truncating the sysfs control. The trial unit runs as that user.
        unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        false
    }
}
fn brightness(paths: &Paths) -> Brightness {
    let Some(path) = backlight_path(paths) else {
        return Brightness::default();
    };
    let maximum = number(&path.join("max_brightness"));
    Brightness {
        requested_percent: number(&path.join("brightness"))
            .zip(maximum)
            .and_then(|(v, m)| percent(v, m)),
        applied_percent: number(&path.join("actual_brightness"))
            .zip(maximum)
            .and_then(|(v, m)| percent(v, m)),
        maximum,
        writable: can_write(&path.join("brightness")),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandError {
    Unavailable,
    Failed,
    Timeout,
    InvalidOutput,
}
trait Runner: Send + 'static {
    fn run(&mut self, program: &str, args: &[&str]) -> Result<String, CommandError>;
}
struct SystemRunner;

/// Original dArkOS tools sometimes restart ES themselves. Never create a second
/// graphical owner after such a handoff; an unknown service state is not free.
pub fn frontend_is_stopped() -> bool {
    if !cfg!(target_os = "linux") { return false; }
    SystemRunner.run("/usr/bin/systemctl", &["show", "emulationstation.service", "--property=ActiveState", "--value"])
        .is_ok_and(|state| matches!(state.trim(), "inactive" | "failed"))
}
impl Runner for SystemRunner {
    fn run(&mut self, program: &str, args: &[&str]) -> Result<String, CommandError> {
        let mut child = Command::new(program)
            .args(args)
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    CommandError::Unavailable
                } else {
                    CommandError::Failed
                }
            })?;
        // Drain stdout while the process is running. Waiting first can fill
        // the pipe and deadlock even a valid query before its timeout.
        let stdout = child.stdout.take().ok_or(CommandError::InvalidOutput)?;
        let (output_tx, output_rx) = mpsc::sync_channel(1);
        if thread::Builder::new()
            .name("system-output".into())
            .spawn(move || {
                let mut bytes = Vec::new();
                let result = stdout
                    .take(MAX_OUTPUT + 1)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
                    .map_err(|_| CommandError::InvalidOutput);
                let _ = output_tx.send(result);
            })
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::Failed);
        }
        let start = Instant::now();
        let mut output = None;
        let status = loop {
            if output.is_none() {
                if let Ok(result) = output_rx.try_recv() {
                    match result {
                        Ok(bytes) if bytes.len() as u64 <= MAX_OUTPUT => output = Some(bytes),
                        _ => {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(CommandError::InvalidOutput);
                        }
                    }
                }
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < COMMAND_TIMEOUT => {
                    thread::sleep(Duration::from_millis(20))
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CommandError::Timeout);
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CommandError::Failed);
                }
            }
        };
        if !status.success() {
            return Err(CommandError::Failed);
        }
        let bytes = match output {
            Some(bytes) => bytes,
            None => output_rx
                .recv_timeout(COMMAND_TIMEOUT.saturating_sub(start.elapsed()))
                .map_err(|_| CommandError::Timeout)??,
        };
        if bytes.len() as u64 > MAX_OUTPUT {
            return Err(CommandError::InvalidOutput);
        }
        String::from_utf8(bytes).map_err(|_| CommandError::InvalidOutput)
    }
}

/// Parse channel percentages, never mistake ALSA's raw 0..255 value for percent.
fn volume(output: &str) -> Option<(u8, Option<bool>)> {
    let capture_only = output
        .lines()
        .find_map(|line| line.trim().strip_prefix("Capabilities:"))
        .is_some_and(|capabilities| {
            let capabilities = capabilities.split_whitespace().collect::<Vec<_>>();
            capabilities.contains(&"cvolume")
                && !capabilities.contains(&"pvolume")
                && !capabilities.contains(&"volume")
        })
        || (output.contains("Capture channels:") && !output.contains("Playback channels:"));
    if capture_only {
        return None;
    }
    let mut values = Vec::new();
    let mut muted = Vec::new();
    for line in output.lines().filter(|line| {
        // Some Master controls advertise the shared `volume` capability and
        // omit "Playback" in their channel values (the RG353V mixer does).
        let channel = line.trim().split_once(':').map(|(name, _)| name);
        line.contains('[')
            && !line.contains("Capture")
            && (line.contains("Playback")
                || channel.is_some_and(|name| {
                    matches!(
                        name,
                        "Mono"
                            | "Front Left"
                            | "Front Right"
                            | "Front Center"
                            | "Rear Left"
                            | "Rear Right"
                            | "Rear Center"
                            | "Side Left"
                            | "Side Right"
                            | "Woofer"
                    )
                }))
    }) {
        for bracket in line.split('[').skip(1).filter_map(|v| v.split(']').next()) {
            if let Some(p) = bracket
                .strip_suffix('%')
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|p| *p <= 100)
            {
                values.push(u32::from(p));
            } else if bracket == "on" || bracket == "off" {
                muted.push(bracket == "off");
            }
        }
    }
    if values.is_empty() {
        return None;
    }
    let average =
        ((values.iter().sum::<u32>() + values.len() as u32 / 2) / values.len() as u32) as u8;
    let mute = (!muted.is_empty() && muted.iter().all(|m| *m == muted[0])).then(|| muted[0]);
    Some((average, mute))
}
fn signal(output: &str) -> Option<u8> {
    // Only the in-use network, with SSID and BSSID deliberately not requested.
    output.lines().find_map(|line| {
        let (active, level) = line.split_once(':')?;
        (active == "*")
            .then_some(level)?
            .parse::<u8>()
            .ok()
            .filter(|v| *v <= 100)
    })
}
struct Probe<R> {
    paths: Paths,
    runner: R,
}
impl<R: Runner> Probe<R> {
    fn new(paths: Paths, runner: R) -> Self {
        Self { paths, runner }
    }
    fn wifi(&mut self) -> Wifi {
        let unavailable = Wifi {
            state: WifiState::Unavailable,
            signal: None,
        };
        if self
            .runner
            .run(
                "/usr/bin/systemctl",
                &["is-active", "--quiet", "NetworkManager.service"],
            )
            .is_err()
        {
            return unavailable;
        }
        let radio = match self.runner.run("/usr/bin/nmcli", &["radio", "wifi"]) {
            Ok(v) => v,
            Err(CommandError::Unavailable) => return unavailable,
            Err(_) => {
                return Wifi {
                    state: WifiState::Error,
                    signal: None,
                }
            }
        };
        match radio.trim() {
            "disabled" => {
                return Wifi {
                    state: WifiState::Disabled,
                    signal: None,
                }
            }
            "enabled" => (),
            _ => return Wifi::default(),
        }
        // A connected *Ethernet* link must not be displayed as connected Wi-Fi.
        let devices = match self.runner.run(
            "/usr/bin/nmcli",
            &["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"],
        ) {
            Ok(v) => v,
            Err(_) => {
                return Wifi {
                    state: WifiState::Error,
                    signal: None,
                }
            }
        };
        let states = devices
            .lines()
            .filter_map(|line| {
                let mut fields = line.splitn(3, ':');
                fields.next()?;
                (fields.next()? == "wifi").then(|| fields.next()).flatten()
            })
            .collect::<Vec<_>>();
        let state = if states.contains(&"connected") {
            WifiState::Connected
        } else if states.is_empty() {
            WifiState::Unavailable
        } else if states
            .iter()
            .all(|v| matches!(*v, "disconnected" | "unavailable" | "unmanaged"))
        {
            WifiState::Disconnected
        } else {
            WifiState::Unknown
        };
        let signal = if state == WifiState::Connected {
            self.runner
                .run(
                    "/usr/bin/nmcli",
                    &[
                        "-t",
                        "-f",
                        "IN-USE,SIGNAL",
                        "device",
                        "wifi",
                        "list",
                        "--rescan",
                        "no",
                    ],
                )
                .ok()
                .and_then(|v| signal(&v))
        } else {
            None
        };
        Wifi { state, signal }
    }
    fn bluetooth(&mut self) -> Bluetooth {
        let unavailable = Bluetooth {
            state: BluetoothState::Unavailable,
            connected: None,
        };
        // Do not run bluetoothctl on an inactive service: its D-Bus connection
        // could activate BlueZ. rfkill alone does not establish a real adapter.
        if self
            .runner
            .run(
                "/usr/bin/systemctl",
                &["is-active", "--quiet", "bluetooth.service"],
            )
            .is_err()
        {
            return if entries(&self.paths.bluetooth).iter().any(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("hci"))
            }) {
                Bluetooth {
                    state: BluetoothState::ServiceInactive,
                    connected: None,
                }
            } else {
                unavailable
            };
        }
        let shown = match self.runner.run("/usr/bin/bluetoothctl", &["show"]) {
            Ok(v) => v,
            Err(CommandError::Unavailable) => return unavailable,
            Err(_) => {
                return Bluetooth {
                    state: BluetoothState::Error,
                    connected: None,
                }
            }
        };
        if shown.contains("No default controller available")
            || !shown.lines().any(|l| l.starts_with("Controller "))
        {
            return unavailable;
        }
        let state = match shown
            .lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix("Powered: "))
        {
            Some("yes") => BluetoothState::Enabled,
            Some("no") => BluetoothState::Disabled,
            _ => BluetoothState::Unknown,
        };
        let connected = if state == BluetoothState::Enabled {
            self.runner
                .run("/usr/bin/bluetoothctl", &["devices", "Connected"])
                .ok()
                .map(|v| v.lines().any(|l| l.starts_with("Device ")))
        } else {
            None
        };
        Bluetooth { state, connected }
    }
    fn snapshot(&mut self, allow_changes: bool) -> Snapshot {
        let volume = self
            .runner
            .run("/usr/bin/amixer", &["-c", "0", "sget", "Master"])
            .ok()
            .and_then(|v| volume(&v));
        Snapshot {
            battery: battery(&self.paths),
            brightness: brightness(&self.paths),
            wifi: self.wifi(),
            bluetooth: self.bluetooth(),
            volume: volume.map(|v| v.0),
            volume_muted: volume.and_then(|v| v.1),
            allow_changes,
            remote_session: remote_session(&self.paths),
            message: None,
        }
    }
    fn apply(&mut self, action: Action) -> Option<String> {
        match action {
            Action::Refresh => None,
            Action::Volume(value) => {
                let value_arg = format!("{value}%");
                let result = self.runner.run(
                    "/usr/bin/amixer",
                    &["-c", "0", "sset", "Master", &value_arg],
                );
                if result.is_err() {
                    return Some("No pudimos ajustar el volumen del sistema.".into());
                }
                let applied = self
                    .runner
                    .run("/usr/bin/amixer", &["-c", "0", "sget", "Master"])
                    .ok()
                    .and_then(|v| volume(&v));
                match applied {
                    Some((actual, _)) if actual.abs_diff(value) <= 1 => {
                        Some(format!("Volumen del sistema: {actual} %."))
                    }
                    _ => Some("El sistema no confirmó el volumen solicitado.".into()),
                }
            }
            Action::Brightness {
                percent: value,
                safe_test,
            } => {
                let Some(path) = backlight_path(&self.paths) else {
                    return Some("No hay un control de brillo compatible.".into());
                };
                let Some(maximum) = number(&path.join("max_brightness")).filter(|m| *m > 0) else {
                    return Some("No pudimos leer el límite de brillo.".into());
                };
                // Recheck at action time: a new SSH connection may have opened
                // since the last periodic snapshot.
                let safe_test = safe_test || remote_session(&self.paths);
                let value = if safe_test {
                    value.max(TEST_BRIGHTNESS_MIN_PERCENT)
                } else {
                    value
                };
                let raw = ((u64::from(maximum) * u64::from(value) + 50) / 100) as u32;
                let raw = if safe_test { raw.max(1) } else { raw };
                if fs::write(path.join("brightness"), format!("{raw}\n")).is_err() {
                    return Some("El sistema no permite ajustar el brillo.".into());
                }
                // Some backlight drivers apply asynchronously; allow a bounded
                // settling period on this worker before claiming success.
                let deadline = Instant::now() + Duration::from_millis(250);
                loop {
                    let applied = brightness(&self.paths);
                    if applied
                        .requested_percent
                        .is_some_and(|p| p.abs_diff(value) <= 1)
                        && applied
                            .applied_percent
                            .is_some_and(|p| p.abs_diff(value) <= 1)
                    {
                        return Some(format!(
                            "Brillo aplicado: {} %.",
                            applied.applied_percent.unwrap()
                        ));
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    thread::sleep(Duration::from_millis(25));
                }
                Some("Brillo solicitado; el controlador aún no confirmó su aplicación.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "emotional-system-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn paths(&self) -> Paths {
            Paths {
                power: self.0.join("power"),
                backlight: self.0.join("backlight"),
                bluetooth: self.0.join("bluetooth"),
                tcp: [self.0.join("tcp"), self.0.join("tcp6")],
            }
        }
        fn file(&self, name: &str, value: &str) {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, value).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct FakeRunner {
        calls: VecDeque<(String, Vec<String>, Result<String, CommandError>)>,
    }
    impl FakeRunner {
        fn new(calls: Vec<(&str, Vec<&str>, Result<&str, CommandError>)>) -> Self {
            Self {
                calls: calls
                    .into_iter()
                    .map(|(p, a, r)| {
                        (
                            p.into(),
                            a.into_iter().map(Into::into).collect(),
                            r.map(Into::into),
                        )
                    })
                    .collect(),
            }
        }
    }
    impl Runner for FakeRunner {
        fn run(&mut self, p: &str, args: &[&str]) -> Result<String, CommandError> {
            let (expected, argv, result) =
                self.calls.pop_front().expect("unexpected system command");
            assert_eq!(p, expected);
            assert_eq!(args, argv);
            result
        }
    }
    #[test]
    fn snapshot_notification_is_change_driven_not_a_repaint_timer() {
        struct Tracked(FakeRunner, Arc<std::sync::atomic::AtomicUsize>);
        impl Runner for Tracked {
            fn run(&mut self, p: &str, a: &[&str]) -> Result<String, CommandError> {
                let result = self.0.run(p, a);
                self.1.fetch_add(1, Ordering::SeqCst);
                result
            }
        }
        let fixture = Fixture::new();
        let mut calls = Vec::new();
        for _ in 0..2 {
            calls.extend([
                (
                    "/usr/bin/amixer",
                    vec!["-c", "0", "sget", "Master"],
                    Ok("Mono: Playback 128 [50%]"),
                ),
                (
                    "/usr/bin/systemctl",
                    vec!["is-active", "--quiet", "NetworkManager.service"],
                    Err(CommandError::Failed),
                ),
                (
                    "/usr/bin/systemctl",
                    vec!["is-active", "--quiet", "bluetooth.service"],
                    Err(CommandError::Failed),
                ),
            ]);
        }
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let probe = Probe::new(
            fixture.paths(),
            Tracked(FakeRunner::new(calls), count.clone()),
        );
        let (sender, receiver) = mpsc::sync_channel(4);
        let (notifications, observed) = mpsc::sync_channel(4);
        let shared = Arc::new(Mutex::new(None));
        let stopped = Arc::new(AtomicBool::new(false));
        let stop_worker = stopped.clone();
        let thread = std::thread::spawn(move || {
            worker(
                probe,
                receiver,
                shared,
                stop_worker,
                false,
                Some(Arc::new(move || {
                    let _ = notifications.try_send(());
                })),
            )
        });
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        sender.send(Action::Refresh).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while count.load(Ordering::SeqCst) < 6 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(count.load(Ordering::SeqCst), 6);
        assert_eq!(
            observed.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        stopped.store(true, Ordering::Relaxed);
        sender.send(Action::Refresh).unwrap();
        thread.join().unwrap();
    }
    #[test]
    fn command_runner_drains_and_bounds_output_without_waiting_for_pipe() {
        let mut runner = SystemRunner;
        assert_eq!(
            runner.run("/usr/bin/printf", &["ready"]),
            Ok("ready".into())
        );
        assert_eq!(
            runner.run("/usr/bin/head", &["-c", "65536", "/dev/zero"]),
            Err(CommandError::InvalidOutput)
        );
        assert_eq!(runner.run("/bin/sleep", &["3"]), Err(CommandError::Timeout));
    }
    #[test]
    fn battery_uses_capacity_never_voltage_and_reports_unknown() {
        let fixture = Fixture::new();
        fixture.file("power/battery/type", "Battery\n");
        fixture.file("power/battery/capacity", "67\n");
        fixture.file("power/battery/status", "Discharging\n");
        assert_eq!(
            battery(&fixture.paths()),
            Battery {
                percent: Some(67),
                state: BatteryState::Discharging
            }
        );
        fixture.file("power/battery/capacity", "101");
        fixture.file("power/battery/voltage_now", "4100000");
        assert_eq!(battery(&fixture.paths()).percent, None);
        fixture.file("power/battery/status", "Charging");
        assert_eq!(battery(&fixture.paths()).state, BatteryState::Charging);
    }
    #[test]
    fn brightness_keeps_requested_and_applied_distinct() {
        let fixture = Fixture::new();
        fixture.file("backlight/backlight/max_brightness", "255");
        fixture.file("backlight/backlight/brightness", "128");
        fixture.file("backlight/backlight/actual_brightness", "0");
        let reading = brightness(&fixture.paths());
        assert_eq!(reading.requested_percent, Some(50));
        assert_eq!(reading.applied_percent, Some(0));
        assert_eq!(percent(256, 255), None);
    }
    #[test]
    fn alsa_percentages_and_mute_are_parsed_without_raw_scale_assumptions() {
        assert_eq!(volume("Limits: Playback 0 - 255\n Front Left: Playback 242 [95%]\n Front Right: Playback 242 [95%]"), Some((95, None)));
        assert_eq!(
            volume(" Mono: Playback 20 [20%] [-10.00dB] [off]"),
            Some((20, Some(true)))
        );
        assert_eq!(volume("Playback 7 [101%]"), None);
        let real_master = "Simple mixer control 'Master',0\n  Capabilities: volume\n  Playback channels: Front Left - Front Right\n  Capture channels: Front Left - Front Right\n  Limits: 0 - 255\n  Front Left: 241 [95%] [-2.80dB]\n  Front Right: 241 [95%] [-2.80dB]\n";
        assert_eq!(volume(real_master), Some((95, None)));
        assert_eq!(
            volume("Capabilities: cvolume cswitch\nCapture channels: Front Left - Front Right\nFront Left: 241 [95%]\nFront Right: 241 [95%]"),
            None
        );
        assert_eq!(volume("Mono: Capture 20 [20%] [on]"), None);
        assert_eq!(volume("Front Left: 241 [101%]"), None);
    }
    #[test]
    fn read_only_and_percentage_guards_run_before_any_mutation() {
        assert_eq!(
            validate_action(Action::Volume(60), false),
            Err(RequestError::ReadOnly)
        );
        assert_eq!(
            validate_action(
                Action::Brightness {
                    percent: 10,
                    safe_test: true
                },
                false
            ),
            Err(RequestError::ReadOnly)
        );
        assert_eq!(
            validate_action(Action::Volume(101), true),
            Err(RequestError::InvalidPercent)
        );
        assert_eq!(validate_action(Action::Refresh, false), Ok(()));
    }
    #[test]
    fn inactive_bluetooth_never_invokes_or_activates_bluez() {
        let fixture = Fixture::new();
        let mut probe = Probe::new(
            fixture.paths(),
            FakeRunner::new(vec![(
                "/usr/bin/systemctl",
                vec!["is-active", "--quiet", "bluetooth.service"],
                Err(CommandError::Failed),
            )]),
        );
        assert_eq!(probe.bluetooth().state, BluetoothState::Unavailable);
        assert!(probe.runner.calls.is_empty());
    }
    #[test]
    fn inactive_service_with_adapter_is_disabled_service_not_missing_hardware() {
        let fixture = Fixture::new();
        fixture.file("bluetooth/hci0/uevent", "");
        let mut probe = Probe::new(
            fixture.paths(),
            FakeRunner::new(vec![(
                "/usr/bin/systemctl",
                vec!["is-active", "--quiet", "bluetooth.service"],
                Err(CommandError::Failed),
            )]),
        );
        assert_eq!(
            probe.bluetooth(),
            Bluetooth {
                state: BluetoothState::ServiceInactive,
                connected: None
            }
        );
        assert!(probe.runner.calls.is_empty());
    }
    #[test]
    fn wifi_reads_only_in_use_signal_without_ssid_or_rescan() {
        let fixture = Fixture::new();
        let mut probe = Probe::new(
            fixture.paths(),
            FakeRunner::new(vec![
                (
                    "/usr/bin/systemctl",
                    vec!["is-active", "--quiet", "NetworkManager.service"],
                    Ok(""),
                ),
                ("/usr/bin/nmcli", vec!["radio", "wifi"], Ok("enabled\n")),
                (
                    "/usr/bin/nmcli",
                    vec!["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"],
                    Ok("wlan0:wifi:connected\n"),
                ),
                (
                    "/usr/bin/nmcli",
                    vec![
                        "-t",
                        "-f",
                        "IN-USE,SIGNAL",
                        "device",
                        "wifi",
                        "list",
                        "--rescan",
                        "no",
                    ],
                    Ok(":12\n*:72\n"),
                ),
            ]),
        );
        assert_eq!(
            probe.wifi(),
            Wifi {
                state: WifiState::Connected,
                signal: Some(72)
            }
        );
        assert!(probe.runner.calls.is_empty());
    }
    #[test]
    fn ethernet_does_not_make_wifi_connected_and_failed_queries_are_errors() {
        let fixture = Fixture::new();
        let mut probe = Probe::new(
            fixture.paths(),
            FakeRunner::new(vec![
                (
                    "/usr/bin/systemctl",
                    vec!["is-active", "--quiet", "NetworkManager.service"],
                    Ok(""),
                ),
                ("/usr/bin/nmcli", vec!["radio", "wifi"], Ok("enabled")),
                (
                    "/usr/bin/nmcli",
                    vec!["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"],
                    Ok("eth0:ethernet:connected\nwlan0:wifi:disconnected"),
                ),
                (
                    "/usr/bin/systemctl",
                    vec!["is-active", "--quiet", "NetworkManager.service"],
                    Ok(""),
                ),
                (
                    "/usr/bin/nmcli",
                    vec!["radio", "wifi"],
                    Err(CommandError::Timeout),
                ),
            ]),
        );
        assert_eq!(probe.wifi().state, WifiState::Disconnected);
        assert_eq!(probe.wifi().state, WifiState::Error);
    }
    #[test]
    fn volume_success_requires_mixer_readback() {
        let fixture = Fixture::new();
        let mut probe = Probe::new(
            fixture.paths(),
            FakeRunner::new(vec![
                (
                    "/usr/bin/amixer",
                    vec!["-c", "0", "sset", "Master", "60%"],
                    Ok(""),
                ),
                (
                    "/usr/bin/amixer",
                    vec!["-c", "0", "sget", "Master"],
                    Ok("Mono: Playback 153 [60%]"),
                ),
            ]),
        );
        assert_eq!(
            probe.apply(Action::Volume(60)).as_deref(),
            Some("Volumen del sistema: 60 %.")
        );
    }
    #[test]
    fn remote_brightness_floor_is_local_to_action_and_requires_actual_readback() {
        let fixture = Fixture::new();
        fixture.file("backlight/backlight/max_brightness", "255");
        fixture.file("backlight/backlight/brightness", "130");
        fixture.file("backlight/backlight/actual_brightness", "0");
        let mut probe = Probe::new(fixture.paths(), FakeRunner::new(vec![]));
        let response = probe.apply(Action::Brightness {
            percent: 0,
            safe_test: true,
        });
        assert_eq!(
            number(&fixture.paths().backlight.join("backlight/brightness")),
            Some(26)
        );
        assert!(response.unwrap().contains("no confirmó"));
        assert_eq!(brightness(&fixture.paths()).applied_percent, Some(0));
    }
    #[test]
    fn ssh_detection_retains_only_boolean_and_fails_conservatively() {
        let fixture = Fixture::new();
        assert!(remote_session(&fixture.paths()));
        fixture.file(
            "tcp",
            "sl local_address rem_address st\n0: 0100007F:0016 0100007F:D202 0A\n",
        );
        fixture.file("tcp6", "sl local_address rem_address st\n");
        assert!(!remote_session(&fixture.paths()));
        fixture.file(
            "tcp",
            "sl local_address rem_address st\n0: 0100007F:0016 0100007F:D202 01\n",
        );
        assert!(remote_session(&fixture.paths()));
    }
}
