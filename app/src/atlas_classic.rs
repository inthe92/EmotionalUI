//! Small prepared catalog, shared by the normal and progressive library paths.
use emotional_core::{atlas::Catalog, library::Game};
use std::sync::OnceLock;
pub const PREPARED: &str = include_str!("../../assets/atlas/classic-v1.toml");
static CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();
pub fn catalog() -> Result<&'static Catalog, String> {
    CATALOG.get_or_init(|| {
        // Optional prepared replacement. Read once, never during navigation.
        let custom = std::env::var_os("EMOTIONAL_ATLAS_CATALOG").map(std::path::PathBuf::from);
        let text = if let Some(path) = custom {
            if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>2*1024*1024 {
                return Err("Classic Atlas exceeds 2 MiB".into());
            }
            std::fs::read_to_string(path).map_err(|e|e.to_string())?
        } else { PREPARED.to_owned() };
        let c: Catalog=toml::from_str(&text).map_err(|e|e.to_string())?;
        c.validate()?;
        Ok(c)
    }).as_ref().map_err(Clone::clone)
}
pub fn apply(games: &mut [Game]) -> Result<usize,String> {
    // The prepared catalog is a fallback. An explicit SQLite match has already
    // selected a version and must not acquire attributes from a different card.
    let catalog = catalog()?;
    let mut unlinked: Vec<_> = games.iter().filter(|g|g.editorial.knowledge.id.is_empty()).cloned().collect();
    let count = catalog.apply(&mut unlinked)?;
    for (game, prepared) in games.iter_mut().filter(|g|g.editorial.knowledge.id.is_empty()).zip(unlinked) {
        game.editorial = prepared.editorial;
        game.genre = prepared.genre;
    }
    Ok(count)
}

/// Imported libraries remain complete in Biblioteca. Only confidently linked
/// identities may enter recommendations; fictional development fixtures retain
/// their existing behavior. Never turn an unknown ROM into a catalog identity.
pub fn eligible_order(order: Vec<usize>, games: &[Game], imported: bool) -> Vec<usize> {
    order.into_iter().filter(|&i| games.get(i).is_some_and(|g|
        g.available && (!imported || !g.editorial.knowledge.id.is_empty())
    )).collect()
}
pub fn now(clock: &emotional_services::clock::Clock) -> Option<(i64,u8,bool)> {
    let local=clock.known_now()?;
    let utc=clock.injected_utc().map(|v|v.timestamp()).or_else(||local.and_local_timezone(chrono::Local).single().map(|v|v.timestamp()));
    emotional_core::atlas::local_context(Some(local),utc)
}
pub fn raining(clock:&emotional_services::clock::Clock, ambient:&emotional_services::ambient::Runtime, fallback:emotional_services::weather::Weather)->bool {
    use emotional_core::ambient::WeatherKind;
    let c=ambient.capture(clock,Default::default(),true);
    matches!(c.weather.kind(),WeatherKind::Rain|WeatherKind::Storm)
        || (clock.known_now().is_some() && fallback==emotional_services::weather::Weather::Rain)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_catalog_matches_only_metadata_title_and_system() {
        let c:Catalog=toml::from_str(PREPARED).unwrap(); c.validate().unwrap();
        assert!(!c.games.is_empty());
        assert!(c.games.iter().all(|g| !g.profile.knowledge.id.is_empty()));
        let mut g=emotional_core::library::demo_catalog();
        for game in &mut g {game.system="snes".into();game.editorial=Default::default();}
        g[0].title="Super Metroid".into(); g[0].editorial.calm=Some(93);
        g[1].title=g[0].title.clone();g[1].system="gba".into();
        assert_eq!(c.apply(&mut g).unwrap(),1);
        assert_eq!(g[0].editorial.calm,Some(93));
        assert_eq!(g[0].editorial.knowledge.id,"atlas:snes:000005");
        assert!(g[1].editorial.knowledge.id.is_empty());
        assert!(g.iter().all(|g|g.editorial.session_minutes.is_none()));
    }
    #[test]
    fn ambiguous_metadata_remains_unknown() {
        let mut c:Catalog=toml::from_str(PREPARED).unwrap();
        let mut other=c.games[0].clone();other.id="atlas:qa:ambiguous".into();
        other.profile.knowledge.id=other.id.clone();c.games.push(other);
        let mut g=emotional_core::library::demo_catalog();
        for game in &mut g {game.editorial=Default::default();game.system="unknown".into();}
        g[0].title=c.games[0].title.clone();g[0].system=c.games[0].system.clone();
        assert_eq!(c.apply(&mut g).unwrap(),0);
        assert!(g[0].editorial.knowledge.id.is_empty());
    }
    #[test]
    fn imported_unknowns_stay_in_library_but_never_enter_recommendations() {
        let mut games=emotional_core::library::demo_catalog();
        games.truncate(3);
        for game in &mut games {game.editorial=Default::default();}
        games[1].editorial.knowledge.id="atlas:linked".into();
        games[2].editorial.knowledge.id="atlas:missing-rom".into();
        games[2].available=false;
        assert_eq!(eligible_order(vec![0,1,2,99],&games,true),vec![1]);
        assert_eq!(eligible_order(vec![0,1,2],&games,false),vec![0,1]);
        assert_eq!(games.len(),3);
        assert!(games[0].editorial.knowledge.id.is_empty());
    }
    #[test]
    fn fallback_does_not_mix_attributes_into_an_existing_version_link() {
        let mut games=emotional_core::library::demo_catalog();
        games.truncate(1);
        games[0].title="Super Metroid".into();games[0].system="snes".into();
        games[0].editorial=Default::default();
        games[0].editorial.knowledge.id="explicit:version".into();
        assert_eq!(apply(&mut games).unwrap(),0);
        assert_eq!(games[0].editorial.knowledge.id,"explicit:version");
        assert!(games[0].editorial.exploration.is_none());
    }
}
