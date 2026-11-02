//! Inspection and bounded byte-preserving edits for a Rekordbox XML subset.
use quick_xml::{Reader, events::Event};
use std::{collections::HashSet, fs, path::Path};

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub comments: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub composer: Option<String>,
    pub grouping: Option<String>,
    pub remixer: Option<String>,
    pub mix: Option<String>,
    pub tonality: Option<String>,
    pub kind: Option<String>,
    pub year: Option<String>,
    pub size: Option<String>,
    pub disc_number: Option<String>,
    pub track_number: Option<String>,
    pub total_time: Option<String>,
    pub bit_rate: Option<String>,
    pub sample_rate: Option<String>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub last_played: Option<String>,
    pub play_count: Option<String>,
    pub rating: Option<String>,
    pub colour: Option<String>,
    pub average_bpm: Option<String>,
    pub location: String,
    pub tempos: Vec<Tempo>,
    pub marks: Vec<PositionMark>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tempo {
    pub start_seconds: String,
    pub bpm: String,
    pub meter: String,
    pub beat: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PositionMark {
    pub name: String,
    pub kind: String,
    pub start_seconds: String,
    pub number: String,
    pub end_seconds: Option<String>,
    pub red: Option<String>,
    pub green: Option<String>,
    pub blue: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Folder {
    pub ordinal: usize,
    pub path: Vec<String>,
    pub name: String,
    pub count: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub path: Vec<String>,
    pub name: String,
    pub tracks: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistNodeMetadata {
    pub path: Vec<String>,
    /// Raw optional NODE Entries attribute; None means absent, Some("") means present-empty.
    pub entries: Option<String>,
    /// Raw optional NODE KeyType attribute; None means absent, Some("") means present-empty.
    pub key_type: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub tracks: Vec<Track>,
    pub folders: Vec<Folder>,
    pub playlists: Vec<Playlist>,
    pub playlist_nodes: Vec<PlaylistNodeMetadata>,
    pub unsupported: Vec<String>,
}

#[derive(Debug)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

mod anlz;
mod edit;
mod library_xml;
mod pdb;
mod playlist;
mod xml;

pub use anlz::*;
pub use edit::*;
pub use library_xml::*;
pub use pdb::*;
pub use playlist::*;
pub use xml::*;

#[cfg(test)]
mod tests;
