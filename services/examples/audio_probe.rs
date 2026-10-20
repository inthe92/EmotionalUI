//! Reproducible lifecycle/device probes. Audible output requires --native-music.
use emotional_core::audio::{Context, Mode, Preferences};
use emotional_services::ambient_audio::{
    assets::Library,
    mixer::{Mixer, BLOCK},
    output::{self, Backend},
    AmbientAudioEngine, Options,
};
use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};
fn wait(mut condition: impl FnMut() -> bool) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        if Instant::now() > deadline {
            return Err("audio probe timeout".into());
        }
        thread::sleep(Duration::from_millis(4));
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().any(|a| a == "--native-music") {
        let args = std::env::args().collect::<Vec<_>>();
        let seconds: u64 = args
            .windows(2)
            .find(|v| v[0] == "--seconds")
            .map(|v| v[1].parse())
            .transpose()?
            .unwrap_or(12);
        if !(2..=120).contains(&seconds) {
            return Err("Native music diagnostic requires 2..120 seconds".into());
        }
        // Explicit supervised diagnostic: bundled piano, low application gain,
        // no mixer changes, no display/input ownership, no persistent settings.
        let mut engine = AmbientAudioEngine::start(
            Preferences {
                mode: Mode::Ambient,
                master: 25,
                // Isolate the music fader from the normal quiet preset. The
                // master remains capped; nothing is saved or changed globally.
                music: 100,
                ..Preferences::default()
            },
            Options {
                backend: Backend::Native,
                lead_in_frames: Some(0),
                ..Options::default()
            },
        )?;
        for second in 1..=seconds {
            thread::sleep(Duration::from_secs(1));
            println!("native_music second={second} status={:?}", engine.status());
            if engine.status().error.is_some() {
                break;
            }
        }
        engine.finish()?;
        let status = engine.status();
        println!("native_music final={status:?}; audibility requires human confirmation");
        if let Some(error) = status.error {
            return Err(error.into());
        }
        if status.written_blocks == 0 {
            return Err("No PCM blocks written".into());
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--native-silent") {
        match output::open(Backend::Native) {
            Ok(mut out) => {
                for _ in 0..4 {
                    out.write(&[0; BLOCK])?;
                }
                out.drain();
                drop(out);
                println!("{{\"scope\":\"native host API, zero PCM only, not RG353V\",\"native_open_write_close\":true,\"music_audibility_verified\":false}}");
            }
            Err(error) => {
                eprintln!("{error}");
                println!("{{\"scope\":\"native host API\",\"native_open_write_close\":false,\"fallback_to_silence\":true}}");
            }
        }
        return Ok(());
    }
    let argv = std::env::args().collect::<Vec<_>>();
    let assets = argv
        .windows(2)
        .find(|v| v[0] == "--assets")
        .map(|v| std::path::PathBuf::from(&v[1]));
    let preferences = Preferences::default();
    let e = AmbientAudioEngine::start(
        preferences.clone(),
        Options {
            backend: Backend::Probe,
            lead_in_frames: Some(0),
            assets: assets.clone(),
        },
    )?;
    wait(|| e.status().output_open)?;
    thread::sleep(Duration::from_millis(220));
    let start = Instant::now();
    let ticket = e.suspend_for_game();
    wait(|| e.released(ticket))?;
    let release_ms = start.elapsed().as_secs_f64() * 1000.0;
    // A real synthetic child can start only after output release. It is not an emulator.
    assert!(!e.status().output_open);
    let mut child = std::process::Command::new("/usr/bin/true").spawn()?;
    let blocks = e.status().blocks;
    thread::sleep(Duration::from_millis(100));
    assert_eq!(e.status().blocks, blocks);
    assert!(child.wait()?.success());
    e.context(Context::default());
    e.return_from_game(Preferences {
        mode: Mode::Silence,
        ..preferences.clone()
    })?;
    thread::sleep(Duration::from_millis(80));
    assert!(!e.status().output_open);
    e.return_from_game(preferences.clone())?;
    wait(|| e.status().output_open)?;
    let ticket = e.suspend_for_game();
    wait(|| e.released(ticket))?;
    let status = e.status();
    let mut mix = Mixer::new(Library::load(assets.as_deref()), preferences, Some(0));
    let t = Instant::now();
    for _ in 0..10_000 {
        mix.render(&mut [0; BLOCK]);
    }
    let rendering_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("{{\"scope\":\"silent timed output simulation, not physical audio\",\"release_ms\":{release_ms:.3},\"resource_pcm_bytes\":{},\"written_blocks\":{},\"rendered_peak\":{},\"opens\":{},\"closes\":{},\"simulated_emulator_overlap\":false,\"render_5120000_frames_ms\":{rendering_ms:.3}}}",status.resource_bytes,status.written_blocks,status.peak,status.opens,status.closes);
    Ok(())
}
