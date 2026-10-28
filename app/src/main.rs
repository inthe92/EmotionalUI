mod ambient_motion;
mod artwork;
mod atlas;
mod atlas_classic;
mod audio_settings;
mod capture;
mod controls;
mod discovery_presentation;
mod game_options;
mod focus;
#[cfg(feature = "gpu-material")]
mod gpu_material;
mod launch_ui;
mod library_browser;
mod library_ui;
#[cfg(any(debug_assertions, feature = "simulator"))]
mod moments;
mod motion;
mod navigation;
mod performance;
mod rom_drop;
#[cfg(test)]
mod pipeline_audit_tests;
mod scene_cache;
mod scene_raster;
#[cfg(feature = "simulator")]
mod simulator;
mod smoke;
mod state;
mod store;
mod supervisor;
mod system_settings;
mod settings_menu;
mod settings_browser;
mod trial;
mod trial_replay;
use chrono::{Datelike, NaiveDateTime};
use emotional_core::{
    input::RepeatGate,
    library::demo_catalog,
    mood::Mood,
    recommendation::Context,
};
use emotional_services::{clock::Clock, weather::Weather};
use serde::Deserialize;
use slint::ComponentHandle;
use state::State;
use std::{cell::RefCell, error::Error, path::PathBuf, rc::Rc, time::Instant};
slint::include_modules!();
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    ambient: emotional_core::ambient::Settings,
    reduced_motion: bool,
    continuity: emotional_core::continuity::Weights,
    region: String,
    session_minutes: u16,
    weather: String,
    at: Option<String>,
    holidays: Vec<emotional_core::calendar::Holiday>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            ambient: Default::default(),
            reduced_motion: false,
            continuity: Default::default(),
            region: "ES".into(),
            session_minutes: 30,
            weather: "unknown".into(),
            at: None,
            holidays: vec![],
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("EmotionalUI v{}", include_str!("../VERSION").trim());
        return Ok(());
    }
    let inspect_args: Vec<String> = std::env::args().collect();
    if inspect_args.iter().any(|a| a == "--system-settings-check") {
        if inspect_args.len() != 3 || inspect_args[1] != "--system-settings-check" || !PathBuf::from(&inspect_args[2]).is_absolute() {
            return Err("Use --system-settings-check with one absolute installed tools root".into());
        }
        use emotional_services::system_options;
        let entries = system_options::discover(std::path::Path::new(&inspect_args[2]))?;
        println!("{}", system_options::inventory_report(&entries));
        return Ok(());
    }
    let result = supervisor::dispatch().and_then(|handled| if handled { Ok(()) } else { run() });
    smoke::emit(
        "process_exit",
        None,
        None,
        Some(if result.is_ok() {
            "success"
        } else {
            "failure; see stderr"
        }),
    );
    if smoke::failed() && result.is_ok() {
        return Err("Smoke journal could not be written".into());
    }
    result
}
fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut config_path = None;
    let mut weather_config_path = None;
    let mut demo_count = None;
    let mut functional_mode = false;
    let mut launcher_profile = None;
    let mut unknown_time = false;
    let mut import_xml = None;
    let mut rom_root = None;
    let mut library_root = None;
    let mut at = None;
    let mut weather = None;
    let mut reduced = false;
    let mut smoke = false;
    let mut smoke_log = None;
    let mut quit_file = None;
    let mut smoke_seconds = 2u64;
    let mut ui_test = false;
    let mut audio_mode = None;
    let mut audio_volume = None;
    let mut audio_assets = None;
    let mut concept = 4;
    let mut requested_view = None;
    let mut demo_history = false;
    let mut history_command = false;
    let mut clear_history = false;
    let mut completed_id = None;
    let mut visual_case = None;
    let mut editorial_path = None;
    let mut atlas_path = None;
    let mut atlas_link = None;
    let mut atlas_experimental = false;
    let mut trial_mode = false;
    let mut calendar_path = None;
    let mut input_profile = None;
    let mut keyboard_map = None;
    let mut dev_moment = None;
    let mut dev_weather_age = None;
    let mut dev_season = None;
    let mut dev_environment_file = None;
    let mut simulation = None;
    let mut snapshot = None;
    let mut snapshot_view = "home".to_owned();
    let mut db_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dev-environment-file" => {
                dev_environment_file = Some(PathBuf::from(
                    args.next().ok_or("DEV environment needs a file")?,
                ))
            }
            "--launcher-config" => {
                launcher_profile = Some(PathBuf::from(
                    args.next().ok_or("Missing launcher argument")?,
                ));
            }
            "--launch-child" | "--sim-return-replay" => {
                args.next().ok_or("Missing launcher argument")?;
            }
            "--launch-physical-supervised" => {}
            "--trial" => trial_mode = true,
            "--functional" => functional_mode = true,
            "--atlas-experimental" => atlas_experimental = true,
            "--atlas" => {
                atlas_path = Some(PathBuf::from(
                    args.next().ok_or("--atlas needs a database")?,
                ))
            }
            "--atlas-link" => {
                atlas_link = Some(args.next().ok_or("--atlas-link needs LOCAL_ID=ATLAS_ID")?)
            }
            "--audio-mode" => {
                audio_mode = Some(match args.next().as_deref() {
                    Some("minimal") => emotional_core::audio::Mode::Minimal,
                    Some("ambient") => emotional_core::audio::Mode::Ambient,
                    Some("silence") => emotional_core::audio::Mode::Silence,
                    _ => return Err("Audio mode: minimal|ambient|silence".into()),
                })
            }
            "--audio-volume" => {
                audio_volume = Some(
                    args.next()
                        .ok_or("--audio-volume needs 0..100")?
                        .parse::<u8>()?,
                )
            }
            "--audio-assets" => {
                audio_assets = Some(PathBuf::from(
                    args.next().ok_or("--audio-assets needs directory")?,
                ))
            }
            "--view" => {
                let v = args.next().ok_or("--view needs resume|emotional")?;
                if !["resume", "emotional"].contains(&v.as_str()) {
                    return Err("Invalid view".into());
                }
                requested_view = Some(v);
            }
            "--demo-history" => demo_history = true,
            "--history" => history_command = true,
            "--clear-history" => clear_history = true,
            "--mark-completed" => {
                completed_id = Some(args.next().ok_or("--mark-completed needs ID")?)
            }
            "--concept" => {
                concept = match args.next().as_deref() {
                    Some("integrated") => 4,
                    Some("legacy") => 0,
                    Some("quiet") => 1,
                    Some("scene") => 2,
                    Some("editorial") => 3,
                    _ => return Err("Concept: legacy|quiet|scene|editorial".into()),
                }
            }
            "--visual-case" => {
                visual_case = Some(args.next().ok_or("--visual-case needs a value")?)
            }
            "--import-editorial" => {
                editorial_path = Some(PathBuf::from(
                    args.next().ok_or("--import-editorial needs a path")?,
                ))
            }
            "--editorial-calendar" => {
                calendar_path = Some(PathBuf::from(
                    args.next().ok_or("--editorial-calendar needs a path")?,
                ))
            }
            "--dev-moment" => dev_moment = Some(args.next().ok_or("--dev-moment necesita nombre")?),
            "--dev-season" => {
                dev_season = Some(
                    args.next()
                        .ok_or("--dev-season needs spring|summer|autumn|winter")?,
                )
            }
            "--dev-weather-age" => {
                dev_weather_age = Some(
                    args.next()
                        .ok_or("--dev-weather-age necesita minutos")?
                        .parse::<i64>()?,
                )
            }
            "--library-root" => {
                library_root = Some(PathBuf::from(
                    args.next().ok_or("--library-root necesita ruta")?,
                ))
            }
            "--keyboard-map" => {
                keyboard_map = Some(PathBuf::from(
                    args.next().ok_or("--keyboard-map necesita ruta")?,
                ))
            }
            "--sim-replay" => {
                simulation = Some(PathBuf::from(
                    args.next().ok_or("--sim-replay necesita ruta")?,
                ))
            }
            "--help" => {
                println!(
                    r#"EmotionalUI 0.2.0
  --launcher-config PROFILE.toml --db ABSOLUTE.sqlite --library-root ROM_ROOT
  --functional                 controles reales explícitos; incompatible con --trial
  --audio-mode minimal|ambient|silence --audio-volume 0..100
  --audio-assets DIR (recursos opcionales locales)
  --view resume|emotional --demo-history (solo desarrollo; no persiste)
  --atlas emotional-atlas.db --atlas-link LOCAL_ID=ATLAS_ID
  --trial  Real-library temporary preview; no demo games or game launching
  --atlas-experimental  Opt in to proposed source-assisted profiles
  --history | --clear-history | --mark-completed ID
  --concept integrated|legacy|quiet|scene|editorial
  --at YYYY-MM-DDTHH:MM  hora local simulada
  --weather clear|rain|cloud|wind|unknown
  --unknown-time
  --dev-moment NAME      preset, solo desarrollo/simulador
  --dev-weather-age MIN  antigüedad simulada, solo desarrollo
  --dev-season spring|summer|autumn|winter (requiere --at; solo desarrollo)
  --dev-environment-file PATH  escenario offline ficticio (solo desarrollo)
  --keyboard-map PATH    sobrescrituras TOML
  --input-profile PATH   evdev Linux, perfil físico verificado
  --library-root DIR     GUI desde el catálogo SQLite importado
  --import-gamelist XML --rom-root DIR --db DB
  --db PATH --config PATH --reduced-motion
  --weather-config PATH.weather-private.toml  ubicación y consentimiento privados (0600)
  --ui-test
  --smoke-test --smoke-seconds 1..30
  --smoke-log PATH --quit-file PATH
  --snapshot PATH.png --snapshot-view home|library|menu|focus|confirmation
  --sim-replay PATH      solo compilación simulator
Teclado: flechas, Z/Enter confirmar, X/Esc volver, S menú, A biblioteca,
Q/E secciones, N otra recomendación, M movimiento reducido.
Desarrollo: F2/F3 escenarios, [/] paletas."#
                );
                return Ok(());
            }
            "--input-profile" => {
                input_profile = Some(PathBuf::from(
                    args.next().ok_or("--input-profile necesita ruta")?,
                ))
            }
            "--demo-count" => {
                demo_count = Some(
                    args.next()
                        .ok_or("--demo-count necesita valor")?
                        .parse::<usize>()?,
                )
            }
            "--unknown-time" => unknown_time = true,
            "--import-gamelist" => {
                import_xml = Some(PathBuf::from(
                    args.next().ok_or("--import-gamelist necesita ruta")?,
                ))
            }
            "--rom-root" => {
                rom_root = Some(PathBuf::from(
                    args.next().ok_or("--rom-root necesita ruta")?,
                ))
            }
            "--at" => at = Some(args.next().ok_or("--at necesita valor")?),
            "--weather" => weather = Some(args.next().ok_or("--weather necesita valor")?),
            "--config" => config_path = Some(args.next().ok_or("--config necesita ruta")?),
            "--weather-config" => {
                weather_config_path =
                    Some(args.next().ok_or("--weather-config needs a private file")?)
            }
            "--db" => db_path = Some(args.next().ok_or("--db necesita ruta")?),
            "--reduced-motion" => reduced = true,
            "--smoke-log" => {
                smoke_log = Some(PathBuf::from(
                    args.next().ok_or("--smoke-log necesita ruta")?,
                ))
            }
            "--quit-file" => {
                quit_file = Some(PathBuf::from(
                    args.next().ok_or("--quit-file necesita ruta")?,
                ))
            }
            "--smoke-seconds" => {
                smoke_seconds = args
                    .next()
                    .ok_or("--smoke-seconds necesita valor")?
                    .parse()?;
                if !(1..=30).contains(&smoke_seconds) {
                    return Err("Smoke duration must be 1..30 seconds".into());
                }
            }
            "--smoke-test" => smoke = true,
            "--ui-test" => {
                ui_test = true;
                smoke = true;
            }
            "--snapshot-view" => {
                snapshot_view = args.next().ok_or("--snapshot-view necesita valor")?;
                if !["home", "library", "menu", "focus", "confirmation"]
                    .contains(&snapshot_view.as_str())
                {
                    return Err("Vista de captura desconocida".into());
                }
            }
            "--snapshot" => snapshot = Some(args.next().ok_or("--snapshot necesita ruta")?),
            _ => return Err(format!("Opción desconocida: {arg}").into()),
        }
    }
    if trial_mode && functional_mode {
        return Err("Trial mode cannot enable functional system controls".into());
    }
    let gpu_replay = trial_replay::configured(trial_mode)?;
    #[cfg(feature = "linuxkms-gpu")]
    if !matches!(
        std::env::var("SLINT_BACKEND").as_deref(),
        Ok("linuxkms-femtovg" | "linuxkms-software")
    ) {
        return Err("GPU experimental builds require explicit SLINT_BACKEND=linuxkms-femtovg or linuxkms-software; use the independent software executable for fallback".into());
    }
    if trial_mode
        && (demo_count.is_some()
            || demo_history
            || visual_case.is_some()
            || import_xml.is_some()
            || editorial_path.is_some())
    {
        return Err("Trial mode does not accept demo data or import commands".into());
    }
    #[cfg(not(any(debug_assertions, feature = "simulator")))]
    if (1..=3).contains(&concept) || visual_case.is_some() || demo_history {
        return Err("Visual prototypes require a development build".into());
    }
    match visual_case.as_deref() {
        None | Some("long-title") | Some("no-art") | Some("resume-three") => {}
        Some("empty") => demo_count = Some(0),
        Some("unknown") => unknown_time = true,
        Some("reduced") => reduced = true,
        _ => return Err("Unknown visual case".into()),
    }
    #[cfg(not(any(debug_assertions, feature = "simulator")))]
    if dev_moment.is_some()
        || dev_weather_age.is_some()
        || dev_season.is_some()
        || dev_environment_file.is_some()
    {
        return Err("Context presets require a development or simulator build".into());
    }
    #[cfg(not(feature = "simulator"))]
    if simulation.is_some() {
        return Err("--sim-replay requires the simulator build feature".into());
    }
    #[cfg(feature = "simulator")]
    if input_profile.is_some() {
        return Err("Simulator cannot open physical controllers; use --sim-replay".into());
    }
    if let Some(path) = smoke_log {
        smoke::enable(&path)?;
    }
    smoke::emit(
        "backend_requested",
        None,
        None,
        std::env::var("SLINT_BACKEND").ok().as_deref(),
    );
    let profile: Option<controls::Profile> = input_profile
        .map(|path| -> Result<_, Box<dyn Error>> {
            let p: controls::Profile = toml::from_str(&std::fs::read_to_string(path)?)?;
            p.validate()?;
            Ok(p)
        })
        .transpose()?;
    #[cfg(not(target_os = "linux"))]
    if profile.is_some() {
        return Err("evdev controls require Linux; use keyboard on desktop".into());
    }
    let mut config: Config = if let Some(path) = config_path {
        toml::from_str(&std::fs::read_to_string(path)?)?
    } else {
        toml::from_str(include_str!("../../assets/themes/default.toml"))?
    };
    #[cfg(any(debug_assertions, feature = "simulator"))]
    let dev_environment = dev_environment_file
        .as_ref()
        .map(|path| moments::Environment::read(path))
        .transpose()?;
    #[cfg(any(debug_assertions, feature = "simulator"))]
    if let Some(case) = &dev_environment {
        if weather_config_path.is_some()
            || at.is_some()
            || weather.is_some()
            || dev_moment.is_some()
            || dev_season.is_some()
            || unknown_time
            || trial_mode
        {
            return Err("DEV environment cannot combine with network configuration or other context overrides".into());
        }
        config.ambient = case.ambient.clone();
        config.at = None;
        config.weather = "unknown".into();
    }
    if let Some(at) = at {
        config.at = Some(at);
    }
    if let Some(w) = weather {
        config.weather = w;
    }
    if reduced {
        config.reduced_motion = true;
    }
    if config.session_minutes == 0 || config.session_minutes > 240 {
        return Err("session_minutes debe estar entre 1 y 240".into());
    }
    for holiday in &config.holidays {
        if chrono::NaiveDate::from_ymd_opt(2000, holiday.month, holiday.day).is_none()
            || holiday.region.trim().is_empty()
            || holiday.label.trim().is_empty()
        {
            return Err("Festividad inválida en configuración".into());
        }
    }
    let simulated = config
        .at
        .as_deref()
        .map(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .transpose()?;
    let day_demo = std::env::var("EMOTIONAL_DAY_DEMO").ok();
    if day_demo.is_some() && (day_demo.as_deref() != Some("four-hours-v1") || !trial_mode || simulated.is_none() || unknown_time) {
        return Err("Day demo requires --trial, --at and EMOTIONAL_DAY_DEMO=four-hours-v1".into());
    }
    let mut clock = Clock::new(simulated);
    if unknown_time {
        clock.unknown();
    }
    let weather = Weather::Unknown;
    let mut ambient = emotional_services::ambient::Runtime::new(config.ambient.clone())?;
    #[cfg(any(debug_assertions, feature = "simulator"))]
    if let Some(case) = &dev_environment {
        clock = case.install(&mut ambient)?;
    }
    if let Some(name) = dev_season {
        if simulated.is_none() {
            return Err("--dev-season requires a simulated --at date".into());
        }
        use emotional_core::calendar::Season;
        ambient.demo_season = Some(match name.as_str() {
            "spring" => Season::Spring,
            "summer" => Season::Summer,
            "autumn" => Season::Autumn,
            "winter" => Season::Winter,
            _ => return Err("Invalid preview season".into()),
        });
    }
    #[cfg(any(debug_assertions, feature = "simulator"))]
    if let Some(name) = &dev_moment {
        let (c, _) = moments::context(name)?;
        clock = c;
    }
    if unknown_time {
        clock.unknown();
    }
    if dev_weather_age.is_some_and(|age| !(-10080..=10080).contains(&age)) {
        return Err("Weather age must be -10080..10080 minutes".into());
    }
    let path = if let Some(p) = db_path {
        PathBuf::from(p)
    } else {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                    .join(".local/share")
            });
        base.join("emotional-ui/demo.sqlite")
    };
    if path.to_str() != Some(":memory:") {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }
    if weather_config_path.is_some() {
        eprintln!("Weather is disabled: the interface uses light/dark appearance only");
    }
    let mut store = store::Store::open(&path)?;
    match store.last_valid_time() {
        Ok(saved) => clock.restore_last_known(saved),
        Err(e) => eprintln!("Clock fallback ignored: {e}"),
    }
    if let Some(path) = editorial_path {
        let catalog: emotional_core::editorial::Catalog =
            toml::from_str(&std::fs::read_to_string(path)?)?;
        store.import_editorial(&catalog)?;
        println!("Editorial metadata imported into own SQLite");
        return Ok(());
    }
    let calendar: emotional_core::editorial::Calendar = if let Some(path) = calendar_path {
        toml::from_str(&std::fs::read_to_string(path)?)?
    } else {
        toml::from_str(include_str!("../../assets/editorial/calendar.toml"))?
    };
    calendar.validate()?;
    if let Some(xml) = import_xml {
        let root = rom_root.ok_or("--import-gamelist requiere --rom-root explícita")?;
        let report = emotional_services::emulationstation::import(&xml, &root)?;
        store.import(&report.games)?;
        println!("Importados: {} · Omitidos (ausentes, duplicados o fuera de raíz): {}. Originales intactos.",report.games.len(),report.skipped);
        return Ok(());
    }
    if rom_root.is_some() {
        return Err("--rom-root requiere --import-gamelist".into());
    }
    if trial_mode && path.to_str() == Some(":memory:") {
        return Err("Trial mode requires its own persistent database".into());
    }
    if trial_mode {
        library_root = trial::root(library_root);
    }
    let trial_root = library_root.clone();
    let imported = library_root.is_some() || trial_mode;
    let library_unavailable = library_root.as_ref().is_some_and(|p| !p.is_dir());
    let mut games = if library_unavailable {
        Vec::new()
    } else if let Some(root) = library_root {
        store.catalog(&root)?
    } else if trial_mode {
        Vec::new()
    } else {
        demo_catalog()
    };
    if let Some(count) = demo_count {
        games.truncate(count);
    }
    if !imported {
        let catalog: emotional_core::editorial::Catalog =
            toml::from_str(include_str!("../../assets/editorial/catalog.toml"))?;
        catalog.apply(&mut games)?;
    }
    if visual_case.as_deref() == Some("long-title") {
        for g in &mut games {
            g.title = "La última luz del faro, al otro lado del invierno".into();
        }
    }
    store.load(&mut games)?;
    if atlas_experimental && atlas_path.is_none() {
        return Err("--atlas-experimental requires --atlas".into());
    }
    let atlas = atlas_path
        .as_deref()
        .map(|p| {
            if atlas_experimental {
                atlas::Atlas::open_channel(p, true)
            } else {
                atlas::Atlas::open(p)
            }
        })
        .transpose()?;
    let atlas_explanations = atlas
        .as_ref()
        .map(|a| a.explanations(&store))
        .transpose()?
        .unwrap_or_default();
    if let Some(link) = atlas_link {
        let (local, target) = link.rsplit_once('=').ok_or("Expected LOCAL_ID=ATLAS_ID")?;
        if !atlas
            .as_ref()
            .ok_or("--atlas-link requires --atlas")?
            .contains(target)?
        {
            return Err("Unknown Atlas ID".into());
        }
        store.atlas_link(local, target)?;
        println!("Explicit local identity linked; original XML unchanged");
        return Ok(());
    }
    if let Some(atlas) = &atlas {
        println!(
            "ATLAS version={} linked={}",
            atlas.version,
            atlas.apply(&store, &mut games)?
        );
    }
    let classic_matches = atlas_classic::apply(&mut games)?;
    println!("ATLAS_CLASSIC version=1.0.0 classified={classic_matches} installed={} offline=true",games.len());
    let atlas_observations = store.atlas_observations()?;
    let privacy = store.privacy()?;
    let mut audio_preferences = store.audio()?;
    if let Some(mode) = audio_mode {
        audio_preferences.mode = mode;
    }
    if let Some(volume) = audio_volume {
        audio_preferences.master = volume;
    }
    audio_preferences.validate()?;
    if clear_history {
        store.clear_usage()?;
        println!("Historial y preferencias eliminados; catálogo y progreso explícito conservados.");
        return Ok(());
    }
    if let Some(id) = completed_id {
        if !games.iter().any(|g| g.available && g.id == id) {
            return Err("Game is not available".into());
        }
        store.complete(&id)?;
        return Ok(());
    }
    let mut history = store.history()?;
    if history_command {
        for session in &history.sessions {
            println!(
                "{} | UTC {} | {:?}",
                session.game_id, session.started_utc, session.duration_seconds
            );
        }
        return Ok(());
    }
    if demo_history {
        if let Some(g) = games.first() {
            history.sessions.push(emotional_core::product::Session {
                game_id: g.id.clone(),
                started_utc: clock
                    .now()
                    .and_local_timezone(chrono::Local)
                    .single()
                    .map(|v| v.timestamp())
                    .unwrap_or(0)
                    .saturating_sub(3600),
                duration_seconds: None,
            });
        }
    }
    config.continuity.validate()?;
    let continuity_preferences = store.continuity_preferences()?;
    let mut continuity_activity = store.continuity_activity()?;
    if demo_history || visual_case.as_deref() == Some("resume-three") {
        for (i, g) in games
            .iter()
            .take(if visual_case.as_deref() == Some("resume-three") {
                3
            } else {
                1
            })
            .enumerate()
        {
            continuity_activity.push(emotional_core::continuity::Evidence {
                game_id: g.id.clone(),
                ordinal: 10_000_000 - i as u64,
                utc: None,
                seconds: Some(3600 / (i as u32 + 1)),
                confirmed: true,
                unexpected: false,
                voluntary_return: None,
                saved: None,
            });
        }
    }
    let continuity_order = emotional_core::continuity::rank(
        &games,
        &continuity_activity,
        &continuity_preferences,
        clock.known_now().map(|d| d.and_utc().timestamp()),
        &config.continuity,
    )
    .into_iter()
    .map(|c| c.index)
    .collect::<Vec<_>>();
    let last_launched = store.last_launched()?;
    let _ = last_launched; // retained query contract; continuity includes undated launch records.
    let recent = continuity_order.first().copied();
    let mut confirmed = std::collections::HashMap::<&str, (u32, [u16; 4])>::new();
    for e in continuity_activity
        .iter()
        .filter(|e| e.confirmed && !e.unexpected)
    {
        let v = confirmed.entry(e.game_id.as_str()).or_default();
        v.0 = v.0.saturating_add(1);
        if let Some(at) = e
            .utc
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .filter(|t| (2020..=2100).contains(&t.year()))
        {
            let slot = Mood::at(at.with_timezone(&chrono::Local).naive_local()).index() as usize;
            v.1[slot] = v.1[slot].saturating_add(1);
        }
    }
    for game in &mut games {
        if let Some((count, habits)) = confirmed.get(game.id.as_str()) {
            game.started = true;
            if privacy.personalized {
                game.choices = game.choices.max(*count);
                for (old, value) in game.choices_by_mood.iter_mut().zip(habits) {
                    *old = (*old).max(*value);
                }
            }
        }
    }
    let launch_directory = supervisor::child_directory();
    let returned = launch_directory.as_deref().and_then(supervisor::returned);
    let resume = requested_view
        .as_deref()
        .map(|v| v == "resume")
        .unwrap_or(recent.is_some());
    let known = clock.known_now();
    let slot = known
        .map(|d| format!("{}-{}", d.date(), Mood::at(d).index()))
        .unwrap_or_else(|| "unknown".into());
    let context = Context {
        mood: known.map(Mood::at).unwrap_or(Mood::Shelter),
        minutes: config.session_minutes,
        raining: atlas_classic::raining(&clock,&ambient,weather),
        slot: known
            .map(|d| (d.and_utc().timestamp().max(0) as u64) / 21600)
            .unwrap_or(0),
        refresh: 0,
    };
    let atlas_habits = emotional_core::atlas::Habits::learn(&games,&atlas_observations,atlas_classic::now(&clock));
    let signals = emotional_core::editorial::Signals {
        date: known.map(|d| d.date()),
        events: calendar.active(known.map(|d| d.date()), &config.region),
        habits: Some(&atlas_habits),
        ..Default::default()
    };
    let affinities = continuity_preferences.iter().map(|(id,p)|(id.clone(),p.affinity)).collect();
    // Rank once on context/library changes; keep the full installed set eligible.
    let order = atlas_classic::eligible_order(
        emotional_core::recommendation::rank_affinity(&games,context,&signals,privacy.personalized,&affinities),
        &games, imported);
    let discovery_reserve = None;
    let selected =
        if concept > 0 && (order.is_empty() || (concept == 4 && resume && recent.is_none())) {
            2
        } else {
            0
        };
    let mut state = State {
        navigation: Default::default(),
        modal_return: None,
        atlas_explanations,
        atlas_observations,
        atlas_habits,
        system_focus: false,
        system_return_dock: None,
        system_open: false,
        system_index: 0,
        system_service: None,
        system_snapshot: None,
        system_note: None,
        drop_session: None,
        drop_open: false,
        drop_ready: false,
        drop_refresh: false,
        drop_message: String::new(),
        drop_image: slint::Image::default(),
        drop_profile: launcher_profile.clone().filter(|_| functional_mode),
        dock_focus: (selected == 2 && concept == 4).then_some(2),
        dock_return: 0,
        trial_mode,
        local_art: if trial_mode {
            Default::default()
        } else if let Some(root) = trial_root.as_deref().filter(|r| r.is_dir()) {
            store.art_paths(root)?
        } else {
            Default::default()
        },
        discovery_reserve,
        launch: launch_directory.map(|directory| launch_ui::Client {
            directory,
            pending: None,
            committed: None,
        }),
        layout: concept,
        statistics: State::statistics_line(&history, &games, demo_history),
        resume,
        recent,
        continuity_order,
        continuity_activity,
        continuity_preferences,
        game_options: None,
        game_option_index: 0,
        game_explained: false,
        history,
        privacy,
        demo_history: demo_history || visual_case.as_deref() == Some("resume-three"),
        history_panel: false,
        clear_armed: false,
        resume_focus: if recent.is_some() { 0 } else { 2 },
        emotional_focus: if order.is_empty() { 2 } else { 0 },
        #[cfg(any(debug_assertions, feature = "simulator"))]
        moment_index: dev_moment
            .as_ref()
            .and_then(|n| moments::NAMES.iter().position(|v| v == n)),
        imported,
        library_browser: library_browser::Browser::new(&games),
        games,
        order,
        selected,
        library_index: 0,
        library: false,
        menu: false,
        menu_index: 0,
        options_open: false,
        options_browser: settings_browser::Browser::default(),
        system_options: launcher_profile.as_deref().and_then(|profile| {
            let config: emotional_services::launcher::configured::Configuration = toml::from_str(&std::fs::read_to_string(profile).ok()?).ok()?;
            config.system_options_root.and_then(|root| emotional_services::system_options::discover(&root).ok())
        }).unwrap_or_default(),
        menu_from_library: false,
        native_held: false,
        audio: emotional_services::ambient_audio::AmbientAudioEngine::start(
            audio_preferences.clone(),
            emotional_services::ambient_audio::Options {
                backend: if cfg!(feature = "simulator") {
                    emotional_services::ambient_audio::output::Backend::Unavailable
                } else {
                    emotional_services::ambient_audio::output::Backend::Native
                },
                assets: audio_assets,
                lead_in_frames: None,
            },
        )?,
        audio_preferences,
        audio_panel: false,
        audio_index: 0,
        ambient,
        clock,
        weather,
        config,
        refresh: 0,
        writer: store::Persistence::start(store),
        slot,
        gate: RepeatGate::default(),
        keyboard: if let Some(path) = keyboard_map {
            let map: controls::Keyboard = toml::from_str(&std::fs::read_to_string(path)?)?;
            map.validate()?;
            map
        } else {
            controls::Keyboard::default()
        },
        start: Instant::now(),
    };
    if let Some(back) = &returned {
        // A newly constructed empty/ranking model may have focused the dock.
        // Returning from a game/tool must restore the content that launched it.
        state.dock_focus = None;
        state.system_focus = false;
        let position = launch_ui::restore_position(&back.position, &state.games, &state.continuity_order);
        if let Some(tool) = &position.system_option {
            state.menu = true;
            state.options_open = true;
            state.menu_index = state.options_browser.show_tool(tool, &state.system_options, false);
        }
        state.resume = position.resume;
        state.selected = emotional_core::continuity::clamp(
            position.selected,
            state.resume,
            state.order.len(),
            state.recent.is_some(),
            state.continuity_order.len(),
        );
        state.library = position.library;
        state.library_index = position
            .library_index
            .min(state.games.len().saturating_sub(1));
        if position.library {
            if let Some(view)=&position.library_view {
                state.library_browser.restore_saved(view,&state.games,Some(&position.game_id));
            } else {
                // Older request files predate grouped navigation. Open all
                // games, then recover the selected game by its stable ID.
                state.library_browser.choose(1,&state.games);
                state.library_browser.restore(position.library_index,Some(&position.game_id));
            }
            state.sync_library_game();
        }
    }
    state.order = discovery_presentation::presentation_order(
        &state.order,
        state.recent,
        &state.games.iter().map(|g| g.available).collect::<Vec<_>>(),
    );
    #[cfg(feature = "simulator")]
    simulator::install()?;
    #[cfg(feature = "gpu-material")]
    gpu_material::select_renderer()?;
    let ui = Home::new().inspect_err(|error| {
        smoke::emit("window_init_error", None, None, Some(&error.to_string()))
    })?;
    focus::install(&ui);
    ui.set_app_version(format!("v{}", include_str!("../VERSION").trim()).into());
    scene_raster::install_glass(&ui);
    #[cfg(feature = "gpu-material")]
    gpu_material::install(&ui);
    let system_weak = ui.as_weak();
    state.system_service = Some(emotional_services::system::Runtime::start_with_notify(
        functional_mode,
        move || {
            let _ = system_weak.upgrade_in_event_loop(|ui| ui.invoke_system_refresh());
        },
    ));
    let _performance = performance::start(&ui);
    ui.set_concept(concept);
    ui.set_trial_mode(trial_mode);
    ui.set_art_available(!imported && visual_case.as_deref() != Some("no-art"));
    if concept > 0 && state.order.is_empty() { /* library remains reachable with an empty catalog */
    }
    let size = ui.window().size();
    smoke::emit(
        "window_created",
        Some(size.width),
        Some(size.height),
        Some("Window size only; does not verify LCD image"),
    );
    state.paint(&ui);
    #[cfg(any(debug_assertions, feature = "simulator"))]
    if dev_environment.is_some() {
        moments::verify_environment(&state, &ui);
    }
    if let Some(at) = state.clock.persistable_now() {
        state.clock.restore_last_known(Some(at));
        if let Err(e) = state.writer.clock(at) {
            eprintln!("Clock checkpoint skipped: {e}");
        }
    }
    println!("ENV scene={} moment={} source={:?} artwork=provisional-private; palette={}; offline-capable=true; weather=disabled; appearance=light-or-dark",ui.get_scene_id(),ui.get_scene_moment(),state.clock.context().source,ui.global::<DaycycleAssets>().get_palette_index());
    if library_unavailable {
        state.notify(
            &ui,
            "Tu biblioteca no está disponible. Puedes volver cuando esté conectada.",
        );
    }
    if let Some(back) = &returned {
        if !library_unavailable {
            state.notify(&ui, &back.message);
        }
    }
    println!(
        "EmotionalUI 0.2.0 | clock={:?} | timezone={} | imported={} | launcher_configured={}",
        state.clock.trust,
        state.clock.timezone(),
        imported,
        state.launch.is_some()
    );
    if trial_mode {
        ui.set_message("Modo prueba · Buscando tu colección sin cambiar el sistema…".into());
    }
    let shared = Rc::new(RefCell::new(state));
    {
        let weak = ui.as_weak();
        let state = shared.clone();
        ui.on_system_refresh(move || {
            if let Some(ui) = weak.upgrade() {
                state.borrow_mut().refresh_system(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = shared.clone();
        ui.on_system_select(move |index| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.system_index = index.clamp(0, 6) as usize;
                state.system_action(&ui, emotional_core::controls::Action::Confirm);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = shared.clone();
        ui.on_system_adjust(move |index, forward| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.system_index = index.clamp(0, 6) as usize;
                state.system_action(
                    &ui,
                    if forward {
                        emotional_core::controls::Action::Right
                    } else {
                        emotional_core::controls::Action::Left
                    },
                );
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = shared.clone();
        ui.on_system_level(move |index, value| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if matches!(index, 2 | 3) && (0..=100).contains(&value) {
                    state.set_system_level(index as usize, value as u8);
                    state.paint(&ui);
                }
            }
        });
    }
    // Start work only after window/model construction. No XML/thumbnail decode in callbacks.
    let drop_refresh_profile = launcher_profile.clone().filter(|_| functional_mode);
    let drop_refresh_db = path.clone();
    let drop_refresh_root = trial_root.clone();
    let drop_refresh_atlas = atlas_path.clone();
    let trial_updates = (trial_mode || functional_mode).then(|| {
        if let Some(profile) = launcher_profile.filter(|_| functional_mode) {
            return trial::start_functional(
                path.clone(),
                trial_root.clone(),
                atlas_path.clone(),
                atlas_experimental,
                profile,
            );
        }
        trial::start(
            path.clone(),
            trial_root.clone(),
            atlas_path.clone(),
            atlas_experimental,
        )
    });
    let trial_timer = slint::Timer::default();
    if let Some(mut updates) = trial_updates {
        let weak = ui.as_weak();
        let s = shared.clone();
        trial_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let refresh = {
                    let mut state = s.borrow_mut();
                    std::mem::take(&mut state.drop_refresh)
                };
                if refresh {
                    if let Some(profile) = drop_refresh_profile.clone() {
                        updates = trial::start_functional(drop_refresh_db.clone(), drop_refresh_root.clone(),
                            drop_refresh_atlas.clone(), atlas_experimental, profile);
                    }
                }
                if let (Some(ui), Ok(update)) = (weak.upgrade(), updates.try_recv()) {
                    let mut state = s.borrow_mut();
                    match update {
                        Ok(mut update) => {
                            state.remember_focus();
                            let focused =
                                state.games.get(state.library_index).map(|g| g.id.clone());
                            let resume_focused_id = if state.resume {
                                let index = match state.selected {
                                    0 => state.recent,
                                    4 | 5 => {
                                        state.continuity_order.get(state.selected - 3).copied()
                                    }
                                    _ => None,
                                };
                                index.and_then(|i| state.games.get(i)).map(|g| g.id.clone())
                            } else {
                                None
                            };
                            trial::preserve_activity(&state.games, &mut update.games);
                            let open_id = state
                                .game_options
                                .and_then(|i| state.games.get(i).map(|g| g.id.clone()));
                            let had_game_modal = open_id.is_some();
                            state.game_options =
                                open_id.and_then(|id| update.games.iter().position(|g| g.id == id));
                            if state.game_options.is_none() && had_game_modal {
                                state.close_modal();
                            }
                            state.atlas_explanations = update.explanations;
                            state.games = update.games;
                            state.rebuild_library();
                            ui.set_library_offset(-1);
                            state.local_art = update.art;
                            ui.set_artwork_generation(ui.get_artwork_generation().wrapping_add(1));
                            ui.set_local_art_id("".into());
                            state.discovery_reserve = None;
                            state.library_index = focused
                                .and_then(|id| state.games.iter().position(|g| g.id == id))
                                .unwrap_or(0);
                            state.rebuild_continuity();
                            if let Some(id) = resume_focused_id {
                                if let Some(position) = state
                                    .continuity_order
                                    .iter()
                                    .position(|i| state.games[*i].id == id)
                                {
                                    state.selected = if position == 0 { 0 } else { position + 3 };
                                }
                            }
                            if let Err(e) = state.rerank() {
                                eprintln!("TRIAL ranking: {e}");
                            }
                            state.restore_remembered_focus();
                            state.paint(&ui);
                            let message = if functional_mode {
                                update.message.strip_prefix("Modo prueba · ").unwrap_or(&update.message)
                            } else {
                                &update.message
                            };
                            state.notify(&ui, message);
                            println!("TRIAL catalog_ready games={}", state.games.len());
                        }
                        Err(e) => {
                            eprintln!("TRIAL worker: {e}");
                            state.notify(
                                &ui,
                                "No pudimos leer parte de tu colección. Puedes seguir explorando.",
                            );
                        }
                    }
                }
            },
        );
    }
    let art_worker = artwork::Worker::start();
    let art_timer = slint::Timer::default();
    let weak = ui.as_weak();
    let s = shared.clone();
    let mut pending: Option<String> = None;
    let mut shown: [Option<String>; 6] = Default::default();
    let mut requested: [Option<String>; 6] = Default::default();
    art_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(30),
        move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let state = s.borrow();
            let indices = [
                state.order.first().copied(),
                state.order.get(1).copied(),
                state.continuity_order.get(1).copied(),
                state.continuity_order.get(2).copied(),
                state.recent,
                state.order.get(2).copied(),
            ];
            let desired: [Option<String>; 6] = std::array::from_fn(|slot| {
                indices[slot].and_then(|i| state.games.get(i)).map(|g| {
                    format!(
                        "{slot}:{}:{}:{}",
                        ui.get_artwork_generation(),
                        if slot == 4 { "resume" } else { "discover" },
                        g.id
                    )
                })
            });
            for slot in 0..6 {
                if shown[slot] != desired[slot] {
                    match slot {
                        0 => {
                            ui.set_local_cover(slint::Image::default());
                            ui.set_local_art_available(false);
                            ui.set_local_art_id("".into());
                        }
                        1 => {
                            ui.set_alternative_cover(slint::Image::default());
                            ui.set_alternative_art_available(false);
                        }
                        2 => {
                            ui.set_resume_two_cover(slint::Image::default());
                            ui.set_resume_two_art_available(false);
                        }
                        3 => {
                            ui.set_resume_three_cover(slint::Image::default());
                            ui.set_resume_three_art_available(false);
                        }
                        5 => {ui.set_third_cover(slint::Image::default());ui.set_third_art_available(false);}
                        _ => {
                            ui.set_resume_cover(slint::Image::default());
                            ui.set_resume_art_available(false);
                        }
                    }
                    shown[slot] = desired[slot].clone();
                    requested[slot] = None;
                }
            }
            if let Ok(thumbnail) = art_worker.ready.try_recv() {
                if let Some(slot) = desired
                    .iter()
                    .position(|id| id.as_ref() == Some(&thumbnail.id))
                {
                    if let Some(p) = thumbnail.prepared {
                        let buffer =
                            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                &p.pixels, p.width, p.height,
                            );
                        let image = slint::Image::from_rgba8(buffer);
                        match slot {
                            0 => {
                                ui.set_local_cover(image);
                                ui.set_local_art_available(true);
                                ui.set_local_art_id(thumbnail.id.clone().into());
                            }
                            1 => {
                                ui.set_alternative_cover(image);
                                ui.set_alternative_art_available(true);
                            }
                            2 => {
                                ui.set_resume_two_cover(image);
                                ui.set_resume_two_art_available(true);
                            }
                            3 => {
                                ui.set_resume_three_cover(image);
                                ui.set_resume_three_art_available(true);
                            }
                            5 => {ui.set_third_cover(image);ui.set_third_art_available(true);}
                            _ => {
                                ui.set_resume_cover(image);
                                ui.set_resume_art_available(true);
                            }
                        }
                        smoke::emit(
                            "local_art_ready",
                            Some(p.width),
                            Some(p.height),
                            Some(&format!(
                                "slot={slot}; mode={:?}; cache_hit={}; prepared off UI thread",
                                p.mode, p.cache_hit
                            )),
                        );
                    }
                }
                pending = None;
            }
            if pending.is_none() {
                // Only visible or immediately adjacent cards, never the entire catalog.
                let slots: &[usize] = if state.resume {
                    &[4, 2, 3, 0, 1, 5]
                } else {
                    &[0, 1, 5, 4, 2, 3]
                };
                for &slot in slots {
                    if requested[slot] == desired[slot] {
                        continue;
                    }
                    if let Some((id, game)) = desired[slot]
                        .as_ref()
                        .zip(indices[slot].and_then(|i| state.games.get(i)))
                    {
                        let (width, height) = if slot == 0 || slot == 4 {
                            (
                                if slot == 4 {
                                    ui.get_prepared_resume_width()
                                } else {
                                    ui.get_prepared_moment_width()
                                } as u32,
                                if slot == 4 {
                                    ui.get_prepared_resume_height()
                                } else {
                                    ui.get_prepared_moment_height()
                                } as u32,
                            )
                        } else {
                            (
                                ui.get_prepared_tile_width() as u32,
                                ui.get_prepared_tile_height() as u32,
                            )
                        };
                        if art_worker
                            .request
                            .try_send(artwork::Request {
                                id: id.clone(),
                                game_id: game.id.clone(),
                                resources: state
                                    .local_art
                                    .get(&game.id)
                                    .cloned()
                                    .unwrap_or_default(),
                                width,
                                height,
                                presentation: if slot == 0 || slot == 4 {
                                    artwork::Presentation::Hero
                                } else {
                                    artwork::Presentation::Tile
                                },
                            })
                            .is_ok()
                        {
                            pending = Some(id.clone());
                            requested[slot] = Some(id.clone());
                        }
                        break;
                    }
                    requested[slot] = desired[slot].clone();
                }
            }
        },
    );
    if day_demo.is_some() {
        trial_replay::day_demo(&ui, &shared);
    }
    #[cfg(feature = "simulator")]
    let _simulation = simulation
        .map(|path| simulator::replay(&path, &ui, shared.clone()))
        .transpose()?;
    let _gpu_replay = gpu_replay
        .then(|| trial_replay::start(&ui, &shared))
        .transpose()?;
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_key_action(move |k| {
        if let Some(ui) = weak.upgrade() {
            if ui.get_native_controls() {
                return;
            } // Native profile owns buttons; ignore virtual keyboard echoes.
            if let Err(e) = s.borrow_mut().key(&ui, &k) {
                eprintln!("Action failed: {e}");
                ui.set_message("No pudimos guardar el cambio. Inténtalo de nuevo.".into());
            }
        }
    });
    let s = shared.clone();
    ui.on_released(move |key| {
        let action = s.borrow_mut().keyboard.release(&key);
        if let Some(action) = action {
            s.borrow_mut().gate.release_key(action.id());
        }
    });
    let s = shared.clone();
    ui.on_input_reset(move || {
        if let Ok(mut state) = s.try_borrow_mut() {
            state.gate.release();
            state.keyboard.reset();
        } else {
            let state = s.clone();
            slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                let mut state = state.borrow_mut();
                state.gate.release();
                state.keyboard.reset();
            });
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_dismiss(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = s.borrow_mut();
            if let Err(e) = state.action(&ui, emotional_core::controls::Action::Back) {
                eprintln!("Back failed: {e}");
            }
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_activate(move |i| {
        if let Some(ui) = weak.upgrade() {
            if let Err(e) = s.borrow_mut().activate(&ui, i as usize) {
                eprintln!("Action failed: {e}");
                ui.set_message("No pudimos guardar el cambio.".into());
            }
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_library_search_key(move |key| {
        if let Some(ui)=weak.upgrade() {
            if key>=0 {s.borrow_mut().library_search_key(&ui,key as usize);}
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_system_toggle(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = s.borrow_mut();
            if state.system_open || !state.modal_active() {
                if state.system_open {
                    state.close_status();
                } else {
                    state.open_status();
                }
                state.paint(&ui);
            }
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_navigate(move |destination| {
        if let Some(ui) = weak.upgrade() {
            s.borrow_mut().navigate(&ui, destination as usize);
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_game_option_select(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = s.borrow_mut();
            if state.game_options.is_some() && (0..6).contains(&index) {
                state.game_option_index = index as usize;
                if let Err(e) =
                    state.game_options_action(&ui, emotional_core::controls::Action::Confirm)
                {
                    eprintln!("Game options: {e}");
                }
            }
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_menu_select(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = s.borrow_mut();
            if state.menu
                && !state.modal_active()
                && (0..state.settings_count() as i32).contains(&index)
            {
                state.dock_focus = None;
                state.system_focus = false;
                state.menu_index = index as usize;
                if let Err(e) = state.action(&ui, emotional_core::controls::Action::Confirm) {
                    eprintln!("Menu failed: {e}");
                }
            }
        }
    });
    let weak = ui.as_weak();
    let s = shared.clone();
    ui.on_audio_select(move |index, delta| {
        if let Some(ui) = weak.upgrade() {
            let mut state = s.borrow_mut();
            if state.audio_panel && (0..7).contains(&index) {
                state.audio_index = index as usize;
                if let Err(error) = state.modal_audio_action(
                    &ui,
                    if delta < 0 {
                        emotional_core::controls::Action::Left
                    } else {
                        emotional_core::controls::Action::Confirm
                    },
                ) {
                    eprintln!("Audio settings: {error}");
                }
            }
        }
    });
    #[cfg(target_os = "linux")]
    let _native_timer = if let Some(profile) = profile {
        let reader = emotional_services::gamepad::Gamepad::open(
            &profile.device_name,
            profile.device_path.as_deref(),
            &profile.mapping,
        )?;
        smoke::emit(
            "controller_opened",
            None,
            None,
            Some(&reader.path.display().to_string()),
        );
        let mut controller = emotional_core::controls::Controller::new(profile.mapping)?;
        let timer = slint::Timer::default();
        let weak = ui.as_weak();
        let s = shared.clone();
        let mut failed = false;
        ui.set_native_controls(true);
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(8),
            move || {
                if failed {
                    return;
                }
                if let Some(ui) = weak.upgrade() {
                    let mut state = s.borrow_mut();
                    let now = state.start.elapsed();
                    let mut actions = Vec::new();
                    match reader.drain() {
                        Ok(events) => {
                            for event in events {
                                actions.extend(controller.event(event, now));
                            }
                        }
                        Err(error) => {
                            controller.disconnect();
                            state.native_held = false;
                            failed = true;
                            smoke::emit("controller_reset", None, None, Some(&error));
                            ui.set_message("El mando se ha desconectado.".into());
                            return;
                        }
                    }
                    actions.extend(controller.tick(now));
                    state.native_held = controller.held();
                    for action in actions {
                        smoke::emit("controller_action", None, None, Some(action.id()));
                        if let Err(error) = state.action(&ui, action) {
                            eprintln!("Controller action failed: {error}");
                            ui.set_message("No pudimos guardar tu elección.".into());
                        } else {
                            smoke::emit("controller_state",None,None,Some(&format!("action={} selected={} library={} menu={} menu_index={} reduced={}",action.id(),state.selected,state.library,state.menu,state.menu_index,state.config.reduced_motion)));
                        }
                    }
                }
            },
        );
        Some(timer)
    } else {
        None
    };
    let launch_timer = slint::Timer::default();
    let weak = ui.as_weak();
    let st = shared.clone();
    launch_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(20),
        move || {
            if let Some(ui) = weak.upgrade() {
                st.borrow_mut().launch_tick(&ui);
            }
        },
    );
    let tick = slint::Timer::default();
    let mut last_context = shared
        .borrow()
        .clock
        .known_now()
        .map(|d| d.format("%Y%m%d%H%M").to_string());
    let mut last_ambient_context = last_context.clone();
    let weak = ui.as_weak();
    let s = shared.clone();
    tick.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = weak.upgrade() {
                let st = s.borrow();
                ui.set_audio_status(st.audio_status().into());
                if let Some(error) = st.writer.error() {
                    eprintln!("Persistence failed: {error}");
                    ui.set_message("No pudimos guardar tu elección.".into());
                }
                // Keep the clock visible in open panels without changing the
                // selected light/dark appearance or querying weather.
                let ambient_context = st
                    .clock
                    .known_now()
                    .map(|d| d.format("%Y%m%d%H%M").to_string());
                if ambient_context != last_ambient_context {
                    if st.menu || st.library {
                        st.paint(&ui);
                    }
                    last_ambient_context = ambient_context;
                }
                if !st.menu
                    && !st.library
                    && !st.gate.is_held()
                    && !st.native_held
                    && st.start.elapsed() > std::time::Duration::from_secs(1)
                {
                    let context = st
                        .clock
                        .known_now()
                        .map(|d| d.format("%Y%m%d%H%M").to_string());
                    if context != last_context {
                        drop(st);
                        let mut st = s.borrow_mut();
                        if let Err(e) = st.sync_context(&ui) {
                            eprintln!("Context update failed: {e}");
                        }
                        last_context = context;
                    }
                }
            }
        },
    );
    if (snapshot.is_some() || std::env::var_os("EMOTIONAL_GLASS_CAPTURE").is_some())
        && snapshot_view != "home"
    {
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(500), move || {
            if let Some(ui) = weak.upgrade() {
                let key = match snapshot_view.as_str() {
                    "library" => "y".into(),
                    "menu" => char::from(slint::platform::Key::F1).to_string(),
                    "focus" => char::from(slint::platform::Key::DownArrow).to_string(),
                    _ => "z".into(),
                };
                ui.window()
                    .dispatch_event(slint::platform::WindowEvent::KeyPressed {
                        text: key.clone().into(),
                    });
                ui.window()
                    .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: key.into() });
            }
        });
    }
    let probe = slint::Timer::default();
    let weak = ui.as_weak();
    probe.start(
        slint::TimerMode::SingleShot,
        std::time::Duration::from_millis(250),
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_motion_ready(true);
                let size = ui.window().size();
                smoke::emit(
                    "event_loop_alive",
                    Some(size.width),
                    Some(size.height),
                    Some("No physical presentation assertion"),
                );
            }
        },
    );
    let cancellation = slint::Timer::default();
    if let Some(path) = quit_file {
        let weak = ui.as_weak();
        cancellation.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                if path.exists() {
                    smoke::emit("external_quit_requested", None, None, None);
                    if let Some(ui) = weak.upgrade() {
                        let _ = ui.hide();
                    }
                    let _ = slint::quit_event_loop();
                }
            },
        );
    }
    let exit = slint::Timer::default();
    let capture_error = Rc::new(RefCell::new(None::<String>));
    if smoke || snapshot.is_some() {
        let capture_error = capture_error.clone();
        let weak = ui.as_weak();
        exit.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_secs(smoke_seconds),
            move || {
                if let Some(ui) = weak.upgrade() {
                    if ui_test {
                        if ui.get_concept() == 0 {
                            run_ui_checks(&ui);
                        } else if ui.get_concept() == 4 {
                            run_integrated_checks(&ui);
                        } else {
                            run_concept_checks(&ui);
                        }
                    }
                    if let Some(path) = &snapshot {
                        match ui.window().take_snapshot() {
                            Ok(buf) => {
                                if let Err(e) = capture::save_ui(std::path::Path::new(path), &buf) {
                                    *capture_error.borrow_mut() = Some(e.to_string());
                                } else {
                                    println!("Snapshot: {path}");
                                }
                            }
                            Err(e) => *capture_error.borrow_mut() = Some(e.to_string()),
                        }
                    }
                    smoke::emit("smoke_deadline", None, None, None);
                    let _ = ui.hide();
                    let _ = slint::quit_event_loop();
                }
            },
        );
    }
    #[cfg(feature = "gpu-material")]
    gpu_material::arm_capture(&ui);
    ui.run()?;
    #[cfg(feature = "simulator")]
    if let Some(replay) = &_simulation {
        if !shared
            .borrow()
            .launch
            .as_ref()
            .is_some_and(|c| c.committed.is_some())
        {
            replay.finish()?;
        }
    }
    shared.borrow_mut().audio.finish()?;
    smoke::emit(
        "audio_final_status",
        None,
        None,
        Some(&format!(
            "{:?}; audibility unverified",
            shared.borrow().audio.status()
        )),
    );
    shared.borrow_mut().writer.finish()?;
    {
        let state = shared.borrow();
        if let Some(client) = &state.launch {
            if let Some(request) = &client.committed {
                supervisor::write(&client.directory.join("request.toml"), request)?;
            }
        }
    }

    smoke::emit(
        "event_loop_returned",
        None,
        None,
        Some("Resource release requested; system recovery unverified"),
    );
    if let Some(error) = capture_error.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

/// Native window event injection exercises the same FocusScope as the keyboard.
fn run_ui_checks(ui: &Home) {
    fn send(ui: &Home, key: impl Into<slint::SharedString>) {
        let text = key.into();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    }
    if ui.get_recommendation_count() < 3 {
        let count = ui.get_recommendation_count();
        for key in [
            slint::platform::Key::DownArrow,
            slint::platform::Key::RightArrow,
            slint::platform::Key::UpArrow,
            slint::platform::Key::LeftArrow,
        ] {
            send(ui, key);
            assert!(ui.get_selected() < count.max(1));
        }
        send(ui, "z");
        send(ui, "n");
        assert_eq!(ui.get_recommendation_count(), count);
        send(ui, "v");
        send(ui, slint::platform::Key::DownArrow);
        send(ui, "x");
        assert!(!ui.get_library_open());
        println!("UI small-catalog checks passed: {count} games");
        return;
    }
    assert_eq!(ui.get_selected(), 0);
    send(ui, slint::platform::Key::DownArrow);
    assert_eq!(ui.get_selected(), 1);
    send(ui, slint::platform::Key::UpArrow);
    assert_eq!(ui.get_selected(), 0);
    send(ui, "z");
    assert!(ui.get_message().contains("ficticio"));
    send(ui, "v");
    assert!(ui.get_library_open());
    send(ui, slint::platform::Key::DownArrow);
    assert_eq!(ui.get_library_index(), 1);
    send(ui, "z");
    assert!(!ui.get_library_open());
    assert_eq!(ui.get_hero_title(), "Jardín de nubes");
    send(ui, slint::platform::Key::F1);
    assert!(ui.get_menu_open());
    let reduced = ui.get_reduced();
    send(ui, "m");
    assert_ne!(ui.get_reduced(), reduced);
    let mood = ui.get_mood();
    send(ui, "]");
    assert_eq!(ui.get_mood(), (mood + 1) % 4);
    send(ui, "x");
    assert!(!ui.get_menu_open());
    send(ui, "n");
    assert_eq!(ui.get_selected(), 0);
    // Close a panel and continue navigating before its fade-out completes.
    send(ui, "v");
    send(ui, "x");
    send(ui, slint::platform::Key::DownArrow);
    assert_eq!(ui.get_selected(), 1);
    send(ui, slint::platform::Key::UpArrow);
    assert_eq!(ui.get_selected(), 0);
    // Interleaved held controls must not re-arm or bypass repeat timing.
    let down = slint::SharedString::from(char::from(slint::platform::Key::DownArrow).to_string());
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: down.clone() });
    assert_eq!(ui.get_selected(), 1);
    send(ui, slint::platform::Key::RightArrow);
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: down.clone() });
    assert_eq!(ui.get_selected(), 2);
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: down });
    send(ui, slint::platform::Key::UpArrow);
    for _ in 0..30 {
        send(ui, slint::platform::Key::DownArrow);
        send(ui, slint::platform::Key::RightArrow);
        send(ui, slint::platform::Key::LeftArrow);
        send(ui, slint::platform::Key::UpArrow);
        assert_eq!(ui.get_selected(), 0);
    }
    send(ui, slint::platform::Key::DownArrow);
    send(ui, slint::platform::Key::F1);
    assert!(ui.get_menu_open());
    send(ui, slint::platform::Key::DownArrow);
    assert_eq!(ui.get_menu_index(), 1);
    send(ui, slint::platform::Key::UpArrow);
    let reduced = ui.get_reduced();
    send(ui, "z");
    assert_ne!(ui.get_reduced(), reduced);
    send(ui, "x");
    assert_eq!(ui.get_selected(), 1);
    send(ui, "v");
    let index = ui.get_library_index();
    send(ui, slint::platform::Key::F1);
    send(ui, "x");
    assert!(ui.get_library_open());
    assert_eq!(ui.get_library_index(), index);
    send(ui, "x");
    send(ui, slint::platform::Key::UpArrow);
    send(ui, "r");
    assert!(ui.get_library_open());
    send(ui, "l");
    assert!(!ui.get_library_open());
    println!("UI checks: passed (native key events, focus, demo action, library, menu, motion, mood, refresh, rapid presses, held controls, panel focus restoration)");
}

fn run_concept_checks(ui: &Home) {
    fn send(ui: &Home, key: impl Into<slint::SharedString>) {
        let text = key.into();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    }
    for _ in 0..20 {
        send(ui, slint::platform::Key::DownArrow);
        send(ui, slint::platform::Key::DownArrow);
        assert_eq!(ui.get_selected(), 2);
        send(ui, "z");
        assert!(ui.get_library_open());
        send(ui, "s");
        assert!(ui.get_menu_open());
        send(ui, "x");
        assert!(ui.get_library_open());
        send(ui, "x");
        assert!(!ui.get_library_open());
        send(ui, slint::platform::Key::UpArrow);
        send(ui, slint::platform::Key::UpArrow);
        assert_eq!(
            ui.get_selected(),
            if ui.get_recommendation_count() == 0 {
                2
            } else {
                0
            }
        );
    }
    println!("Concept UI checks passed: {}", ui.get_concept());
}

fn run_integrated_checks(ui: &Home) {
    fn send(ui: &Home, key: impl Into<slint::SharedString>) {
        let text = key.into();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    }
    assert_eq!(
        ui.get_section_title().as_str(),
        if ui.get_resume_view() {
            "Continuar"
        } else {
            "Descubre"
        }
    );
    // Four destinations replace the former two-tab footer. Keep coverage for
    // empty libraries, focus restoration, library access, panels and privacy.
    send(ui, "x");
    send(ui, "x");
    let timing = Instant::now();
    for _ in 0..30 {
        send(ui, "q");
        let view = ui.get_resume_view();
        send(ui, "e");
        assert_ne!(ui.get_resume_view(), view);
        assert_eq!(
            ui.get_section_title().as_str(),
            if ui.get_resume_view() {
                "Continuar"
            } else {
                "Descubre"
            }
        );
        let available = if ui.get_resume_view() {
            ui.get_has_recent()
        } else {
            ui.get_recommendation_count() > 0
        };
        if available {
            send(ui, slint::platform::Key::RightArrow);
            assert_eq!(ui.get_dock_cursor(), 2);
            send(ui, "z");
        } else {
            send(ui, "a");
        }
        assert!(ui.get_library_open());
        assert_eq!(ui.get_active_section(), 2);
        send(ui, slint::platform::Key::UpArrow);
        send(ui, "s");
        assert!(ui.get_menu_open());
        send(ui, "x");
        assert!(ui.get_library_open());
        send(ui, "x");
        assert!(!ui.get_library_open());
        if available {
            assert_eq!(ui.get_selected(), 0);
        }
    }
    println!("UI_ACTION_BENCH cycles=30 elapsed_ms={:.3}; software_dispatch_only=true; input_to_photon=UNMEASURED",timing.elapsed().as_secs_f64()*1000.0);
    if !ui.get_resume_view() {
        send(ui, "q");
    }
    if ui.get_has_recent() {
        send(ui, slint::platform::Key::RightArrow);
        assert_eq!(ui.get_dock_cursor(), 2);
        send(ui, slint::platform::Key::LeftArrow);
        assert_eq!(ui.get_dock_cursor(), 1);
        assert_eq!(ui.get_active_section(), 0);
        send(ui, "z");
        assert_eq!(ui.get_active_section(), 1);
        send(ui, slint::platform::Key::UpArrow);
        send(ui, "q");
    }
    send(ui, "s");
    let p = ui.get_statistics_on();
    send(ui, slint::platform::Key::DownArrow);
    send(ui, slint::platform::Key::DownArrow);
    send(ui, "z");
    assert_ne!(ui.get_statistics_on(), p);
    send(ui, "z");
    assert_eq!(ui.get_statistics_on(), p);
    send(ui, "x");
    // Audio settings use the same button events; close returns to the previous menu focus.
    send(ui, "s");
    send(ui, slint::platform::Key::UpArrow);
    send(ui, "z");
    assert!(ui.get_audio_open());
    let mode = ui.get_audio_mode();
    send(ui, slint::platform::Key::RightArrow);
    assert_ne!(ui.get_audio_mode(), mode);
    send(ui, slint::platform::Key::LeftArrow);
    assert_eq!(ui.get_audio_mode(), mode);
    send(ui, slint::platform::Key::DownArrow);
    let volume = ui.get_audio_master();
    send(ui, slint::platform::Key::LeftArrow);
    assert_eq!(ui.get_audio_master(), (volume - 5).max(0));
    send(ui, "x");
    assert!(!ui.get_audio_open());
    assert!(ui.get_menu_open());
    assert_eq!(ui.get_menu_index(), 1);
    send(ui, "x");
    println!("Integrated UI checks passed: immediate view switching, library <=2 actions, stable focus, panels, privacy");
    // Exercise the actual Slint stagger binding at a partial position, without
    // pretending a synthetic setter measures presented animation timing.
    let reduced = ui.get_reduced();
    ui.set_reduced(true);
    ui.set_section_position(0.5);
    assert!(ui.get_discovery_main() > ui.get_discovery_secondary());
    assert_eq!(ui.get_discovery_secondary(), 0.0);
    ui.set_section_position(0.76);
    assert!(ui.get_discovery_secondary() > 0.0 && ui.get_discovery_main() < 1.0);
    ui.set_section_position(ui.get_section_target());
    ui.set_reduced(reduced);
    // Interrupt ambient interpolation from its CURRENT rendered colors; no jumps.
    let saved_reduced = ui.get_reduced();
    ui.set_reduced(false);
    let style = ui.global::<DaycycleAssets>();
    let saved_index = style.get_palette_index();
    // A continuous blend can quantize to the next keyframe's identical 8-bit
    // colors. Select two genuinely different targets for this transition test;
    // a no-op must not be mistaken for a failed animation.
    use slint::Model;
    let saved_next = style.get_palette_next_index();
    let saved_mix = style.get_day_mix();
    let original_target = style.get_target_colors().iter().collect::<Vec<_>>();
    let mut distinct = Vec::new();
    let mut previous = original_target.clone();
    for index in 0..7 {
        style.set_palette_index(index);
        style.set_palette_next_index(index);
        style.set_day_mix(0.);
        let target = style.get_target_colors().iter().collect::<Vec<_>>();
        if target != original_target && target != previous {
            distinct.push(index);
            previous = target;
            if distinct.len() == 2 {
                break;
            }
        }
    }
    style.set_palette_index(saved_index);
    style.set_palette_next_index(saved_next);
    style.set_day_mix(saved_mix);
    assert_eq!(
        distinct.len(),
        2,
        "Ambient palettes must contain distinct targets"
    );
    // Slint .alpha is 0..255; with-alpha expects 0..1. Preserve translucent grades.
    assert!(style.get_background_tint().alpha() < 40);
    assert!(style.get_active().alpha() < 70);
    let start = style.get_ink();
    ambient_motion::transition(ui, distinct[0], false);
    assert_eq!(style.get_palette_progress(), 0.);
    assert_eq!(style.get_ink(), start);
    style.set_palette_progress(0.5);
    let halfway = style.get_ink();
    let generation = ui.get_ambient_generation();
    ambient_motion::transition(ui, distinct[1], false);
    assert_eq!(style.get_ink(), halfway);
    assert_ne!(ui.get_ambient_generation(), generation);
    ui.set_reduced(true);
    ambient_motion::transition(ui, saved_index, false);
    assert_eq!(style.get_palette_progress(), 1.);
    // Same time, different environment: accents change, text alpha stays valid.
    let season = style.get_season_index();
    let weather = style.get_weather_index();
    let celebration = style.get_celebration_index();
    let blend = emotional_core::ambient_art::Blend {
        from: 3,
        to: 4,
        mix: 0.5,
    };
    ambient_motion::update(ui, blend, 0, 4, 0, true);
    let snow = style.get_mark();
    let ink = style.get_ink();
    ambient_motion::update(ui, blend, 0, 4, 2, true);
    assert_ne!(style.get_mark(), snow);
    assert_eq!(style.get_ink(), ink);
    assert!(style.get_background_tint().alpha() < 40);
    let generation = ui.get_ambient_generation();
    ambient_motion::update(ui, blend, 0, 4, 2, true);
    assert_eq!(ui.get_ambient_generation(), generation); // No repeated work at rest.
    let winter_secondary = style.get_muted();
    ambient_motion::update(ui, blend, 3, 2, 1, true);
    assert_ne!(style.get_muted(), winter_secondary);
    assert!(style.get_background_tint().alpha() < 40);
    ambient_motion::update(
        ui,
        emotional_core::ambient_art::Blend::still(saved_index as usize),
        season,
        weather,
        celebration,
        true,
    );
    ui.set_reduced(saved_reduced);
}
