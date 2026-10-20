use chrono::{NaiveDateTime, Timelike};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Dawn,
    Horizon,
    Shelter,
    Silence,
}
impl Mood {
    pub fn at(time: NaiveDateTime) -> Self {
        match time.hour() {
            6..=11 => Self::Dawn,
            12..=18 => Self::Horizon,
            19..=23 => Self::Shelter,
            _ => Self::Silence,
        }
    }
    pub fn index(self) -> i32 {
        match self {
            Self::Dawn => 0,
            Self::Horizon => 1,
            Self::Shelter => 2,
            Self::Silence => 3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Dawn => "Despertar",
            Self::Horizon => "Horizonte",
            Self::Shelter => "Refugio",
            Self::Silence => "Silencio",
        }
    }
    pub fn greeting(self) -> &'static str {
        match self {
            Self::Dawn => "Un nuevo día, una aventura.",
            Self::Horizon => "Un pequeño viaje te espera.",
            Self::Shelter => "Ya estás en tu refugio.",
            Self::Silence => "El mundo puede esperar.",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_time_boundaries() {
        for (h, m) in [
            (0, Mood::Silence),
            (5, Mood::Silence),
            (6, Mood::Dawn),
            (11, Mood::Dawn),
            (12, Mood::Horizon),
            (18, Mood::Horizon),
            (19, Mood::Shelter),
            (23, Mood::Shelter),
        ] {
            let d = chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
                .unwrap()
                .and_hms_opt(h, 0, 0)
                .unwrap();
            assert_eq!(Mood::at(d), m);
        }
    }
}
