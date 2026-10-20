//! Local editorial artwork. Selection, decoding, composition and disk I/O run on
//! the existing single worker; Slint receives only bounded prepared pixels.
use image::{imageops, DynamicImage, ImageFormat, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    hash::{Hash, Hasher},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
};
const INPUT: u64 = 8 * 1024 * 1024;
const ALGORITHM: &str = "editorial-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Curated,
    Hero,
    Boxart,
    Screenshot,
    Title,
    Logo,
    Generic,
}
#[derive(Clone, Debug)]
pub struct Resource {
    pub path: PathBuf,
    pub role: Role,
    pub root: PathBuf,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Landscape,
    Boxart,
    Screenshot,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Presentation {
    Hero,
    Tile,
}
#[derive(Clone, Debug)]
pub struct Request {
    pub id: String,
    pub game_id: String,
    pub resources: Vec<Resource>,
    pub width: u32,
    pub height: u32,
    pub presentation: Presentation,
}
#[derive(Clone)]
pub struct Prepared {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub mode: Mode,
    pub cache_hit: bool,
}
pub struct Thumbnail {
    pub id: String,
    pub prepared: Option<Prepared>,
}
pub struct Worker {
    pub request: mpsc::SyncSender<Request>,
    pub ready: mpsc::Receiver<Thumbnail>,
}
fn allowed(path: &Path, root: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let path = path.canonicalize().ok()?;
    (path.starts_with(&root) && path.is_file()).then_some(path)
}
fn probe(resource: &Resource) -> Option<(u32, u32)> {
    let path = allowed(&resource.path, &resource.root)?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg")
        || fs::metadata(&path).ok()?.len() > INPUT
    {
        return None;
    }
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    if !matches!(reader.format()?, ImageFormat::Png | ImageFormat::Jpeg) {
        return None;
    }
    let (w, h) = reader.into_dimensions().ok()?;
    (w > 0 && h > 0 && w <= 2048 && h <= 2048).then_some((w, h))
}
fn source(resource: &Resource) -> Option<(Vec<u8>, ImageFormat, u32, u32)> {
    let path = allowed(&resource.path, &resource.root)?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        return None;
    }
    let mut file = fs::File::open(&path).ok()?;
    if !file.metadata().ok()?.is_file() || file.metadata().ok()?.len() > INPUT {
        return None;
    }
    // Check containment again after open. No command or external decoder is used.
    if allowed(&resource.path, &resource.root)? != path {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata().ok()?;
        let current = fs::metadata(&path).ok()?;
        if opened.dev() != current.dev() || opened.ino() != current.ino() {
            return None;
        }
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(INPUT + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > INPUT {
        return None;
    }
    let format = image::guess_format(&bytes).ok()?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return None;
    }
    let (w, h) = image::ImageReader::with_format(Cursor::new(&bytes), format)
        .into_dimensions()
        .ok()?;
    if w == 0 || h == 0 || w > 2048 || h > 2048 {
        return None;
    }
    Some((bytes, format, w, h))
}
fn mode(role: Role, w: u32, h: u32) -> Option<Mode> {
    match role {
        Role::Curated if h > w => Some(Mode::Boxart),
        Role::Curated => Some(Mode::Landscape),
        Role::Hero if w >= h * 4 / 3 && w >= 160 && h >= 90 => Some(Mode::Landscape),
        Role::Boxart => Some(Mode::Boxart),
        Role::Screenshot | Role::Title => Some(Mode::Screenshot),
        Role::Logo => None, // A logo alone is not scene art; the title remains readable in Slint.
        Role::Generic if h > w => Some(Mode::Boxart),
        Role::Generic => Some(Mode::Screenshot),
        _ => None,
    }
}
fn priority(mode: Mode, p: Presentation) -> u8 {
    match (p, mode) {
        (_, Mode::Landscape) => 0,
        (Presentation::Hero, Mode::Boxart) | (Presentation::Tile, Mode::Screenshot) => 1,
        _ => 2,
    }
}
fn read_image(bytes: &[u8], format: ImageFormat) -> Option<DynamicImage> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(24 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().ok()
}
fn opaque(pixel: Rgba<u8>) -> Rgba<u8> {
    let a = u32::from(pixel[3]);
    let base = [32, 38, 34];
    Rgba([
        ((u32::from(pixel[0]) * a + base[0] * (255 - a)) / 255) as u8,
        ((u32::from(pixel[1]) * a + base[1] * (255 - a)) / 255) as u8,
        ((u32::from(pixel[2]) * a + base[2] * (255 - a)) / 255) as u8,
        255,
    ])
}
/// Contain instead of crop: logos, characters and screenshot edges survive.
fn compose(image: DynamicImage, mode: Mode, w: u32, h: u32) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(w, h, Rgba([32, 38, 34, 255]));
    if mode != Mode::Screenshot {
        let small = image
            .thumbnail(40, 32)
            .resize_to_fill(w, h, imageops::FilterType::Triangle)
            .to_rgba8();
        canvas = imageops::blur(&small, 5.0);
        for (x, _, p) in canvas.enumerate_pixels_mut() {
            let src = opaque(*p);
            let dim = 0.35 + 0.15 * (1.0 - x as f32 / w as f32);
            *p = Rgba([
                (f32::from(src[0]) * dim) as u8,
                (f32::from(src[1]) * dim) as u8,
                (f32::from(src[2]) * dim) as u8,
                255,
            ]);
        }
    }
    let (max_w, max_h) = if mode == Mode::Boxart {
        (w.saturating_sub(20), h.saturating_sub(16))
    } else {
        (w, h)
    };
    let filter = if mode == Mode::Screenshot {
        imageops::FilterType::Nearest
    } else {
        imageops::FilterType::Lanczos3
    };
    let scaled = if mode == Mode::Screenshot && image.width() <= max_w && image.height() <= max_h {
        let scale = (max_w / image.width()).min(max_h / image.height()).max(1);
        image.resize_exact(image.width() * scale, image.height() * scale, filter)
    } else {
        image.resize(max_w.max(1), max_h.max(1), filter)
    }
    .to_rgba8();
    let x = (w - scaled.width()) / 2;
    let y = (h - scaled.height()) / 2;
    imageops::overlay(&mut canvas, &scaled, i64::from(x), i64::from(y));
    for p in canvas.pixels_mut() {
        *p = opaque(*p);
    }
    canvas
}
fn key(request: &Request, resource: &Resource, bytes: &[u8], mode: Mode) -> String {
    // Derived cache identity, not an authenticity/security checksum. SipHash's
    // implementation is pinned by the toolchain and the algorithm namespace.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (
        ALGORITHM,
        &request.game_id,
        &resource.path,
        bytes,
        request.width,
        request.height,
        request.presentation,
        mode,
    )
        .hash(&mut h);
    format!("{ALGORITHM}-{:016x}.png", h.finish())
}
/// Resolve even a not-yet-created destination through existing parent aliases
/// (notably /var -> /private/var on macOS), before allowing any cache write.
fn destination(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        if let Ok(mut resolved) = ancestor.canonicalize() {
            for part in missing.iter().rev() {
                resolved.push(part);
            }
            return Some(resolved);
        }
        missing.push(ancestor.file_name()?.to_os_string());
        ancestor = ancestor.parent()?;
    }
}
pub struct Cache {
    directory: Option<PathBuf>,
    max_bytes: u64,
    memory: VecDeque<(String, Prepared)>,
    memory_bytes: usize,
}
impl Cache {
    pub fn new(directory: Option<PathBuf>, max_bytes: u64) -> Self {
        Self {
            directory,
            max_bytes: max_bytes.clamp(1024 * 1024, 256 * 1024 * 1024),
            memory: VecDeque::new(),
            memory_bytes: 0,
        }
    }
    pub fn configured() -> Self {
        let directory = std::env::var_os("EMOTIONAL_ARTWORK_CACHE")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache")))
                    .map(|p| p.join("emotional-ui/artwork-v1"))
            });
        let max = std::env::var("EMOTIONAL_ARTWORK_CACHE_MIB")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(64)
            .clamp(1, 256);
        Self::new(directory, max * 1024 * 1024)
    }
    fn remember(&mut self, key: String, prepared: Prepared) {
        let size = prepared.pixels.len();
        while self.memory_bytes + size > 4 * 1024 * 1024 {
            if let Some((_, p)) = self.memory.pop_front() {
                self.memory_bytes -= p.pixels.len();
            } else {
                break;
            }
        }
        self.memory_bytes += size;
        self.memory.push_back((key, prepared));
    }
    fn get(&mut self, key: &str, w: u32, h: u32, mode: Mode) -> Option<Prepared> {
        if let Some(index) = self.memory.iter().position(|(k, _)| k == key) {
            let entry = self.memory.remove(index)?;
            let mut result = entry.1.clone();
            result.cache_hit = true;
            self.memory.push_back(entry);
            return Some(result);
        }
        let path = self.directory.as_ref()?.join(key);
        let meta = fs::symlink_metadata(&path).ok()?;
        if !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
            return None;
        }
        let resource = Resource {
            path,
            root: self.directory.clone()?,
            role: Role::Generic,
        };
        let (bytes, format, cw, ch) = source(&resource)?;
        if cw != w || ch != h {
            return None;
        }
        let pixels = read_image(&bytes, format)?.to_rgba8().into_raw();
        let p = Prepared {
            width: w,
            height: h,
            pixels,
            mode,
            cache_hit: true,
        };
        self.remember(key.into(), p.clone());
        Some(p)
    }
    fn put(&mut self, key: &str, p: &Prepared) {
        self.remember(key.into(), p.clone());
        let Some(dir) = &self.directory else {
            return;
        };
        if fs::create_dir_all(dir).is_err()
            || fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_symlink())
        {
            return;
        }
        let target = dir.join(key);
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let temp = dir.join(format!(
            ".{key}.{}-{}.part",
            std::process::id(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let write = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::ImageRgba8(
                RgbaImage::from_raw(p.width, p.height, p.pixels.clone()).ok_or("pixels")?,
            )
            .write_to(&mut bytes, ImageFormat::Png)?;
            file.write_all(bytes.get_ref())?;
            file.sync_all()?;
            fs::rename(&temp, &target)?;
            Ok(())
        })();
        if write.is_err() {
            let _ = fs::remove_file(&temp);
        }
        self.prune();
    }
    fn prune(&self) {
        let Some(dir) = &self.directory else {
            return;
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut files = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name();
                let name = name.to_str()?;
                let valid = name
                    .strip_prefix(&format!("{ALGORITHM}-"))
                    .and_then(|s| s.strip_suffix(".png"))
                    .is_some_and(|s| s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()));
                if !valid {
                    return None;
                }
                let m = fs::symlink_metadata(e.path()).ok()?;
                m.is_file().then(|| (m.modified().ok(), m.len(), e.path()))
            })
            .collect::<Vec<_>>();
        files.sort_by_key(|f| f.0);
        let mut total = files.iter().map(|f| f.1).sum::<u64>();
        for (_, len, path) in files {
            if total <= self.max_bytes {
                break;
            }
            if fs::remove_file(path).is_ok() {
                total = total.saturating_sub(len);
            }
        }
    }
    pub fn prepare(&mut self, r: &Request) -> Option<Prepared> {
        if r.width == 0 || r.height == 0 || r.width > 640 || r.height > 480 {
            return None;
        }
        if let Some(directory) = &self.directory {
            let resolved = destination(directory);
            if resolved.as_ref().is_none_or(|dir| {
                r.resources.iter().any(|a| {
                    a.root
                        .canonicalize()
                        .is_ok_and(|root| dir.starts_with(root))
                })
            }) {
                self.directory = None;
            }
        }
        // Probe one bounded input at a time. Do not retain all source files.
        let mut candidates = r
            .resources
            .iter()
            .take(16)
            .filter_map(|resource| {
                let (w, h) = probe(resource)?;
                let mode = mode(resource.role, w, h)?;
                Some((
                    if resource.role == Role::Curated {
                        0
                    } else {
                        1 + priority(mode, r.presentation)
                    },
                    std::cmp::Reverse(w.min(h)),
                    resource,
                    mode,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|c| (c.0, c.1));
        for (_, _, resource, mode) in candidates {
            let Some((bytes, format, _, _)) = source(resource) else {
                continue;
            };
            let key = key(r, resource, &bytes, mode);
            if let Some(p) = self.get(&key, r.width, r.height, mode) {
                return Some(p);
            }
            if let Some(image) = read_image(&bytes, format) {
                let pixels = compose(image, mode, r.width, r.height).into_raw();
                let p = Prepared {
                    width: r.width,
                    height: r.height,
                    pixels,
                    mode,
                    cache_hit: false,
                };
                self.put(&key, &p);
                return Some(p);
            }
        }
        None
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Overrides {
    games: std::collections::BTreeMap<String, Override>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    path: PathBuf,
    root: PathBuf,
}
fn overrides() -> std::collections::BTreeMap<String, Override> {
    let parsed = (|| -> Option<Overrides> {
        let path = std::env::var_os("EMOTIONAL_ARTWORK_OVERRIDES")?;
        let file = fs::File::open(path).ok()?;
        let mut text = String::new();
        file.take(256 * 1024 + 1).read_to_string(&mut text).ok()?;
        if text.len() > 256 * 1024 {
            return None;
        }
        toml::from_str(&text).ok()
    })();
    parsed.map(|p| p.games).unwrap_or_default()
}
impl Worker {
    pub fn start() -> Self {
        let (request, rx) = mpsc::sync_channel::<Request>(1);
        let (tx, ready) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut cache = Cache::configured();
            let overrides = overrides();
            while let Ok(mut r) = rx.recv() {
                while let Ok(new) = rx.try_recv() {
                    r = new;
                }
                if let Some(art) = overrides.get(&r.game_id) {
                    r.resources.insert(
                        0,
                        Resource {
                            path: art.path.clone(),
                            root: art.root.clone(),
                            role: Role::Curated,
                        },
                    );
                }
                #[cfg(feature = "simulator")]
                if !r.resources.is_empty() {
                    if let Some(ms) = std::env::var("EMOTIONAL_ARTWORK_DELAY_MS")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        std::thread::sleep(std::time::Duration::from_millis(ms.min(2000)));
                    }
                }
                let prepared = cache.prepare(&r);
                if tx.send(Thumbnail { id: r.id, prepared }).is_err() {
                    break;
                }
            }
        });
        Self { request, ready }
    }
}
// Kept for existing safety tests and tools; follows the same bounded decoder.
#[cfg(test)]
pub fn decode(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let p = path.canonicalize().ok()?;
    let root = p.parent()?.into();
    let (bytes, format, _, _) = source(&Resource {
        path: p,
        root,
        role: Role::Generic,
    })?;
    let img = read_image(&bytes, format)?.thumbnail(292, 232).to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}
#[cfg(test)]
#[path = "artwork/tests.rs"]
mod tests;
