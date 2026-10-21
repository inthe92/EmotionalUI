//! Opt-in, bounded diagnostics. No per-frame disk writes or presentation claims.
use crate::{AtmosphereDesign, Home};
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    time::{Duration, Instant},
};

const LIMIT: usize = 16384;
#[derive(Default)]
struct Metrics {
    samples: BTreeMap<&'static str, Vec<f64>>,
    dropped: usize,
    #[cfg(feature = "simulator")]
    pending_input: Option<Instant>,
    #[cfg(feature = "simulator")]
    last_animation_frame: Option<Instant>,
    frame_buckets: BTreeMap<u64, usize>,
}
thread_local! { static METRICS: RefCell<Option<Metrics>> = const { RefCell::new(None) }; }
pub struct Session {
    _heartbeat: slint::Timer,
}
pub fn start(ui: &Home) -> Option<Session> {
    let mode = std::env::var("EMOTIONAL_PERF").ok()?;
    let index = match mode.as_str() {
        "full" => 0,
        "static" => 1,
        "simple" => 2,
        _ => return None,
    };
    METRICS.with(|m| *m.borrow_mut() = Some(Metrics::default()));
    ui.global::<AtmosphereDesign>().set_diagnostic_mode(index);
    println!("PERF mode={mode}; diagnostic_only=true; physical_input_to_photon=UNMEASURED");
    let timer = slint::Timer::default();
    let mut last = Instant::now();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let now = Instant::now();
            record(
                "event_loop_lateness_ms",
                now.duration_since(last)
                    .saturating_sub(Duration::from_millis(16))
                    .as_secs_f64()
                    * 1000.,
            );
            last = now;
        },
    );
    Some(Session { _heartbeat: timer })
}
pub fn enabled() -> bool {
    METRICS.with(|m| m.borrow().is_some())
}
pub fn record(name: &'static str, ms: f64) {
    METRICS.with(|m| {
        if let Some(m) = m.borrow_mut().as_mut() {
            let samples = m.samples.entry(name).or_default();
            if samples.len() < LIMIT {
                samples.push(ms);
            } else {
                m.dropped += 1;
            }
        }
    });
}
pub struct Span(&'static str, Option<Instant>);
pub fn span(name: &'static str) -> Span {
    Span(
        name,
        METRICS.with(|m| m.borrow().as_ref().map(|_| Instant::now())),
    )
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(at) = self.1 {
            record(self.0, at.elapsed().as_secs_f64() * 1000.);
        }
    }
}
pub fn input() {
    #[cfg(feature = "simulator")]
    METRICS.with(|m| {
        if let Some(m) = m.borrow_mut().as_mut() {
            m.pending_input.get_or_insert_with(Instant::now);
        }
    });
}
#[cfg(feature = "simulator")]
pub fn raster_finished() {
    let at = METRICS.with(|m| m.borrow_mut().as_mut().and_then(|m| m.pending_input.take()));
    if let Some(at) = at {
        record(
            "dispatch_to_raster_end_ms_NOT_LCD",
            at.elapsed().as_secs_f64() * 1000.,
        );
    }
}
#[cfg(feature = "simulator")]
pub fn frame(at: Instant, elapsed: Duration, animated: bool) {
    let previous = METRICS.with(|m| {
        let mut m = m.borrow_mut();
        let m = m.as_mut()?;
        let second = elapsed.as_secs();
        if m.frame_buckets.len() < LIMIT || m.frame_buckets.contains_key(&second) {
            *m.frame_buckets.entry(second).or_default() += 1;
        } else {
            m.dropped += 1;
        }
        let previous = m.last_animation_frame.take();
        if animated {
            m.last_animation_frame = Some(at);
        }
        animated.then_some(previous).flatten()
    });
    if let Some(previous) = previous {
        if at.duration_since(previous) < Duration::from_millis(500) {
            record(
                "animation_raster_interval_ms_NOT_LCD",
                at.duration_since(previous).as_secs_f64() * 1000.,
            );
        }
    }
}
fn percentile(sorted: &[f64], percentile: usize) -> f64 {
    sorted[(sorted.len() * percentile)
        .div_ceil(100)
        .saturating_sub(1)
        .min(sorted.len() - 1)]
}
impl Drop for Session {
    fn drop(&mut self) {
        self._heartbeat.stop();
        METRICS.with(|m| { if let Some(m) = m.borrow_mut().take() {
            println!("PERF raster_frames_by_second={:?}; NOT_LCD=true",m.frame_buckets);
            for (name, mut values) in m.samples { values.sort_by(f64::total_cmp);
                println!("PERF metric={name} n={} median={:.3} p95={:.3} max={:.3} total={:.3}",values.len(),percentile(&values,50),percentile(&values,95),values.last().unwrap(),values.iter().sum::<f64>());
            }
            println!("PERF dropped_samples={}; renderer_notifier_software=UNSUPPORTED; LCD_frame_times=UNMEASURED",m.dropped);
        }});
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_rank_percentiles_and_bounded_samples() {
        assert_eq!(percentile(&[1., 2., 3., 4., 5.], 50), 3.);
        assert_eq!(percentile(&[1., 2., 3., 4., 5.], 95), 5.);
        METRICS.with(|m| *m.borrow_mut() = Some(Metrics::default()));
        for _ in 0..LIMIT + 2 {
            record("test", 1.);
        }
        METRICS.with(|m| {
            let m = m.borrow_mut().take().unwrap();
            assert_eq!(m.samples["test"].len(), LIMIT);
            assert_eq!(m.dropped, 2);
        });
    }
}
