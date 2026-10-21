// SPDX-License-Identifier: GPL-3.0-only
//! Opt-in bounded DRM page-flip diagnostics. Kernel flip completion is not an
//! ioctl submission or a photon/input-to-LCD measurement. No timers or I/O/frame.
use std::time::{Duration, Instant};

const LIMIT: usize = 16384;
#[derive(Clone, Copy, Debug, PartialEq)]
struct FlipStamp {
    sequence: u32,
    kernel_us: u64,
}

pub(crate) struct PresentationMetrics {
    pub present_calls: u64,
    pub page_flip_submitted: u64,
    pub initial_modesets: u64,
    pub submit_errors: u64,
    pub confirmed: u64,
    pub receive_calls: u64,
    pub receive_errors: u64,
    pub other_crtc_events: u64,
    pub unexpected_events: u64,
    pub invalid_timestamps: u64,
    pub duplicate_sequences: u64,
    pub backwards_sequences: u64,
    pub refresh_slots_without_update: u64,
    pub blocked_waits: u64,
    pub dropped_samples: u64,
    timestamp_monotonic: Option<bool>,
    mode_hz: u32,
    first: Option<FlipStamp>,
    last: Option<FlipStamp>,
    interval_ms: Vec<f64>,
    wait_ms: Vec<f64>,
    submit_to_observed_ms: Vec<f64>,
    submitted_at: Option<Instant>,
    trace: Option<Vec<FlipStamp>>,
    trace_dropped: u64,
    finished: bool,
}
impl PresentationMetrics {
    pub fn new(timestamp_monotonic: Option<bool>, mode_hz: u32) -> Self {
        Self::with_trace(
            timestamp_monotonic,
            mode_hz,
            std::env::var("EMOTIONAL_PERF_TRACE").as_deref() == Ok("1"),
        )
    }
    fn with_trace(timestamp_monotonic: Option<bool>, mode_hz: u32, trace: bool) -> Self {
        Self {
            present_calls: 0,
            page_flip_submitted: 0,
            initial_modesets: 0,
            submit_errors: 0,
            confirmed: 0,
            receive_calls: 0,
            receive_errors: 0,
            other_crtc_events: 0,
            unexpected_events: 0,
            invalid_timestamps: 0,
            duplicate_sequences: 0,
            backwards_sequences: 0,
            refresh_slots_without_update: 0,
            blocked_waits: 0,
            dropped_samples: 0,
            timestamp_monotonic,
            mode_hz,
            first: None,
            last: None,
            interval_ms: Vec::new(),
            wait_ms: Vec::new(),
            submit_to_observed_ms: Vec::new(),
            submitted_at: None,
            trace: trace.then(Vec::new),
            trace_dropped: 0,
            finished: false,
        }
    }
    pub fn submitted(&mut self) {
        self.page_flip_submitted = self.page_flip_submitted.saturating_add(1);
        self.submitted_at = Some(Instant::now());
    }
    fn push(samples: &mut Vec<f64>, value: f64) -> bool {
        if samples.len() < LIMIT {
            samples.push(value);
            true
        } else {
            false
        }
    }
    pub fn waited(&mut self, elapsed: Duration) {
        self.blocked_waits = self.blocked_waits.saturating_add(1);
        if !Self::push(&mut self.wait_ms, elapsed.as_secs_f64() * 1000.) {
            self.dropped_samples = self.dropped_samples.saturating_add(1);
        }
    }
    pub fn flipped(&mut self, sequence: u32, stamp: Duration) {
        self.confirmed = self.confirmed.saturating_add(1);
        // Event observation may occur AFTER the next raster: this is not the
        // kernel submit-to-flip latency, hence the explicit metric label.
        if let Some(at) = self.submitted_at.take() {
            if !Self::push(
                &mut self.submit_to_observed_ms,
                at.elapsed().as_secs_f64() * 1000.,
            ) {
                self.dropped_samples = self.dropped_samples.saturating_add(1);
            }
        }
        let Ok(kernel_us) = u64::try_from(stamp.as_micros()) else {
            self.invalid_timestamps = self.invalid_timestamps.saturating_add(1);
            return;
        };
        let event = FlipStamp {
            sequence,
            kernel_us,
        };
        if let Some(trace) = &mut self.trace {
            if trace.len() < LIMIT {
                trace.push(event);
            } else {
                self.trace_dropped = self.trace_dropped.saturating_add(1);
            }
        }
        if let Some(last) = self.last {
            let delta = sequence.wrapping_sub(last.sequence);
            if delta == 0 {
                self.duplicate_sequences = self.duplicate_sequences.saturating_add(1);
            } else if delta >= (1_u32 << 31) {
                self.backwards_sequences = self.backwards_sequences.saturating_add(1);
            } else {
                self.refresh_slots_without_update = self
                    .refresh_slots_without_update
                    .saturating_add(u64::from(delta - 1));
            }
            if let Some(us) = kernel_us
                .checked_sub(last.kernel_us)
                .filter(|value| *value > 0)
            {
                if !Self::push(&mut self.interval_ms, us as f64 / 1000.) {
                    self.dropped_samples = self.dropped_samples.saturating_add(1);
                }
            } else {
                self.invalid_timestamps = self.invalid_timestamps.saturating_add(1);
            }
        }
        self.first.get_or_insert(event);
        self.last = Some(event);
    }
    pub fn report(&self) {
        let pending = self.page_flip_submitted.saturating_sub(self.confirmed);
        println!("KMS_DRM present_calls={} page_flips_submitted={} initial_modesets_without_event={} kernel_confirmed={} pending_or_unobserved={} submit_errors={} receive_calls={} receive_errors={} other_crtc_events={} unexpected_events={} timestamp_monotonic={:?} nominal_mode_hz={} photons=UNMEASURED input_to_photon=UNMEASURED",self.present_calls,self.page_flip_submitted,self.initial_modesets,self.confirmed,pending,self.submit_errors,self.receive_calls,self.receive_errors,self.other_crtc_events,self.unexpected_events,self.timestamp_monotonic,self.mode_hz);
        if let (Some(first), Some(last)) = (self.first, self.last) {
            println!("KMS_DRM first_sequence={} last_sequence={} first_kernel_us={} last_kernel_us={} invalid_timestamps={} duplicate_sequences={} backwards_sequences={} refresh_slots_without_new_ui_update={} dropped_samples={} cadence_window_includes_idle=true lost_ui_frames=UNMEASURED",first.sequence,last.sequence,first.kernel_us,last.kernel_us,self.invalid_timestamps,self.duplicate_sequences,self.backwards_sequences,self.refresh_slots_without_update,self.dropped_samples);
            if self.timestamp_monotonic == Some(true)
                && self.confirmed > 1
                && last.kernel_us > first.kernel_us
                && self.invalid_timestamps == 0
                && self.duplicate_sequences == 0
                && self.backwards_sequences == 0
            {
                let hz = (self.confirmed - 1) as f64 * 1_000_000.
                    / (last.kernel_us - first.kernel_us) as f64;
                println!("KMS_DRM kernel_update_hz_over_observed_window={hz:.3} basis=flip_completion_timestamps_NOT_ioctl_or_timer");
            }
        }
        report_samples("kernel_flip_interval_ms", &self.interval_ms);
        report_samples("blocking_wait_for_previous_flip_ms", &self.wait_ms);
        report_samples(
            "submit_to_event_observed_ms_NOT_kernel_latency",
            &self.submit_to_observed_ms,
        );
    }
    fn trace_line(&self) -> Option<String> {
        use std::fmt::Write;
        let trace = self.trace.as_ref()?;
        let mut line = format!(
            "KMS_DRM_TRACE format=sequence:kernel_us retained={} dropped={} stamps=[",
            trace.len(),
            self.trace_dropped
        );
        for (index, stamp) in trace.iter().enumerate() {
            if index > 0 {
                line.push(',');
            }
            let _ = write!(line, "{}:{}", stamp.sequence, stamp.kernel_us);
        }
        line.push(']');
        Some(line)
    }
    /// Aggregate and optional bounded trace only; never receive/drain/wait here.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.report();
        if let Some(line) = self.trace_line() {
            println!("{line}");
        }
    }
}
fn report_samples(name: &str, samples: &[f64]) {
    if samples.is_empty() {
        return;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let percentile = |p: usize| sorted[(n * p).div_ceil(100).saturating_sub(1)];
    let p99 = if n >= 100 {
        format!("{:.3}", percentile(99))
    } else {
        "UNAVAILABLE_LT100".into()
    };
    println!(
        "KMS_DRM metric={name} n={n} median={:.3} p95={:.3} p99={p99} max={:.3} total={:.3}",
        percentile(50),
        percentile(95),
        sorted[n - 1],
        sorted.iter().sum::<f64>()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kernel_sequence_wrap_and_intervals_are_not_submission_counts() {
        let mut metrics = PresentationMetrics::new(Some(true), 60);
        metrics.submitted();
        metrics.flipped(u32::MAX, Duration::from_micros(1_000_000));
        metrics.submitted();
        metrics.flipped(1, Duration::from_micros(1_033_334));
        metrics.submitted(); // Still not a confirmed presentation.
        assert_eq!(metrics.page_flip_submitted, 3);
        assert_eq!(metrics.confirmed, 2);
        assert_eq!(metrics.refresh_slots_without_update, 1);
        assert_eq!(metrics.backwards_sequences, 0);
        assert_eq!(metrics.interval_ms, vec![33.334]);
    }
    #[test]
    fn timestamp_errors_and_sample_limit_are_explicit() {
        let mut metrics = PresentationMetrics::new(None, 0);
        metrics.flipped(10, Duration::from_millis(100));
        metrics.flipped(9, Duration::from_millis(90));
        assert_eq!(metrics.invalid_timestamps, 1);
        assert_eq!(metrics.backwards_sequences, 1);
        assert!(metrics.interval_ms.is_empty());
        for _ in 0..LIMIT + 3 {
            metrics.waited(Duration::ZERO);
        }
        assert_eq!(metrics.wait_ms.len(), LIMIT);
        assert_eq!(metrics.dropped_samples, 3);
    }
    #[test]
    fn optional_trace_preserves_kernel_stamps_and_is_bounded() {
        let mut disabled = PresentationMetrics::with_trace(Some(true), 60, false);
        disabled.flipped(9, Duration::from_micros(123456));
        assert!(disabled.trace_line().is_none());
        let mut enabled = PresentationMetrics::with_trace(Some(true), 60, true);
        for sequence in 0..(LIMIT + 3) as u32 {
            enabled.flipped(
                sequence,
                Duration::from_micros(1_000_000 + u64::from(sequence) * 16667),
            );
        }
        let trace = enabled.trace.as_ref().unwrap();
        assert_eq!(trace.len(), LIMIT);
        assert_eq!(
            trace[0],
            FlipStamp {
                sequence: 0,
                kernel_us: 1_000_000
            }
        );
        assert_eq!(trace[LIMIT - 1].sequence, (LIMIT - 1) as u32);
        assert_eq!(enabled.trace_dropped, 3);
        let line = enabled.trace_line().unwrap();
        assert!(line.starts_with("KMS_DRM_TRACE format=sequence:kernel_us retained=16384 dropped=3 stamps=[0:1000000,1:1016667,"));
        assert!(line.ends_with(']'));
        assert!(!enabled.finished);
    }
}
