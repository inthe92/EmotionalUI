//! Bind the catalog view to Slint; actions retain the existing launcher/supervisor.
use crate::{library_browser::Row, state::State, Home, LibraryEntry};
use emotional_core::controls::Action;
impl State {
    pub(super) fn paint_library(&self, ui: &Home) {
        if self.browser_painted.get() != self.browser.revision {
            let rows:Vec<LibraryEntry>=self.browser.rows.iter().map(|row|match row {
                Row::System(g)=>LibraryEntry {title:self.browser.groups[*g].label.as_str().into(),detail:format!("{} juegos",self.browser.count(*g)).into(),header:true,expanded:self.browser.expanded==Some(*g),icon:self.browser.groups[*g].icon},
                Row::Game(i)=>LibraryEntry {title:self.games[*i].title.as_str().into(),detail:"".into(),header:false,expanded:false,icon:0},
            }).collect();
            ui.set_library_rows(slint::ModelRc::new(slint::VecModel::from(rows)));
            self.browser_painted.set(self.browser.revision);
            ui.set_library_revision(self.browser.revision as i32);
        }
        ui.set_library_cursor(self.browser.cursor as i32);
        ui.set_library_toolbar(self.browser.toolbar);
        ui.set_library_editing(self.browser.editing);
        ui.set_library_query(self.browser.query.as_str().into());
        ui.set_library_results(self.browser.result_count() as i32);
        ui.set_library_keyboard_index(self.browser.key as i32);
    }
    pub(super) fn library_text(&mut self, ui:&Home, key:&str)->bool {
        if self.layout!=4 || !self.library || !self.browser.editing || self.modal_active() {return false;}
        if key==char::from(slint::platform::Key::Return).to_string() || key==char::from(slint::platform::Key::Escape).to_string() {
            self.browser.editing=false;self.browser.toolbar=false;
        } else if key==char::from(slint::platform::Key::Backspace).to_string() {
            self.browser.keyboard_choose(37);
        } else if !key.is_empty() && key.chars().all(|c|!c.is_control()&&!('\u{e000}'..='\u{f8ff}').contains(&c)) {
            self.browser.set_query(&format!("{}{}",self.browser.query,key));
        } else {return false;}
        self.paint(ui);true
    }
    pub(super) fn library_action(&mut self,ui:&Home,action:Action)->bool {
        if self.layout!=4 || !self.library || self.history_panel || self.dock_focus.is_some() || self.system_focus {return false;}
        if self.browser.editing {
            match action {
                Action::Up=>self.browser.key=self.browser.key.saturating_sub(10),
                Action::Down=>self.browser.key=(self.browser.key+10).min(39),
                Action::Left=>self.browser.key=self.browser.key.saturating_sub(1),
                Action::Right=>self.browser.key=(self.browser.key+1).min(39),
                Action::Confirm=>self.browser.keyboard_choose(self.browser.key),
                Action::Back=>{self.browser.editing=false;self.browser.toolbar=false;},
                _=>return false,
            }
        } else if self.browser.toolbar {
            match action {
                Action::Confirm=>{self.browser.editing=true;self.browser.key=0;},
                Action::Down=>self.browser.down(),
                Action::Back if !self.browser.query.is_empty()=>self.browser.set_query(""),
                _=>return false,
            }
        } else {
            match action {
                Action::Up if self.browser.cursor==0=>self.browser.toolbar=true,
                Action::Up=>self.browser.cursor-=1,
                Action::Down=>self.browser.down(),
                Action::Left=>{self.browser.collapse();},
                Action::Right=>{
                    if matches!(self.browser.rows.get(self.browser.cursor),Some(Row::System(g)) if self.browser.expanded!=Some(*g)) {self.browser.toggle(self.browser.cursor);} else {self.browser.down();}
                },
                Action::Confirm=>{let row=self.browser.cursor;let _=self.activate(ui,10+row);},
                Action::Back if self.browser.expanded.is_some()=>{self.browser.collapse();},
                Action::Back if !self.browser.query.is_empty()=>self.browser.set_query(""),
                _=>return false,
            }
        }
        if let Some(index)=self.browser.selected_game(){self.library_index=index;}
        self.paint(ui);true
    }
}
