//! Typed, read-only OneLibrary preview boundary.
//!
//! SQLCipher deliberately does not appear here. The application communicates with
//! the separately built reader process through a versioned, non-secret protocol.

use std::fmt;

/// Current synthetic schema profile. This is not a claim about vendor schemas.
pub const PROTOCOL_VERSION: u16 = 1;
pub const SCHEMA_PROFILE_V1: u16 = 1;

/// Source-scoped identifier; never interchangeable with an OxiDJ catalog ID.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(String);

impl SourceId {
    pub fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(Error::InvalidResult("invalid source-scoped identifier"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Supported raw track facts. Absence remains distinct from an empty string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Track {
    pub id: SourceId,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Playlist {
    pub id: SourceId,
    pub parent_id: Option<SourceId>,
    pub sibling_order: u32,
    pub name: String,
    /// Ordered track IDs; duplicate membership is retained when present in source.
    pub track_ids: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    pub protocol_version: u16,
    pub schema_profile: u16,
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidResult(&'static str),
    ReaderFailed,
    ProtocolMismatch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResult(message) => f.write_str(message),
            Self::ReaderFailed => f.write_str("OneLibrary reader failed"),
            Self::ProtocolMismatch => f.write_str("unsupported OneLibrary reader protocol"),
        }
    }
}

impl std::error::Error for Error {}

/// Decode the versioned reader response and validate all relationships before exposure.
pub fn decode_preview(payload: &[u8]) -> Result<Preview, Error> {
    let v: serde_json::Value =
        serde_json::from_slice(payload).map_err(|_| Error::ProtocolMismatch)?;
    let number = |key: &str| {
        v.get(key)
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| u16::try_from(n).ok())
            .ok_or(Error::ProtocolMismatch)
    };
    let text = |v: &serde_json::Value| v.as_str().map(str::to_owned).ok_or(Error::ProtocolMismatch);
    let maybe_text = |v: &serde_json::Value| {
        if v.is_null() {
            Ok(None)
        } else {
            text(v).map(Some)
        }
    };
    let sid = |v: &serde_json::Value| text(v).and_then(SourceId::new);
    let tracks = v
        .get("tracks")
        .and_then(serde_json::Value::as_array)
        .ok_or(Error::ProtocolMismatch)?
        .iter()
        .map(|t| {
            Ok(Track {
                id: sid(t.get("id").ok_or(Error::ProtocolMismatch)?)?,
                title: maybe_text(t.get("title").ok_or(Error::ProtocolMismatch)?)?,
                artist: maybe_text(t.get("artist").ok_or(Error::ProtocolMismatch)?)?,
                album: maybe_text(t.get("album").ok_or(Error::ProtocolMismatch)?)?,
            })
        })
        .collect::<Result<_, Error>>()?;
    let playlists = v
        .get("playlists")
        .and_then(serde_json::Value::as_array)
        .ok_or(Error::ProtocolMismatch)?
        .iter()
        .map(|p| {
            let parent = p.get("parent_id").ok_or(Error::ProtocolMismatch)?;
            let members = p
                .get("track_ids")
                .and_then(serde_json::Value::as_array)
                .ok_or(Error::ProtocolMismatch)?
                .iter()
                .map(sid)
                .collect::<Result<_, _>>()?;
            Ok(Playlist {
                id: sid(p.get("id").ok_or(Error::ProtocolMismatch)?)?,
                parent_id: if parent.is_null() {
                    None
                } else {
                    Some(sid(parent)?)
                },
                sibling_order: u32::try_from(
                    p.get("sibling_order")
                        .and_then(serde_json::Value::as_u64)
                        .ok_or(Error::ProtocolMismatch)?,
                )
                .map_err(|_| Error::ProtocolMismatch)?,
                name: text(p.get("name").ok_or(Error::ProtocolMismatch)?)?,
                track_ids: members,
            })
        })
        .collect::<Result<_, Error>>()?;
    validate(Preview {
        protocol_version: number("protocol_version")?,
        schema_profile: number("schema_profile")?,
        tracks,
        playlists,
    })
}

/// Encode a validated preview for the one-shot reader pipe.
pub fn encode_preview(preview: Preview) -> Result<Vec<u8>, Error> {
    let p = validate(preview)?;
    let ids = |xs: &[SourceId]| {
        xs.iter()
            .map(|id| serde_json::Value::String(id.0.clone()))
            .collect::<Vec<_>>()
    };
    let tracks = p
        .tracks
        .iter()
        .map(|t| serde_json::json!({"id":t.id.0,"title":t.title,"artist":t.artist,"album":t.album}))
        .collect::<Vec<_>>();
    let playlists = p.playlists.iter().map(|x| serde_json::json!({"id":x.id.0,"parent_id":x.parent_id.as_ref().map(|i| i.0.clone()),"sibling_order":x.sibling_order,"name":x.name,"track_ids":ids(&x.track_ids)})).collect::<Vec<_>>();
    serde_json::to_vec(&serde_json::json!({"protocol_version":p.protocol_version,"schema_profile":p.schema_profile,"tracks":tracks,"playlists":playlists})).map_err(|_| Error::ProtocolMismatch)
}

/// Reject malformed cross-references before a complete preview is returned.
pub fn validate(preview: Preview) -> Result<Preview, Error> {
    if preview.protocol_version != PROTOCOL_VERSION || preview.schema_profile != SCHEMA_PROFILE_V1 {
        return Err(Error::ProtocolMismatch);
    }
    let track_ids: std::collections::HashSet<_> =
        preview.tracks.iter().map(|track| &track.id).collect();
    if track_ids.len() != preview.tracks.len() {
        return Err(Error::InvalidResult("duplicate track identifiers"));
    }
    let playlist_ids: std::collections::HashSet<_> = preview
        .playlists
        .iter()
        .map(|playlist| &playlist.id)
        .collect();
    if playlist_ids.len() != preview.playlists.len() {
        return Err(Error::InvalidResult("duplicate playlist identifiers"));
    }
    for playlist in &preview.playlists {
        if playlist
            .parent_id
            .as_ref()
            .is_some_and(|id| !playlist_ids.contains(id))
        {
            return Err(Error::InvalidResult("playlist parent does not exist"));
        }
        if playlist.track_ids.iter().any(|id| !track_ids.contains(id)) {
            return Err(Error::InvalidResult("playlist track does not exist"));
        }
    }
    // Parent cycles are rejected with a bounded walk over the finite node set.
    for playlist in &preview.playlists {
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(&playlist.id);
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(Error::InvalidResult("playlist parent cycle"));
            }
            current = preview
                .playlists
                .iter()
                .find(|candidate| &candidate.id == id)
                .and_then(|candidate| candidate.parent_id.as_ref());
        }
    }
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> SourceId {
        SourceId::new(value.to_owned()).unwrap()
    }

    #[test]
    fn validates_nested_ordered_memberships_and_source_scoped_ids() {
        let preview = Preview {
            protocol_version: PROTOCOL_VERSION,
            schema_profile: SCHEMA_PROFILE_V1,
            tracks: vec![Track {
                id: id("track:1"),
                title: Some(String::new()),
                artist: None,
                album: Some("Album".into()),
            }],
            playlists: vec![
                Playlist {
                    id: id("playlist:root"),
                    parent_id: None,
                    sibling_order: 0,
                    name: "Root".into(),
                    track_ids: vec![id("track:1"), id("track:1")],
                },
                Playlist {
                    id: id("playlist:child"),
                    parent_id: Some(id("playlist:root")),
                    sibling_order: 0,
                    name: "Child".into(),
                    track_ids: vec![],
                },
            ],
        };
        let checked = validate(preview).unwrap();
        assert_eq!(checked.tracks[0].title.as_deref(), Some(""));
        assert_eq!(checked.tracks[0].artist, None);
        assert_eq!(checked.playlists[0].track_ids.len(), 2);
    }

    #[test]
    fn rejects_dangling_membership_duplicate_ids_and_parent_cycles() {
        let base = Preview {
            protocol_version: PROTOCOL_VERSION,
            schema_profile: SCHEMA_PROFILE_V1,
            tracks: vec![Track {
                id: id("t"),
                title: None,
                artist: None,
                album: None,
            }],
            playlists: vec![Playlist {
                id: id("p"),
                parent_id: None,
                sibling_order: 0,
                name: "P".into(),
                track_ids: vec![id("missing")],
            }],
        };
        assert_eq!(
            validate(base.clone()),
            Err(Error::InvalidResult("playlist track does not exist"))
        );
        let cycle = Preview {
            playlists: vec![Playlist {
                id: id("p"),
                parent_id: Some(id("p")),
                sibling_order: 0,
                name: "P".into(),
                track_ids: vec![],
            }],
            ..base
        };
        assert_eq!(
            validate(cycle),
            Err(Error::InvalidResult("playlist parent cycle"))
        );
    }
}
