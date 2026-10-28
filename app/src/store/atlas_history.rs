use super::Store;
use emotional_core::atlas::{Habits, Observation};
impl Store {
    pub fn atlas_observations(&self) -> rusqlite::Result<Vec<Observation>> {
        self.0.prepare("SELECT game_id,started_utc,local_hour,weekend,seconds,recommended FROM atlas_activity ORDER BY started_utc DESC LIMIT 256")?
            .query_map([], |r| Ok(Observation {game_id:r.get(0)?,started_utc:r.get(1)?,local_hour:r.get(2)?,weekend:r.get(3)?,seconds:r.get(4)?,recommended:r.get(5)?}))?
            .collect()
    }
    pub fn atlas_habits(&self, games:&[emotional_core::library::Game], now:Option<(i64,u8,bool)>) -> rusqlite::Result<Habits> {
        Ok(Habits::learn(games,&self.atlas_observations()?,now))
    }
}
