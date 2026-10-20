use std::{collections::BTreeMap, path::Path, sync::Arc};
pub const RATE: u32 = 22050;
pub const MAX_RESOURCE_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone, Default)]
pub struct Library {
    pub clips: BTreeMap<String, Arc<[i16]>>,
    pub missing: Vec<String>,
}
impl Library {
    pub fn load(directory: Option<&Path>) -> Self {
        let sources: [(&str, &[u8]); 5] = [
            (
                "umbral",
                include_bytes!("../../../assets/audio/runtime/umbral.wav"),
            ),
            (
                "brasa",
                include_bytes!("../../../assets/audio/runtime/brasa.wav"),
            ),
            (
                "rastro",
                include_bytes!("../../../assets/audio/runtime/rastro.wav"),
            ),
            (
                "rain",
                include_bytes!("../../../assets/audio/runtime/rain.wav"),
            ),
            (
                "air",
                include_bytes!("../../../assets/audio/runtime/air.wav"),
            ),
        ];
        let mut result = Self::default();
        for (id, bundled) in sources {
            let decoded = if let Some(root) = directory {
                let path = root.join(format!("{id}.wav"));
                std::fs::metadata(&path)
                    .ok()
                    .filter(|m| m.len() <= MAX_RESOURCE_BYTES as u64)
                    .and_then(|_| std::fs::read(path).ok())
                    .and_then(|b| decode(&b).ok())
            } else {
                decode(bundled).ok()
            };
            if let Some(samples) = decoded {
                result.clips.insert(id.into(), samples.into());
            } else {
                result.missing.push(id.into());
            }
        }
        result
    }
    pub fn bytes(&self) -> usize {
        self.clips.values().map(|s| s.len() * 2).sum()
    }
}
pub fn decode(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if bytes.len() > MAX_RESOURCE_BYTES
        || bytes.get(..4) != Some(b"RIFF")
        || bytes.get(8..12) != Some(b"WAVE")
    {
        return Err("Not a bounded WAV".into());
    }
    let mut at = 12;
    let mut format = false;
    let mut data = None;
    while at + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let end = at
            .checked_add(8)
            .and_then(|v| v.checked_add(len))
            .filter(|v| *v <= bytes.len())
            .ok_or("Truncated WAV")?;
        let chunk = &bytes[at + 8..end];
        match &bytes[at..at + 4] {
            b"fmt " => {
                format = chunk.len() >= 16
                    && chunk[..4] == [1, 0, 1, 0]
                    && u32::from_le_bytes(chunk[4..8].try_into().unwrap()) == RATE
                    && chunk[12..16] == [2, 0, 16, 0];
            }
            b"data" => {
                if data.is_some() || !len.is_multiple_of(2) {
                    return Err("Invalid PCM data".into());
                }
                data = Some(chunk);
            }
            _ => {}
        }
        at = end + len % 2;
    }
    if !format {
        return Err("Expected mono S16_LE 22050Hz".into());
    }
    let data = data.ok_or("Missing PCM")?;
    if data.is_empty() {
        return Err("Empty PCM".into());
    }
    Ok(data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_resources_are_bounded_and_real() {
        let a = Library::load(None);
        assert!(a.missing.is_empty());
        assert!(a.bytes() < 4 * 1024 * 1024);
        assert!(a.clips.values().all(|c| c.iter().any(|v| *v != 0)));
    }
    #[test]
    fn nonexistent_assets_are_optional() {
        let a = Library::load(Some(Path::new("/emotional-ui-deliberately-missing")));
        assert_eq!(a.missing.len(), 5);
        assert_eq!(a.bytes(), 0);
    }
    #[test]
    fn malformed_and_truncated_wav_rejected() {
        assert!(decode(b"RIFFxxxxxxxxWAVE").is_err());
        let mut b = include_bytes!("../../../assets/audio/runtime/umbral.wav").to_vec();
        b.truncate(50);
        assert!(decode(&b).is_err());
    }
}
