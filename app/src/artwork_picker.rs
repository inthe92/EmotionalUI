//! Per-game, explicit SteamGridDB selection. Network/decoding/writes stay on workers.
use crate::{artwork, state::State, Home};
use emotional_core::controls::Action;
use serde::Deserialize;
use slint::{ModelRc, SharedString, VecModel};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize)]
struct Game {
    id: u64,
    name: String,
    release_date: Option<i64>,
}
#[derive(Clone, Deserialize)]
struct Author {
    name: String,
}
#[derive(Clone, Deserialize)]
struct Grid {
    id: u64,
    url: String,
    author: Option<Author>,
    style: Option<String>,
}
#[derive(Default, Deserialize)]
struct Reply {
    error: Option<String>,
    games: Option<Vec<Game>>,
    grids: Option<Vec<Grid>>,
    path: Option<PathBuf>,
    root: Option<PathBuf>,
}
enum Task {
    Search(String),
    Grids(u64),
    Preview(Grid),
    Apply(String, Option<artwork::Resource>),
}
enum Event {
    Games(Vec<Game>),
    Grids(Vec<Grid>),
    Preview(u64, artwork::Resource, artwork::Prepared),
    Saved,
    Error,
}
pub(super) struct Picker {
    game_id: String,
    title: String,
    system: String,
    query: String,
    index: usize,
    key: usize,
    games: Vec<Game>,
    grids: Vec<Grid>,
    in_grids: bool,
    keyboard: bool,
    status: String,
    receiver: Option<mpsc::Receiver<Event>>,
    preview: Option<(u64, artwork::Resource)>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Picker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn helper() -> Option<PathBuf> {
    #[cfg(any(debug_assertions, feature = "simulator"))]
    if let Some(path) = std::env::var_os("EMOTIONAL_SGDB_HELPER") {
        return Some(path.into());
    }
    Some(
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("steamgriddb_rpc.py"),
    )
}
fn rpc(request: serde_json::Value, cancel: &AtomicBool) -> Result<Reply, ()> {
    let path = helper().ok_or(())?;
    if !path.is_absolute() || !path.is_file() || path.is_symlink() {
        return Err(());
    }
    let mut child = Command::new("/usr/bin/python3")
        .arg("-B")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    if child
        .stdin
        .take()
        .ok_or(())?
        .write_all(request.to_string().as_bytes())
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(());
    }
    let pipe = child.stdout.take().ok_or(())?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(());
            }
        }
    };
    let output = reader.join().map_err(|_| ())?.map_err(|_| ())?;
    if output.len() > 4 * 1024 * 1024 {
        return Err(());
    }
    let result: Reply = serde_json::from_slice(&output).map_err(|_| ())?;
    if result.error.is_some() || !status.success() {
        return Err(());
    }
    Ok(result)
}
impl Picker {
    fn start(&mut self, task: Task) {
        if self.receiver.is_some() {
            return;
        }
        match &task {
            Task::Search(_) => {
                self.games.clear();
                self.grids.clear();
                self.in_grids = false;
                self.preview = None;
                self.index = 0;
            }
            Task::Grids(_) => {
                self.grids.clear();
                self.preview = None;
            }
            _ => {}
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.status = "Conectando con SteamGridDB…".into();
        let cancel = self.cancel.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<Event, ()> {
                if cancel.load(Ordering::Relaxed) {
                    return Err(());
                }
                Ok(match task {
                    Task::Search(query) => Event::Games(
                        rpc(
                            serde_json::json!({"operation":"search","query":query}),
                            &cancel,
                        )?
                        .games
                        .ok_or(())?,
                    ),
                    Task::Grids(id) => Event::Grids(
                        rpc(
                            serde_json::json!({"operation":"grids","game_id":id}),
                            &cancel,
                        )?
                        .grids
                        .ok_or(())?,
                    ),
                    Task::Preview(grid) => {
                        let reply = rpc(
                            serde_json::json!({"operation":"preview","url":grid.url,"grid_id":grid.id,"author":grid.author.map(|a|a.name)}),
                            &cancel,
                        )?;
                        if cancel.load(Ordering::Relaxed) {
                            return Err(());
                        }
                        let path = reply.path.ok_or(())?;
                        let root = reply.root.ok_or(())?;
                        let (resource, image) = artwork::prepare_download(&path, &root).ok_or(())?;
                        Event::Preview(grid.id, resource, image)
                    }
                    Task::Apply(id, resource) => {
                        artwork::save_choice(&id, resource.as_ref()).map_err(|_| ())?;
                        Event::Saved
                    }
                })
            })();
            let _ = tx.send(result.unwrap_or(Event::Error));
        });
    }
    fn request_preview(&mut self) {
        if self.receiver.is_none()
            && self.in_grids
            && self.preview.as_ref().map(|p| p.0) != self.grids.get(self.index).map(|g| g.id)
        {
            if let Some(grid) = self.grids.get(self.index) {
                self.start(Task::Preview(grid.clone()));
            }
        }
    }
    fn paint(&self, ui: &Home) {
        ui.set_artwork_picker_open(true);
        ui.set_artwork_picker_keyboard(self.keyboard);
        ui.set_artwork_picker_image_available(
            self.in_grids
                && self.preview.as_ref().map(|p| p.0) == self.grids.get(self.index).map(|g| g.id)
                && self.preview.is_some(),
        );
        ui.set_artwork_picker_query(self.query.clone().into());
        ui.set_artwork_picker_key(self.key as i32);
        ui.set_artwork_picker_title(
            format!("{} · {}", self.title, self.system.to_uppercase()).into(),
        );
        let labels: Vec<SharedString> = if self.in_grids {
            self.grids
                .iter()
                .map(|g| {
                    format!(
                        "{} · {}",
                        g.author
                            .as_ref()
                            .map(|a| a.name.as_str())
                            .unwrap_or("Autor sin indicar"),
                        g.style.as_deref().unwrap_or("Carátula")
                    )
                    .into()
                })
                .collect()
        } else {
            self.games
                .iter()
                .map(|g| {
                    format!(
                        "{}{}",
                        g.name,
                        g.release_date
                            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                            .map(|d| format!(" · {}", d.format("%Y")))
                            .unwrap_or_default()
                    )
                    .into()
                })
                .collect()
        };
        let offset = self
            .index
            .saturating_sub(2)
            .min(labels.len().saturating_sub(5));
        ui.set_artwork_picker_rows(ModelRc::new(VecModel::from(
            labels.into_iter().skip(offset).take(5).collect::<Vec<_>>(),
        )));
        ui.set_artwork_picker_offset(offset as i32);
        ui.set_artwork_picker_index(self.index as i32);
        ui.set_artwork_picker_status(self.status.clone().into());
        ui.set_artwork_picker_ready(
            self.in_grids
                && self.receiver.is_none()
                && self.preview.as_ref().map(|p| p.0) == self.grids.get(self.index).map(|g| g.id)
                && self.preview.is_some(),
        );
    }
}
impl State {
    pub(super) fn artwork_picker_choose(&mut self, ui: &Home, index: i32) {
        let Some(picker) = &mut self.artwork_picker else {
            return;
        };
        if index < 0 || picker.keyboard {
            return;
        }
        let index = index as usize;
        let count = if picker.in_grids {
            picker.grids.len()
        } else {
            picker.games.len()
        };
        if index >= count {
            return;
        }
        picker.index = index;
        if picker.in_grids {
            ui.set_artwork_picker_image_available(false);
            picker.request_preview();
            picker.paint(ui);
        } else {
            self.artwork_picker_action(ui, Action::Confirm);
        }
    }
    pub(super) fn open_artwork_picker(&mut self, ui: &Home, index: usize) {
        let Some(game) = self.games.get(index) else {
            return;
        };
        let mut picker = Picker {
            game_id: game.id.clone(),
            title: game.title.clone(),
            system: game.system.clone(),
            query: game.title.chars().take(120).collect(),
            index: 0,
            key: 0,
            games: Vec::new(),
            grids: Vec::new(),
            in_grids: false,
            keyboard: false,
            status: String::new(),
            receiver: None,
            preview: None,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        picker.start(Task::Search(picker.query.clone()));
        ui.set_artwork_picker_image_available(false);
        picker.paint(ui);
        self.artwork_picker = Some(picker);
    }
    pub(super) fn close_artwork_picker(&mut self, ui: &Home) {
        self.artwork_picker = None;
        ui.set_artwork_picker_open(false);
        ui.set_artwork_picker_image_available(false);
    }
    pub(super) fn artwork_picker_action(&mut self, ui: &Home, action: Action) {
        let Some(picker) = &mut self.artwork_picker else {
            return;
        };
        if action == Action::Back {
            if picker.keyboard {
                picker.keyboard = false;
            } else if picker.in_grids && picker.receiver.is_none() {
                picker.in_grids = false;
                picker.index = 0;
                picker.preview = None;
                ui.set_artwork_picker_image_available(false);
            } else {
                self.close_artwork_picker(ui);
                return;
            }
        } else if action == Action::Library && !picker.keyboard && picker.receiver.is_none() {
            picker.start(Task::Apply(picker.game_id.clone(), None));
        } else if action == Action::Refresh && picker.receiver.is_none() {
            picker.keyboard = true;
        } else if picker.keyboard {
            match action {
                Action::Left => picker.key = picker.key.saturating_sub(1),
                Action::Right => picker.key = (picker.key + 1).min(39),
                Action::Up => picker.key = picker.key.saturating_sub(10),
                Action::Down => picker.key = (picker.key + 10).min(39),
                Action::Confirm => {
                    let key = picker.key;
                    self.artwork_picker_key(ui, key);
                    return;
                }
                Action::Menu => {
                    picker.keyboard = false;
                    picker.in_grids = false;
                    picker.index = 0;
                    picker.start(Task::Search(picker.query.clone()));
                }
                _ => {}
            }
        } else {
            let count = if picker.in_grids {
                picker.grids.len()
            } else {
                picker.games.len()
            };
            match action {
                Action::Up => picker.index = picker.index.saturating_sub(1),
                Action::Down => picker.index = (picker.index + 1).min(count.saturating_sub(1)),
                Action::Confirm if picker.receiver.is_none() => {
                    if picker.in_grids {
                        if let Some((id, resource)) = &picker.preview {
                            if picker.grids.get(picker.index).is_some_and(|g| g.id == *id) {
                                picker.start(Task::Apply(
                                    picker.game_id.clone(),
                                    Some(resource.clone()),
                                ));
                            }
                        }
                    } else if let Some(game) = picker.games.get(picker.index) {
                        picker.start(Task::Grids(game.id));
                    }
                }
                _ => {}
            }
            picker.request_preview();
            if picker.in_grids
                && picker.preview.as_ref().map(|p| p.0)
                    != picker.grids.get(picker.index).map(|g| g.id)
            {
                ui.set_artwork_picker_image_available(false);
            }
        }
        if let Some(picker) = &self.artwork_picker {
            picker.paint(ui);
        }
    }
    pub(super) fn artwork_picker_key(&mut self, ui: &Home, key: usize) {
        let Some(picker) = &mut self.artwork_picker else {
            return;
        };
        if !picker.keyboard || picker.receiver.is_some() {
            return;
        }
        match key {
            0..=35 if picker.query.chars().count() < 120 => {
                picker.query.push_str(crate::library_browser::KEYS[key])
            }
            36 if picker.query.chars().count() < 120 => picker.query.push(' '),
            37 => {
                picker.query.pop();
            }
            38 => {
                picker.keyboard = false;
                picker.in_grids = false;
                picker.index = 0;
                picker.start(Task::Search(picker.query.clone()));
            }
            39 => picker.keyboard = false,
            _ => {}
        }
        picker.paint(ui);
    }
    pub(super) fn artwork_picker_tick(&mut self, ui: &Home) {
        let Some(picker) = &mut self.artwork_picker else {
            return;
        };
        let event = picker.receiver.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(event) = event else {
            return;
        };
        picker.receiver = None;
        let failed = matches!(&event, Event::Error);
        match event {
            Event::Games(games)=>{picker.games=games;picker.index=0;picker.status="Elige el juego correcto; comprueba edición y sistema.".into();if picker.games.is_empty(){picker.status="Sin coincidencias. Pulsa X para buscar otro nombre.".into();}}
            Event::Grids(grids)=>{picker.grids=grids;picker.index=0;picker.in_grids=true;picker.preview=None;ui.set_artwork_picker_image_available(false);picker.status="Elige una carátula vertical.".into();}
            Event::Preview(id,resource,image)=>{
                if picker.grids.get(picker.index).is_some_and(|g|g.id==id) {
                    let pixels=slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&image.pixels,image.width,image.height);
                    ui.set_artwork_picker_image(slint::Image::from_rgba8(pixels));ui.set_artwork_picker_image_available(true);
                    picker.preview=Some((id,resource));picker.status="A aplicar esta carátula · B volver".into();
                }
            }
            Event::Saved=>{self.close_artwork_picker(ui);ui.set_artwork_generation(ui.get_artwork_generation().wrapping_add(1));self.notify(ui,"Carátula actualizada. Tus juegos y partidas se conservan.");return;}
            Event::Error=>picker.status="No se pudo cargar. Comprueba la clave de SteamGridDB y la conexión; X permite volver a buscar.".into(),
        }
        if !failed {
            picker.request_preview();
        }
        picker.paint(ui);
    }
}
