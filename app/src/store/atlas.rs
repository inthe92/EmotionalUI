use super::*;
use emotional_core::reserve::Reserve;
use std::error::Error;
impl Store {
    pub fn atlas_links(&self) -> rusqlite::Result<std::collections::BTreeMap<String, String>> {
        self.0
            .prepare("SELECT game_id,atlas_id FROM atlas_links")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect()
    }
    pub fn atlas_link(&self, id: &str, atlas_id: &str) -> Result<(), Box<dyn Error>> {
        if self.launch_rom(id)?.is_none() {
            return Err("Link requires an imported local game ID".into());
        }
        self.0.execute("INSERT INTO atlas_links VALUES(?1,?2,'explicit-user-link') ON CONFLICT(game_id) DO UPDATE SET atlas_id=excluded.atlas_id",[id,atlas_id])?;
        Ok(())
    }
    pub fn reserve(&self) -> Result<Option<Reserve>, Box<dyn Error>> {
        use rusqlite::OptionalExtension;
        let text: Option<String> = self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='discovery_reserve'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(text
            .filter(|s| s.len() <= 1_000_000)
            .and_then(|s| toml::from_str(&s).ok()))
    }
    pub fn save_reserve(&self, r: &Reserve) -> Result<(), Box<dyn Error>> {
        self.0.execute("INSERT INTO settings VALUES('discovery_reserve',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[toml::to_string(r)?])?;
        Ok(())
    }
}
