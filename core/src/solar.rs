//! NOAA fractional-year approximation; no network or guessed local offset.
//! https://gml.noaa.gov/grad/solcalc/solareqns.PDF
//! Civil twilight uses -6 degrees; sunrise uses -0.833 degrees.
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}
impl Location {
    pub fn valid(self) -> bool {
        self.latitude.is_finite()
            && self.longitude.is_finite()
            && (-90. ..=90.).contains(&self.latitude)
            && (-180. ..=180.).contains(&self.longitude)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Daylight {
    Normal,
    PolarDay,
    PolarNight,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarDay {
    pub dawn: Option<NaiveDateTime>,
    pub rise: Option<NaiveDateTime>,
    pub noon: NaiveDateTime,
    pub set: Option<NaiveDateTime>,
    pub dusk: Option<NaiveDateTime>,
    pub daylight: Daylight,
}
fn terms(date: NaiveDate, hour: f64) -> (f64, f64) {
    let days = if date.leap_year() { 366. } else { 365. };
    let g = 2. * PI / days * (f64::from(date.ordinal()) - 1. + (hour - 12.) / 24.);
    let eq = 229.18
        * (0.000075 + 0.001868 * g.cos()
            - 0.032077 * g.sin()
            - 0.014615 * (2. * g).cos()
            - 0.040849 * (2. * g).sin());
    let decl = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2. * g).cos()
        + 0.000907 * (2. * g).sin()
        - 0.002697 * (3. * g).cos()
        + 0.00148 * (3. * g).sin();
    (eq, decl)
}
pub fn day(date: NaiveDate, location: Location) -> Option<SolarDay> {
    if !location.valid() {
        return None;
    }
    let midnight = date.and_hms_opt(0, 0, 0)?;
    // Refine solar noon once. UTC values deliberately retain date rollover.
    let (eq, _) = terms(date, 12.);
    let (eq, decl) = terms(date, (720. - 4. * location.longitude - eq) / 60.);
    let noon_minutes = 720. - 4. * location.longitude - eq;
    let at = |minutes: f64| {
        midnight.checked_add_signed(Duration::seconds((minutes * 60.).round() as i64))
    };
    let lat = location.latitude.to_radians();
    let horizon = |zenith: f64| {
        // Protect poles without inventing rise/set times.
        let numerator = zenith.to_radians().cos() - lat.sin() * decl.sin();
        let denominator = lat.cos() * decl.cos();
        if denominator.abs() < 1e-12 {
            return if numerator < 0. { -2. } else { 2. };
        }
        numerator / denominator
    };
    let events = |zenith: f64| {
        let h = horizon(zenith);
        if !(-1. ..=1.).contains(&h) {
            return (None, None);
        }
        let delta = h.acos().to_degrees() * 4.;
        (at(noon_minutes - delta), at(noon_minutes + delta))
    };
    let h = horizon(90.833);
    let (rise, set) = events(90.833);
    let (dawn, dusk) = events(96.);
    Some(SolarDay {
        dawn,
        rise,
        noon: at(noon_minutes)?,
        set,
        dusk,
        daylight: if h < -1. {
            Daylight::PolarDay
        } else if h > 1. {
            Daylight::PolarNight
        } else {
            Daylight::Normal
        },
    })
}
impl SolarDay {
    /// Solar art bands. Fractions are artistic policy, not measured sunrise times.
    pub fn moment(self, utc: NaiveDateTime, local: NaiveDateTime) -> crate::environment::DayMoment {
        use crate::environment::DayMoment as M;
        if self.daylight == Daylight::PolarDay {
            return if utc < self.noon {
                M::MidMorning
            } else {
                M::EarlyAfternoon
            };
        }
        if let (Some(rise), Some(set)) = (self.rise, self.set) {
            if (self.dawn.is_none() && utc < rise) || (self.dusk.is_none() && utc >= set) {
                return M::Dusk; // Continuous civil light, not a fabricated dark night.
            }
            let dawn = self.dawn.unwrap_or(rise);
            let dusk = self.dusk.unwrap_or(set);
            let morning = rise + (self.noon - rise) * 35 / 100;
            let afternoon = self.noon + (set - self.noon) * 55 / 100;
            if utc < dawn {
                M::LateNight
            } else if utc < morning {
                M::EarlyMorning
            } else if utc < self.noon {
                M::MidMorning
            } else if utc < afternoon {
                M::EarlyAfternoon
            } else if utc < set {
                M::LateAfternoon
            } else if utc < dusk {
                M::Dusk
            } else {
                M::Night
            }
        } else if self
            .dawn
            .zip(self.dusk)
            .is_some_and(|(a, b)| utc >= a && utc < b)
        {
            M::Dusk // Real polar civil twilight, not a fabricated sunrise.
        } else if local.hour() < 6 {
            M::LateNight
        } else {
            M::Night
        }
    }
}
