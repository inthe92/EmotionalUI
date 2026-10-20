use chrono::{DateTime, Duration, Utc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Cloud,
    Wind,
    Fog,
    Snow,
    Unknown,
}
impl Weather {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fog" => Ok(Self::Fog),
            "snow" => Ok(Self::Snow),
            "clear" => Ok(Self::Clear),
            "rain" => Ok(Self::Rain),
            "cloud" => Ok(Self::Cloud),
            "wind" => Ok(Self::Wind),
            "unknown" => Ok(Self::Unknown),
            _ => Err("clima válido: clear, rain, cloud, wind, fog, snow, unknown".into()),
        }
    }
}
pub struct Observation {
    pub condition: Weather,
    pub updated: DateTime<Utc>,
}
impl Observation {
    pub fn fresh(&self, now: DateTime<Utc>) -> Option<Weather> {
        let age = now - self.updated;
        if age >= Duration::zero() && age <= Duration::hours(3) {
            Some(self.condition)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_and_future_are_ignored() {
        let now = Utc::now();
        for delta in [-4, 1] {
            assert_eq!(
                Observation {
                    condition: Weather::Rain,
                    updated: now + Duration::hours(delta)
                }
                .fresh(now),
                None
            );
        }
    }
    #[test]
    fn cache_boundary() {
        let now = Utc::now();
        assert_eq!(
            Observation {
                condition: Weather::Rain,
                updated: now - Duration::hours(3)
            }
            .fresh(now),
            Some(Weather::Rain)
        );
    }
}
