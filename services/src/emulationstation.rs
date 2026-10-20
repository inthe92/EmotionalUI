//! Non-destructive gamelist adapter. Metadata is data, never a shell command.
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
const MAX_XML: u64 = 8 * 1024 * 1024;
#[derive(Debug, Clone, Serialize)]
pub struct ImportedGame {
    pub id: String,
    pub rom: PathBuf,
    pub source: PathBuf,
    pub metadata: BTreeMap<String, String>,
}
impl ImportedGame {
    /// Revalidate immediately before any future launch; an import is only a snapshot.
    pub fn available(&self, root: &Path) -> bool {
        let Ok(root) = root.canonicalize() else {
            return false;
        };
        self.rom
            .canonicalize()
            .ok()
            .is_some_and(|p| p.starts_with(&root) && p.is_file())
    }
}
#[derive(Debug, Default)]
pub struct ImportReport {
    pub games: Vec<ImportedGame>,
    pub skipped: usize,
}
pub fn import(gamelist: &Path, root: &Path) -> Result<ImportReport, Box<dyn Error>> {
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err("La raíz de ROMs debe ser un directorio".into());
    }
    let source = gamelist.canonicalize()?;
    let mut xml = String::new();
    File::open(&source)?
        .take(MAX_XML + 1)
        .read_to_string(&mut xml)?;
    if xml.len() as u64 > MAX_XML {
        return Err("gamelist.xml supera 8 MiB".into());
    }
    // roxmltree disables DTD by default; keep it explicitly disabled and cap nodes.
    let doc = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
        },
    )?;
    if !doc.root_element().has_tag_name("gameList") {
        return Err("Se esperaba gameList".into());
    }
    if doc.descendants().any(|n| n.ancestors().count() > 32) {
        return Err("XML demasiado profundo".into());
    }
    let mut result = ImportReport::default();
    let mut seen = HashSet::new();
    for node in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("game"))
    {
        let mut metadata = BTreeMap::new();
        for child in node.children().filter(|n| n.is_element()) {
            let value = child.text().unwrap_or("");
            if value.len() > 16 * 1024 {
                return Err("Campo XML demasiado largo".into());
            }
            metadata.insert(child.tag_name().name().to_owned(), value.to_owned());
        }
        let raw = metadata.get("path").map(|s| s.trim()).unwrap_or("");
        let path = Path::new(raw);
        // ~/ expansion is deliberately not implicit: use an explicit authorized root.
        let candidate = if path.is_absolute() {
            path.to_owned()
        } else {
            root.join(path)
        };
        let Some(rom) = candidate
            .canonicalize()
            .ok()
            .filter(|p| !raw.is_empty() && p.is_file() && p.starts_with(&root))
        else {
            result.skipped += 1;
            continue;
        };
        if !seen.insert(rom.clone()) {
            result.skipped += 1;
            continue;
        }
        result.games.push(ImportedGame {
            id: format!("es:{}", rom.to_string_lossy()),
            rom,
            source: source.clone(),
            metadata,
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "emotional-import-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn run(&self, xml: &str) -> Result<ImportReport, Box<dyn Error>> {
            let p = self.0.join("gamelist.xml");
            std::fs::write(&p, xml)?;
            import(&p, &self.0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn reads_metadata_without_altering_sources() {
        let f = Fixture::new();
        let rom = f.0.join("original.test");
        std::fs::write(&rom, b"synthetic fixture").unwrap();
        let xml="<gameList><game><path>./original.test</path><name>A &amp; B</name><favorite>true</favorite><playcount>7</playcount><lastplayed>20261018T000000</lastplayed><image>./art.png</image><desc><![CDATA[Texto <original>]]></desc></game></gameList>";
        let r = f.run(xml).unwrap();
        assert_eq!(r.games.len(), 1);
        assert_eq!(r.games[0].metadata["name"], "A & B");
        assert_eq!(r.games[0].metadata["playcount"], "7");
        assert!(r.games[0].available(&f.0));
        assert_eq!(std::fs::read(&rom).unwrap(), b"synthetic fixture");
        assert_eq!(
            std::fs::read_to_string(f.0.join("gamelist.xml")).unwrap(),
            xml
        );
        std::fs::remove_file(rom).unwrap();
        assert!(!r.games[0].available(&f.0));
    }
    #[test]
    fn rejects_missing_outside_and_duplicates() {
        let f = Fixture::new();
        std::fs::write(f.0.join("one.test"), "fixture").unwrap();
        let r=f.run("<gameList><game><path>one.test</path></game><game><path>./one.test</path></game><game><path>missing</path></game><game><path>/etc/hosts</path></game><game><path>../outside</path></game></gameList>").unwrap();
        assert_eq!(r.games.len(), 1);
        assert_eq!(r.skipped, 4);
    }
    #[test]
    fn dtd_and_malformed_rejected() {
        let f = Fixture::new();
        assert!(f.run("<!DOCTYPE gameList [<!ENTITY x SYSTEM 'file:///etc/hosts'>]><gameList>&x;</gameList>").is_err());
        assert!(f.run("<gameList><game></gameList>").is_err());
    }
    #[test]
    fn empty_catalog_is_valid() {
        let f = Fixture::new();
        assert!(f.run("<gameList/>").unwrap().games.is_empty());
    }
    #[test]
    fn large_and_deep_documents_rejected() {
        let f = Fixture::new();
        assert!(f
            .run(&format!(
                "<gameList>{}</gameList>",
                " ".repeat(MAX_XML as usize)
            ))
            .is_err());
        assert!(f
            .run(&format!(
                "<gameList>{}{}</gameList>",
                "<folder>".repeat(33),
                "</folder>".repeat(33)
            ))
            .is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let f = Fixture::new();
        std::os::unix::fs::symlink("/etc/hosts", f.0.join("escape")).unwrap();
        assert_eq!(
            f.run("<gameList><game><path>escape</path></game></gameList>")
                .unwrap()
                .skipped,
            1
        );
    }
}
