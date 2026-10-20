use chrono::{DateTime, Datelike, Local, NaiveDateTime};
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
        }
    }
    pub fn now(&self) -> NaiveDateTime {
        self.simulated.unwrap_or_else(|| Local::now().naive_local())
    }
    pub fn unknown(&mut self) {
        self.trust = Trust::Unknown;
    }
    pub fn known_now(&self) -> Option<NaiveDateTime> {
        let now = self.now();
        if self.trust == Trust::Unknown
            || (self.trust != Trust::Simulated && !(2020..=2100).contains(&now.year()))
        {
            None
        } else {
            Some(now)
        }
    }
    pub fn timezone(&self) -> String {
        let now: DateTime<Local> = Local::now();
        now.format("%Z %:z").to_string()
    }
    pub fn set(&mut self, value: NaiveDateTime) {
        self.simulated = Some(value);
        self.trust = Trust::Simulated;
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
