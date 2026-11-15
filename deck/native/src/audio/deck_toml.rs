//! deck.toml's audio keys (#208, #428): the machine's host, device, buffer, render thread and channel map – read, and
//! written back into the file without touching its other keys and comments (the session's, read by deck-session with sed:
//! one key per line, strings in double quotes).
use anyhow::{Context, Result};
use std::path::PathBuf;

/// The machine's audio settings; the stage's flags win over them, they win over the scene's.
/// `audio_buffer = 256`, `audio_host = "asio"`, `audio_device = "Fireface"`; empty = unset.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct DeckAudio {
    #[serde(default)]
    pub audio_buffer: Option<u32>,
    #[serde(default)]
    pub audio_host: Option<String>,
    #[serde(default)]
    pub audio_device: Option<String>,
    #[serde(default)]
    pub audio_thread: Option<bool>,
    /// this machine's channel map (#428): `[[audio_outlet]]`, `group = "cue"`, `channels = [3, 4]`, counted from 1 – the scene names
    /// the group, this says where it plays. `main` without an entry plays on channels 1.., any other group without one is silent
    #[serde(default)]
    pub audio_outlet: Vec<deck_engine::OutletMap>,
}

impl DeckAudio {
    /// deck.toml's text (its other keys are the session's; a broken file = nothing set, said in the log)
    pub fn parse(text: &str) -> DeckAudio {
        let d: DeckAudio = toml::from_str(text).unwrap_or_else(|e| {
            tracing::warn!("deck.toml (audio): {e}");
            DeckAudio::default()
        });
        d.tidy()
    }

    /// empty strings and a zero buffer are unset; audio_thread stays as written
    fn tidy(mut self) -> DeckAudio {
        self.audio_host = self.audio_host.filter(|s| !s.trim().is_empty());
        self.audio_device = self.audio_device.filter(|s| !s.trim().is_empty());
        self.audio_buffer = self.audio_buffer.filter(|&n| n > 0);
        self
    }

    /// the deck's file; `DECK_TOML` names another (a dev machine's)
    pub fn path() -> PathBuf {
        std::env::var_os("DECK_TOML").map(PathBuf::from).unwrap_or_else(|| "/var/lib/deck/deck.toml".into())
    }

    /// the deck's file, nothing set when there is none
    pub fn read() -> DeckAudio {
        std::fs::read_to_string(Self::path()).map(|t| DeckAudio::parse(&t)).unwrap_or_default()
    }

    /// `text` with these audio keys in it: each key set in place (its comment kept) or removed when unset, the channel map
    /// replaced; everything else as it was. A file that is not TOML is refused, never overwritten.
    pub fn write(&self, text: &str) -> Result<String> {
        use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};
        let mut doc: DocumentMut = text.parse().context("deck.toml is not TOML – left as it is")?;
        let d = self.clone().tidy();
        let root = doc.as_table_mut();
        let mut put = |key: &str, new: Option<toml_edit::Value>| match (new, root.get_mut(key)) {
            (Some(mut v), Some(Item::Value(old))) => {
                *v.decor_mut() = old.decor().clone();
                *old = v;
            }
            (Some(v), _) => {
                root.insert(key, Item::Value(v));
            }
            (None, _) => {
                root.remove(key);
            }
        };
        put("audio_host", d.audio_host.map(|s| s.into()));
        put("audio_device", d.audio_device.map(|s| s.into()));
        put("audio_buffer", Some(i64::from(d.audio_buffer.unwrap_or(0)).into()));
        put("audio_thread", d.audio_thread.map(|b| b.into()));
        let mut maps = ArrayOfTables::new();
        for o in &d.audio_outlet {
            let mut t = Table::new();
            t.insert("group", value(o.group.as_str()));
            t.insert("channels", value(o.channels.iter().map(|&c| c as i64).collect::<Array>()));
            maps.push(t);
        }
        match (maps.is_empty(), root.get_mut("audio_outlet")) {
            (true, _) => {
                root.remove("audio_outlet");
            }
            (false, Some(Item::ArrayOfTables(old))) => {
                for (new, was) in maps.iter_mut().zip(old.iter()) {
                    *new.decor_mut() = was.decor().clone();
                    if let Some(p) = was.position() {
                        new.set_position(p);
                    }
                }
                *old = maps;
            }
            (false, _) => {
                root.insert("audio_outlet", Item::ArrayOfTables(maps));
            }
        }
        Ok(doc.to_string())
    }

    /// writes these keys into the deck's file (made when there is none) through a file beside it; the path written
    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::path();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let next = self.write(&text)?;
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        }
        let tmp = path.with_extension("toml.new");
        std::fs::write(&tmp, next).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_engine::OutletMap;

    #[test]
    fn deck_toml_names_the_machines_audio() {
        let d = DeckAudio::parse("kiosk = \"stage\"\nrelay = \"\"\naudio_buffer = 256\naudio_host = \"asio\"\naudio_device = \"Fireface\"\n");
        assert_eq!(d, DeckAudio { audio_buffer: Some(256), audio_host: Some("asio".into()), audio_device: Some("Fireface".into()), audio_thread: None, audio_outlet: vec![] });
        let map = DeckAudio::parse("outlet = \"stage\"\naudio_host = \"asio\"\n[[audio_outlet]]\ngroup = \"cue\"\nchannels = [3, 4]\n");
        assert_eq!(map.audio_outlet, [OutletMap { group: "cue".into(), channels: vec![3, 4] }]);
        assert_eq!(DeckAudio::parse("audio_thread = false\n").audio_thread, Some(false));
        // the file a deck starts with: the device's default buffer, the render thread on
        let shipped = DeckAudio::parse(include_str!("../../../os/rootfs/usr/share/deck/deck.default.toml"));
        assert_eq!((shipped.audio_buffer, shipped.audio_thread), (None, Some(true)));
        let empty = DeckAudio::parse("audio_host = \"\"\naudio_device = \" \"\naudio_buffer = 0\n");
        assert_eq!(empty, DeckAudio::default(), "empty = unset");
        assert_eq!(DeckAudio::parse("this is = not toml ["), DeckAudio::default());
    }

    fn rme() -> DeckAudio {
        DeckAudio {
            audio_buffer: Some(128),
            audio_host: Some("asio".into()),
            audio_device: Some("ASIO Fireface USB".into()),
            audio_thread: Some(true),
            audio_outlet: vec![OutletMap { group: "main".into(), channels: vec![1, 2] }, OutletMap { group: "cue".into(), channels: vec![3, 4] }],
        }
    }

    #[test]
    fn written_audio_keys_read_back_and_the_rest_stays() {
        let shipped = include_str!("../../../os/rootfs/usr/share/deck/deck.default.toml");
        let written = rme().write(shipped).unwrap();
        assert_eq!(DeckAudio::parse(&written), rme(), "parse ↔ write");
        assert_eq!(rme().write(&written).unwrap(), written, "writing again changes nothing");
        // every other key and every comment line of the shipped file is still there, in its order
        let kept: Vec<&str> = shipped.lines().filter(|l| !l.starts_with("audio_")).collect();
        let now: Vec<&str> = written.lines().collect();
        let mut at = 0;
        for line in kept {
            at += now[at..].iter().position(|l| *l == line).unwrap_or_else(|| panic!("lost: {line}\n{written}")) + 1;
        }
        // deck-session reads the plain keys with sed: one per line, before the tables, strings in double quotes
        assert!(written.contains("\naudio_host = \"asio\"\n") && written.contains("\naudio_buffer = 128\n"), "{written}");
        assert!(written.find("audio_device").unwrap() < written.find("[[audio_outlet]]").unwrap());
        let toml: toml::Value = toml::from_str(&written).unwrap();
        assert_eq!(toml["kiosk"].as_str(), Some("stage"));
        assert_eq!(toml["update_channel"].as_str(), Some("dev"));
    }

    #[test]
    fn a_key_keeps_its_comment_and_unset_keys_leave() {
        let text = "audio_buffer = 256 # the RME's setting\naudio_device = \"Fireface\"\n[[audio_outlet]]\ngroup = \"cue\"\nchannels = [3, 4]\n";
        let less = DeckAudio { audio_buffer: Some(64), ..DeckAudio::default() };
        let written = less.write(text).unwrap();
        assert!(written.contains("audio_buffer = 64 # the RME's setting"), "{written}");
        assert!(!written.contains("audio_device") && !written.contains("audio_outlet"), "{written}");
        assert_eq!(DeckAudio::parse(&written), less);
        assert_eq!(DeckAudio::default().write("").map(|t| DeckAudio::parse(&t)).unwrap(), DeckAudio::default(), "an empty file: the default buffer only");
        assert!(rme().write("this is = not toml [").is_err(), "a broken file is not overwritten");
    }

    #[test]
    fn a_dev_machines_file_keeps_its_map_comment() {
        let text = "# this PC\naudio_host = \"asio\"\naudio_device = \"Fireface\"\naudio_buffer = 128\nname = \"pc\"\n\n# where the groups play\n[[audio_outlet]]\ngroup = \"cue\"\nchannels = [3, 4]\n\n[[audio_outlet]]\ngroup = \"stem\"\nchannels = [5, 6]\n";
        let mut d = DeckAudio::parse(text);
        assert_eq!(d.write(text).unwrap(), text, "unchanged settings write the same file");
        d.audio_outlet[0].channels = vec![7, 8];
        d.audio_outlet.pop();
        let written = d.write(text).unwrap();
        assert!(written.contains("# where the groups play\n[[audio_outlet]]\ngroup = \"cue\"\nchannels = [7, 8]\n"), "{written}");
        assert!(!written.contains("stem") && written.starts_with("# this PC\naudio_host = \"asio\"\n") && written.contains("name = \"pc\""), "{written}");
    }
}
