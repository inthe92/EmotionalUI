//! An in-memory view over the existing catalog. No filesystem, SQL or launch logic.
use emotional_core::library::Game;
use std::collections::{BTreeMap, HashMap};

pub type Systems = BTreeMap<String, String>;
#[derive(Clone, Debug, PartialEq)]
pub enum Row { System(usize), Game(usize) }
#[derive(Default)]
pub struct Group { pub key: String, pub label: String, pub icon: i32, pub games: Vec<usize> }
#[derive(Default)]
pub struct Browser {
    pub groups: Vec<Group>,
    pub rows: Vec<Row>,
    folded: Vec<String>,
    matches: Vec<Vec<usize>>,
    pub expanded: Option<usize>,
    pub cursor: usize,
    pub toolbar: bool,
    pub editing: bool,
    pub query: String,
    pub key: usize,
    pub revision: u64,
    remembered: HashMap<String, usize>,
}
pub const KEYS: [&str; 40] = [
    "a","b","c","d","e","f","g","h","i","j",
    "k","l","m","n","o","p","q","r","s","t",
    "u","v","w","x","y","z","0","1","2","3",
    "4","5","6","7","8","9","Espacio","Borrar","Limpiar","Listo",
];
fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).map(|c| match c {
        'á'|'à'|'ä'|'â' => 'a', 'é'|'è'|'ë'|'ê' => 'e',
        'í'|'ì'|'ï'|'î' => 'i', 'ó'|'ò'|'ö'|'ô' => 'o',
        'ú'|'ù'|'ü'|'û' => 'u', _ => c,
    }).collect()
}
fn presentation(key: &str) -> (String, i32) {
    let (name, icon) = match key {
        "snes"|"sfc" => ("SNES",0), "nes"|"famicom" => ("NES",0),
        "gba" => ("Game Boy Advance",1), "gb" => ("Game Boy",1),
        "gbc" => ("Game Boy Color",1), "nds" => ("Nintendo DS",1),
        "psx"|"ps1"|"ps" => ("PlayStation",2), "psp" => ("PSP",1),
        "n64" => ("Nintendo 64",0), "megadrive"|"genesis" => ("Mega Drive",0),
        "mastersystem" => ("Master System",0), "dreamcast" => ("Dreamcast",2),
        "gamegear" => ("Game Gear",1), "arcade"|"mame"|"mame2003"|"fbneo" => ("Arcade",0),
        "pcengine" => ("PC Engine",0), "neogeo" => ("Neo Geo",0),
        "unknown"|"" => ("Colección",0), _ => (key,0),
    };
    (name.to_owned(),icon)
}
impl Browser {
    pub fn new(games: &[Game], systems: &Systems) -> Self {
        let mut this = Self::default();
        let mut grouped: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        this.folded = games.iter().map(|g| fold(&g.title)).collect();
        for (i, game) in games.iter().enumerate().filter(|(_,g)|g.available) {
            let key = systems.get(&game.id).map(String::as_str).unwrap_or("unknown").to_lowercase();
            grouped.entry(key).or_default().push(i);
        }
        this.groups = grouped.into_iter().map(|(key,mut indices)| {
            indices.sort_by(|a,b| this.folded[*a].cmp(&this.folded[*b]).then_with(||games[*a].id.cmp(&games[*b].id)));
            let (label,icon)=presentation(&key);
            Group{key,label,icon,games:indices}
        }).collect();
        this.groups.sort_by(|a,b|a.label.cmp(&b.label).then_with(||a.key.cmp(&b.key)));
        this.filter();
        this
    }
    pub fn replace(&mut self, games: &[Game], systems: &Systems) {
        let expanded=self.expanded.and_then(|g|self.groups.get(g)).map(|g|g.key.clone());
        let query=self.query.clone(); let editing=self.editing; let toolbar=self.toolbar; let revision=self.revision;
        *self=Self::new(games,systems);
        self.expanded=expanded.and_then(|key|self.groups.iter().position(|g|g.key==key));
        self.query=query;self.editing=editing;self.toolbar=toolbar;self.revision=revision;
        self.filter();
    }
    fn filter(&mut self) {
        let query=fold(self.query.trim());
        self.matches=self.groups.iter().map(|g|g.games.iter().copied().filter(|i|self.folded[*i].contains(&query)).collect()).collect();
        self.rebuild_rows();
    }
    fn rebuild_rows(&mut self) {
        self.rows.clear();
        for (group,games) in self.matches.iter().enumerate().filter(|(_,g)|!g.is_empty()) {
            self.rows.push(Row::System(group));
            if self.expanded==Some(group) { self.rows.extend(games.iter().map(|i|Row::Game(*i))); }
        }
        self.cursor=self.cursor.min(self.rows.len().saturating_sub(1));
        self.revision=self.revision.wrapping_add(1);
    }
    pub fn set_query(&mut self, text: &str) {
        let text:String=text.chars().filter(|c|!c.is_control()).take(80).collect();
        if self.query!=text {self.query=text;self.cursor=0;self.filter();}
    }
    pub fn result_count(&self) -> usize {self.matches.iter().map(Vec::len).sum()}
    pub fn count(&self, group: usize) -> usize {self.matches.get(group).map_or(0,Vec::len)}
    pub fn selected_game(&self) -> Option<usize> {
        if self.toolbar || self.editing {return None;}
        match self.rows.get(self.cursor) {Some(Row::Game(i))=>Some(*i),_=>None}
    }
    pub fn select_game(&mut self,index:usize) -> bool {
        let Some(group)=self.matches.iter().position(|g|g.contains(&index)) else {return false;};
        self.expanded=Some(group);self.rebuild_rows();
        self.cursor=self.rows.iter().position(|r|*r==Row::Game(index)).unwrap_or(0);
        self.toolbar=false;true
    }
    pub fn toggle(&mut self, row: usize) -> Option<usize> {
        self.cursor=row.min(self.rows.len().saturating_sub(1));self.toolbar=false;
        match self.rows.get(self.cursor).cloned() {
            Some(Row::Game(i))=>Some(i),
            Some(Row::System(group))=>{
                self.expanded=if self.expanded==Some(group){None}else{Some(group)};
                self.rebuild_rows();
                self.cursor=self.rows.iter().position(|r|*r==Row::System(group)).unwrap_or(0);
                None
            },None=>None,
        }
    }
    pub fn collapse(&mut self) -> bool {
        let Some(group)=self.expanded.take() else{return false;};
        if let Some(Row::Game(i))=self.rows.get(self.cursor) {self.remembered.insert(self.groups[group].key.clone(),*i);}
        self.rebuild_rows();self.cursor=self.rows.iter().position(|r|*r==Row::System(group)).unwrap_or(0);true
    }
    pub fn down(&mut self) {
        if self.toolbar {self.toolbar=false;return;}
        if let Some(Row::System(g))=self.rows.get(self.cursor) {
            if self.expanded==Some(*g) {
                if let Some(index)=self.remembered.get(&self.groups[*g].key).copied() {
                    if let Some(row)=self.rows.iter().position(|r|*r==Row::Game(index)){self.cursor=row;return;}
                }
            }
        }
        self.cursor=(self.cursor+1).min(self.rows.len().saturating_sub(1));
    }
    pub fn keyboard_choose(&mut self, key: usize) {
        match key {
            36=>{let value=format!("{} ",self.query);self.set_query(&value);},
            37=>{let mut value=self.query.clone();value.pop();self.set_query(&value);},
            38=>self.set_query(""),39=>{self.editing=false;self.toolbar=false;},
            i if i<36=>{let value=format!("{}{}",self.query,KEYS[i]);self.set_query(&value);},_=>{}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Vec<Game>,Systems) {
        let mut games=emotional_core::library::demo_catalog();
        for (i,g) in games.iter_mut().enumerate(){g.id=format!("game-{i}");}
        games[0].title="ZÉLDA: El vínculo".into();games[1].title="Otro camino".into();games[2].title="Zelda portátil".into();
        let systems=games.iter().enumerate().map(|(i,g)|(g.id.clone(),if i<2{"snes"}else{"gba"}.into())).collect();
        (games,systems)
    }
    #[test] fn only_available_games_form_real_groups() {
        let (mut games,systems)=fixture();games[0].available=false;
        let b=Browser::new(&games,&systems);
        assert_eq!(b.groups.len(),2);assert_eq!(b.result_count(),5);
        assert!(!b.groups.iter().any(|g|g.games.contains(&0)));
        assert_eq!(b.rows.len(),2);assert_eq!(b.selected_game(),None);
    }
    #[test] fn expanding_another_system_closes_the_first() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);
        b.toggle(0);assert_eq!(b.expanded,Some(0));
        let other=b.rows.iter().position(|r|*r==Row::System(1)).unwrap();b.toggle(other);
        assert_eq!(b.expanded,Some(1));assert!(!b.rows.contains(&Row::Game(2)));
        b.collapse();assert_eq!(b.rows.len(),2);assert_eq!(b.cursor,1);
    }
    #[test] fn search_ignores_case_and_accents_across_systems() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);b.set_query("zELdA");
        assert_eq!(b.result_count(),2);assert_eq!(b.rows.len(),2);
        b.toggle(1);assert_eq!(b.count(1),1);assert!(b.rows.contains(&Row::Game(0)));
    }
    #[test] fn filtered_rows_resolve_the_original_game_not_the_row_number() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);b.set_query("portátil");
        b.toggle(0);b.down();assert_eq!(b.selected_game(),Some(2));
        assert_eq!(b.toggle(b.cursor),Some(2));
    }
    #[test] fn collapse_and_reopen_remember_the_last_game() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);
        assert!(b.select_game(0));assert_eq!(b.selected_game(),Some(0));
        assert!(b.collapse());assert_eq!(b.selected_game(),None);
        b.toggle(b.cursor);b.down();assert_eq!(b.selected_game(),Some(0));
    }
    #[test] fn empty_search_is_safe_and_typing_is_bounded() {
        let mut b=Browser::new(&[],&Systems::new());b.down();b.toggle(0);assert!(!b.collapse());
        b.set_query(&"á".repeat(100));assert_eq!(b.query.chars().count(),80);assert_eq!(b.result_count(),0);
    }
    #[test] fn keyboard_backspace_removes_one_unicode_character() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);b.editing=true;
        b.set_query("áñ");b.keyboard_choose(37);assert_eq!(b.query,"á");
        b.keyboard_choose(38);assert_eq!(b.query,"");b.keyboard_choose(25);assert_eq!(b.query,"z");
        b.keyboard_choose(39);assert!(!b.editing);
    }
    #[test] fn import_refresh_preserves_query_and_expanded_system() {
        let (games,s)=fixture();let mut b=Browser::new(&games,&s);b.set_query("zelda");b.toggle(1);
        let rev=b.revision;b.replace(&games,&s);assert_eq!(b.query,"zelda");assert_eq!(b.expanded,Some(1));assert!(b.revision>rev);
    }
    #[test] fn large_catalog_is_filtered_from_the_existing_memory_index() {
        let prototype=emotional_core::library::demo_catalog().remove(0);
        let games:Vec<_>=(0..20000).map(|i|{let mut g=prototype.clone();g.id=format!("id-{i}");g.title=format!("Camino {i:05}");g}).collect();
        let mut b=Browser::new(&games,&Systems::new());assert_eq!(b.result_count(),20000);
        b.set_query("1234");assert_eq!(b.result_count(),12);assert_eq!(b.rows.len(),1);
        let revision=b.revision;b.set_query("1234");assert_eq!(b.revision,revision);
    }
}
