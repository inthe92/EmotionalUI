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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    Network,
    DisplayPower,
    Audio,
    Emulators,
    Storage,
    Maintenance,
    #[default]
    All,
}
impl Category {
    pub const ALL: [Self; 7] = [
        Self::Network,
        Self::DisplayPower,
        Self::Audio,
        Self::Emulators,
        Self::Storage,
        Self::Maintenance,
        Self::All,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Network => "Red y conexiones",
            Self::DisplayPower => "Pantalla y energía",
            Self::Audio => "Sonido del sistema",
            Self::Emulators => "Mandos y emuladores",
            Self::Storage => "Almacenamiento",
            Self::Maintenance => "Mantenimiento",
            Self::All => "Herramientas",
        }
    }
    pub fn contains(self, entry: &Entry) -> bool {
        self == Self::All || entry.category() == self
    }
}
impl Entry {
    pub fn category(&self) -> Category {
        let name = self.relative.to_lowercase();
        if [
            "wifi",
            "bluetooth",
            "network",
            "remote services",
            "password",
        ]
        .iter()
        .any(|s| name.contains(s))
        {
            Category::Network
        } else if [
            "sleep",
            "quick mode",
            "battery",
            "brightness",
            "panel",
            "theme",
        ]
        .iter()
        .any(|s| name.contains(s))
        {
            Category::DisplayPower
        } else if name.contains("audio") || name.contains("sound") {
            Category::Audio
        } else if [
            "retroarch",
            "controls",
            "joystick",
            "controller",
            "hotkeys",
            "drastic",
            "bigpemu",
            "gzdoom",
            "lzdoom",
        ]
        .iter()
        .any(|s| name.contains(s))
        {
            Category::Emulators
        } else if ["files", "file manager", "usb", "sd for roms", "m3u"]
            .iter()
            .any(|s| name.contains(s))
        {
            Category::Storage
        } else {
            Category::Maintenance
        }
    }
    pub fn notice(&self) -> &'static str {
        let name = self.relative.to_lowercase();
        if name.contains("wifi") || name.contains("remote services") || name.contains("password") {
            "La herramienta puede cambiar la conexión o el acceso remoto."
        } else if ["reset", "restore", "delete", "remove", "clear"]
            .iter()
            .any(|s| name.contains(s))
        {
            "La herramienta puede restaurar o borrar datos. Revisa sus indicaciones."
        } else if name.contains("update") {
            "Abre el actualizador original de dArkOS. No interrumpas una actualización."
        } else if name.contains("quick mode")
            || name.contains("sleep")
            || name.contains("sd for roms")
        {
            "La herramienta cambia el comportamiento de la consola y puede pedir reinicio."
        } else {
            "Abre la herramienta original. Al salir puede volver a EmulationStation."
        }
    }
}

fn localized_title(relative: &str) -> Option<&'static str> {
    Some(match relative {
        "Wifi.sh" => "Configurar Wi-Fi",
        "Bluetooth.sh" => "Dispositivos Bluetooth",
        "Network Info.sh" => "Información de red",
        "System Info.sh" => "Información del sistema",
        "Change Password.sh" => "Cambiar contraseña del sistema",
        "Enable Remote Services.sh" => "Activar servicios remotos",
        "Disable Remote Services.sh" => "Desactivar servicios remotos",
        "Advanced/Wifi On.sh" => "Activar Wi-Fi",
        "Advanced/Wifi Off.sh" => "Desactivar Wi-Fi",
        "Advanced/Backup dArkOS Settings.sh" => "Copia de seguridad de dArkOS",
        "Advanced/Restore dArkOS Settings.sh" => "Restaurar copia de dArkOS",
        "Advanced/Controller Tester.sh" => "Comprobar mandos",
        "Advanced/Fix Audio.sh" => "Reparar audio",
        "Advanced/Fix Global Hotkeys.sh" => "Restaurar atajos globales",
        "Advanced/Enable Quick Mode.sh" => "Activar Quick Mode",
        "Advanced/Disable Quick Mode.sh" => "Desactivar Quick Mode",
        "Advanced/Disable Low Battery Warning.sh" => "Desactivar aviso de batería baja",
        "Advanced/Enable Low Battery Warning.sh" => "Activar aviso de batería baja",
        "Advanced/Switch to Main SD for Roms.sh" => "Usar tarjeta principal para los juegos",
        "Advanced/Clear Last Played Collection.sh" => "Vaciar colección de últimos juegos",
        "Advanced/Reset Retroarch Settings.sh" => "Restaurar ajustes de RetroArch",
        "Advanced/Reset Retroarch32 Settings.sh" => "Restaurar ajustes de RetroArch32",
        "Advanced/Reset Retroarch Core Settings.sh" => "Restaurar ajustes de cores RetroArch",
        "Advanced/Reset Retroarch32 Core Settings.sh" => "Restaurar ajustes de cores RetroArch32",
        "Advanced/Sleep - Switch to Light sleep support.sh" => "Cambiar a suspensión ligera",
        "Advanced/Restore Default PPSSPP Controls.sh" => "Restaurar controles de PPSSPP",
        "Advanced/Restore Default Drastic Settings.sh" => "Restaurar ajustes de DraStic",
        "Advanced/Restore Default BigPEmu Settings.sh" => "Restaurar ajustes de BigPEmu",
        "Advanced/Restore Default GZdoom Settings.sh" => "Restaurar ajustes de GZDoom",
        "Advanced/Restore Default LZdoom Settings.sh" => "Restaurar ajustes de LZDoom",
        "Advanced/Restore ECWolf Joystick.sh" => "Restaurar mando de ECWolf",
        "File Manager.sh" => "Gestor de archivos",
        "351Files.sh" => "351Files",
        "USB Drive Mount.sh" => "Montar unidad USB",
        "USB Drive Unmount.sh" => "Desmontar unidad USB",
        "PS1 - Generate m3u files.sh" => "PS1: crear listas M3U",
        "PS1 - Delete m3u files.sh" => "PS1: borrar listas M3U",
        "Remove ._ Files.sh" => "Eliminar archivos auxiliares de macOS",
        "Tools/PortMaster.sh" => "PortMaster",
        "Tools/ThemeMaster.sh" => "ThemeMaster (EmulationStation)",
        "Update.sh" => "Actualizar dArkOS",
        _ => return None,
    })
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
            // A wrapper and a same-named directory form one tool (PortMaster,
            // ThemeMaster, etc.). Its private helper scripts are not menu items.
            if kind.is_dir() && depth < 6 && !item.path().with_extension("sh").is_file() {
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
                let title = labels.get(&path).cloned().unwrap_or_else(|| {
                    localized_title(&relative)
                        .map(String::from)
                        .unwrap_or(fallback)
                });
                result.push(Entry { relative, title });
            }
        }
    }
    result.sort_by_cached_key(|e| {
        (
            match e.relative.as_str() {
                "Wifi.sh" | "System Info.sh" => 0,
                "Bluetooth.sh" => 1,
                "Network Info.sh" => 2,
                _ => 10,
            },
            e.title.to_lowercase(),
        )
    });
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
    // A stock backup/USB/configuration tool needs the same host view it has in
    // ES. Never weaken the ROM read-only namespace of the player/game service.
    // PID 1 starts this explicit tool outside it; PartOf/BindsTo keep recovery
    // attached to the verified functional session, including its descendants.
    let session = fs::read_to_string("/proc/self/cgroup")?;
    let session = functional_session(&session)?;
    #[cfg(target_os = "linux")]
    let uid = unsafe { libc::getuid() };
    #[cfg(not(target_os = "linux"))]
    let uid = 0;
    let args = tool_service_arguments(&script, &session, uid)?;
    let terminal = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty1")?;
    Command::new("/usr/bin/sudo")
        .args(args)
        .env_remove("LD_LIBRARY_PATH")
        .env("TERM", "linux")
        .stdin(Stdio::from(terminal.try_clone()?))
        .stdout(Stdio::from(terminal.try_clone()?))
        .stderr(Stdio::from(terminal))
        .spawn()
}

fn functional_session(cgroup: &str) -> io::Result<String> {
    let units = cgroup
        .lines()
        .filter_map(|line| line.strip_prefix("0::"))
        .filter_map(|path| Path::new(path).file_name()?.to_str())
        .filter(|unit| valid_session(unit))
        .collect::<Vec<_>>();
    let [unit] = units.as_slice() else {
        return Err(io::Error::other(
            "No verified functional session owns this tool",
        ));
    };
    Ok((*unit).into())
}
fn valid_session(unit: &str) -> bool {
    unit.strip_prefix("emotional-ui-functional-")
        .and_then(|s| s.strip_suffix(".service"))
        .is_some_and(|token| token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn tool_service_arguments(script: &Path, session: &str, uid: u32) -> io::Result<Vec<String>> {
    if uid == 0 || !valid_session(session) || !script.is_absolute() {
        return Err(io::Error::other("Invalid tool ownership"));
    }
    let token = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    Ok(vec![
        "-n".into(),
        "/usr/bin/systemd-run".into(),
        "--quiet".into(),
        "--wait".into(),
        "--collect".into(),
        format!("--unit=emotional-ui-option-{}-{token}", std::process::id()),
        "--service-type=exec".into(),
        format!("--uid={uid}"),
        format!("--property=BindsTo={session}"),
        format!("--property=PartOf={session}"),
        "--property=StandardInput=tty".into(),
        "--property=StandardOutput=tty".into(),
        "--property=StandardError=tty".into(),
        "--property=TTYPath=/dev/tty1".into(),
        "--property=TimeoutStopSec=infinity".into(),
        "--property=SendSIGKILL=no".into(),
        "--property=Environment=TERM=linux".into(),
        format!(
            "--working-directory={}",
            script
                .parent()
                .ok_or_else(|| io::Error::other("Invalid tool directory"))?
                .display()
        ),
        "/bin/bash".into(),
        script.to_string_lossy().into_owned(),
    ])
}

pub fn inventory_report(entries: &[Entry]) -> serde_json::Value {
    serde_json::json!({"kind":"darkos-settings-inventory","graphics_started":false,
        "tools":entries.iter().map(|e| serde_json::json!({"relative":e.relative,"title":e.title,"category":e.category().label()})).collect::<Vec<_>>(),
        "categories":Category::ALL.iter().map(|c|serde_json::json!({"title":c.label(),"count":entries.iter().filter(|e|c.contains(e)).count()})).collect::<Vec<_>>() })
}

/// Several stock scripts restart ES (sometimes in the background). Their return
/// must belong to the original console even before ActiveState catches up.
pub fn returns_to_console(root: &Path, relative: &str) -> io::Result<bool> {
    let path = resolve(root, relative)?;
    if path.metadata()?.len() > 512 * 1024 {
        return Ok(true);
    }
    let source = fs::read_to_string(path)?.to_lowercase();
    Ok(source.contains("emulationstation")
        || source.contains("reboot")
        || source.contains("shutdown")
        || source.contains("poweroff"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_tools_have_host_access_but_stay_bound_to_verified_recovery() {
        let unit = "emotional-ui-functional-0123456789abcdef0123456789abcdef.service";
        assert_eq!(
            functional_session(&format!("0::/system.slice/{unit}\n")).unwrap(),
            unit
        );
        for data in [
            "0::/user.slice/unrelated.service",
            "1::/system.slice/other.service",
            "0::/../../evil.service",
        ] {
            assert!(functional_session(data).is_err());
        }
        let args = tool_service_arguments(
            Path::new("/opt/system/Advanced/Backup dArkOS Settings.sh"),
            unit,
            1002,
        )
        .unwrap();
        assert!(args.contains(&format!("--property=PartOf={unit}")));
        assert!(args.contains(&format!("--property=BindsTo={unit}")));
        assert!(args.contains(&"--property=TTYPath=/dev/tty1".into()));
        assert!(args.contains(&"--uid=1002".into()));
        assert_eq!(
            args.last().unwrap(),
            "/opt/system/Advanced/Backup dArkOS Settings.sh"
        );
        assert!(!args
            .iter()
            .any(|a| a.contains("ReadOnlyPaths") || a == "-c"));
        assert!(tool_service_arguments(Path::new("/tool"), unit, 0).is_err());
        assert!(tool_service_arguments(Path::new("/tool"), "unrelated.service", 1002).is_err());
    }
    #[test]
    fn wrappers_hide_payload_helpers_without_hiding_other_menu_folders() {
        let root = std::env::temp_dir().join(format!("eui-options-payload-{}", std::process::id()));
        fs::create_dir_all(root.join("Tools/PortMaster/internal")).unwrap();
        fs::create_dir_all(root.join("Advanced")).unwrap();
        for name in [
            "Tools/PortMaster.sh",
            "Tools/PortMaster/internal/update.sh",
            "Advanced/Wifi On.sh",
        ] {
            fs::write(root.join(name), "exit 0").unwrap();
        }
        let entries = discover(&root).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(!entries.iter().any(|e| e.relative.contains("internal")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn asynchronous_frontend_restarts_require_console_return_before_state_catches_up() {
        let root = std::env::temp_dir().join(format!("eui-options-return-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Restart.sh"),
            "sudo systemctl restart emulationstation &\n",
        )
        .unwrap();
        fs::write(root.join("Wifi.sh"), "dialog --menu Wi-Fi\n").unwrap();
        assert!(returns_to_console(&root, "Restart.sh").unwrap());
        assert!(!returns_to_console(&root, "Wifi.sh").unwrap());
        assert!(returns_to_console(&root, "../not-allowed.sh").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn every_tool_has_one_category_and_all_preserves_unknown_additions() {
        for (relative, expected) in [
            ("Wifi.sh", Category::Network),
            ("Advanced/Sleep - Switch.sh", Category::DisplayPower),
            ("Advanced/Fix Audio.sh", Category::Audio),
            ("Advanced/Controller Tester.sh", Category::Emulators),
            ("USB Drive Mount.sh", Category::Storage),
            ("New tool.sh", Category::Maintenance),
        ] {
            let entry = Entry {
                relative: relative.into(),
                title: relative.into(),
            };
            assert_eq!(entry.category(), expected);
            assert!(Category::All.contains(&entry));
            assert_eq!(
                Category::ALL
                    .into_iter()
                    .filter(|c| *c != Category::All && c.contains(&entry))
                    .count(),
                1
            );
        }
    }
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
