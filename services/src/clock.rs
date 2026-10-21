use chrono::{DateTime, Local, NaiveDateTime};
use emotional_core::environment::{resolve_time, TimeContext, TimeSource};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    Unknown,
    SystemUnverified,
    Simulated,
    UserConfirmed,
}
#[derive(Clone)]
pub struct Clock {
    simulated: Option<NaiveDateTime>,
    simulated_utc: Option<DateTime<chrono::Utc>>,
    last_known: Option<NaiveDateTime>,
    synchronized: Option<(NaiveDateTime, std::time::Instant)>,
    pub trust: Trust,
}
impl Clock {
    pub fn new(simulated: Option<NaiveDateTime>) -> Self {
        Self {
            trust: if simulated.is_some() {
                Trust::Simulated
            } else {
                Trust::SystemUnverified
            },
            simulated,
            simulated_utc: None,
            last_known: None,
            synchronized: None,
        }
    }
    pub fn now(&self) -> NaiveDateTime {
        self.simulated.unwrap_or_else(|| Local::now().naive_local())
    }
    pub fn unknown(&mut self) {
        self.trust = Trust::Unknown;
    }
    pub fn known_now(&self) -> Option<NaiveDateTime> {
        let context = self.context();
        context.local.filter(|_| context.reliable)
    }
    pub fn restore_last_known(&mut self, value: Option<NaiveDateTime>) {
        self.last_known = value;
    }
    /// Adapter seam for an ALREADY synchronized sample; never fetches the network.
    pub fn supply_synchronized(&mut self, value: NaiveDateTime) {
        self.synchronized = Some((value, std::time::Instant::now()));
    }
    pub fn context(&self) -> TimeContext {
        if self.trust == Trust::Unknown {
            return resolve_time(None, None, None);
        }
        if self.trust == Trust::Simulated {
            return TimeContext {
                local: Some(self.now()),
                source: TimeSource::Simulated,
                reliable: true,
            };
        }
        let synchronized = self.synchronized.and_then(|(sample, received)| {
            chrono::Duration::from_std(received.elapsed())
                .ok()
                .and_then(|elapsed| sample.checked_add_signed(elapsed))
        });
        resolve_time(Some(self.now()), synchronized, self.last_known)
    }
    /// Only current real clock sources may update the historical fallback.
    pub fn persistable_now(&self) -> Option<NaiveDateTime> {
        let context = self.context();
        context.local.filter(|_| {
            context.reliable
                && matches!(
                    context.source,
                    TimeSource::System | TimeSource::NetworkSynchronized
                )
        })
    }
    pub fn timezone(&self) -> String {
        let now: DateTime<Local> = Local::now();
        now.format("%Z %:z").to_string()
    }
    pub fn set(&mut self, value: NaiveDateTime) {
        self.simulated = Some(value);
        self.simulated_utc = None;
        self.trust = Trust::Simulated;
    }
    /// Explicit offset resolves repeated DST hours without changing the OS clock.
    pub fn set_zoned(&mut self, value: DateTime<chrono::FixedOffset>, zone: chrono_tz::Tz) {
        self.simulated = Some(value.with_timezone(&zone).naive_local());
        self.simulated_utc = Some(value.with_timezone(&chrono::Utc));
        self.trust = Trust::Simulated;
    }
    pub fn injected_utc(&self) -> Option<DateTime<chrono::Utc>> {
        (self.trust == Trust::Simulated)
            .then_some(self.simulated_utc)
            .flatten()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simulation_is_frozen() {
        let d = NaiveDateTime::parse_from_str("2026-10-31T23:59", "%Y-%m-%dT%H:%M").unwrap();
        let c = Clock::new(Some(d));
        assert_eq!(c.now(), d);
        assert_eq!(c.trust, Trust::Simulated);
    }
    #[test]
    fn system_is_not_claimed_synced() {
        assert_eq!(Clock::new(None).trust, Trust::SystemUnverified);
    }
    #[test]
    fn invalid_date_rejected() {
        assert!(NaiveDateTime::parse_from_str("2026-02-30T12:00", "%Y-%m-%dT%H:%M").is_err());
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;
    #[test]
    fn unknown_is_not_used_for_context() {
        let mut c = Clock::new(None);
        c.unknown();
        assert!(c.known_now().is_none());
    }
    #[test]
    fn implausible_system_date_is_not_trusted() {
        let d = NaiveDateTime::parse_from_str("1970-01-01T00:00", "%Y-%m-%dT%H:%M").unwrap();
        let mut clock = Clock::new(Some(d));
        clock.trust = Trust::SystemUnverified;
        assert!(clock.known_now().is_none());
    }
    #[test]
    fn midnight_changes_mood_and_date() {
        let d = NaiveDateTime::parse_from_str("2026-12-31T23:59", "%Y-%m-%dT%H:%M").unwrap();
        let next = d + chrono::Duration::minutes(1);
        assert_ne!(d.date(), next.date());
        assert_eq!(
            emotional_core::mood::Mood::at(next),
            emotional_core::mood::Mood::Silence
        );
    }
}

#[cfg(test)]
mod simulation_tests {
    use super::*;
    #[test]
    fn historical_simulation_is_allowed() {
        let d = NaiveDateTime::parse_from_str("1989-04-21T08:00", "%Y-%m-%dT%H:%M").unwrap();
        assert_eq!(Clock::new(Some(d)).known_now(), Some(d));
    }
}

#[cfg(test)]
mod fallback_tests {
    use super::*;
    fn date(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M").unwrap()
    }
    fn invalid_system() -> Clock {
        let mut c = Clock::new(Some(date("1970-01-01T00:00")));
        c.trust = Trust::SystemUnverified;
        c
    }
    #[test]
    fn saved_time_is_not_an_invented_current_date() {
        let mut c = invalid_system();
        c.restore_last_known(Some(date("2026-10-20T08:00")));
        assert_eq!(c.context().source, TimeSource::LastKnown);
        assert!(c.context().local.is_some());
        assert_eq!(c.known_now(), None);
        assert_eq!(c.persistable_now(), None);
        c.unknown();
        assert_eq!(c.context().source, TimeSource::Unknown);
    }
    #[test]
    fn already_synchronized_sample_can_supply_invalid_system_time() {
        let mut c = invalid_system();
        c.supply_synchronized(date("2026-10-20T09:00"));
        assert_eq!(c.context().source, TimeSource::NetworkSynchronized);
        assert!(c.known_now().is_some());
    }
    #[test]
    fn simulation_never_overwrites_real_clock_cache() {
        assert_eq!(
            Clock::new(Some(date("2026-10-20T09:00"))).persistable_now(),
            None
        );
        let mut c = Clock::new(Some(date("2026-10-20T09:00")));
        c.trust = Trust::SystemUnverified;
        assert_eq!(c.context().source, TimeSource::System);
    }
}
