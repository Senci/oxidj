//! Reference-only imported DJ-library metadata: tracks, playlist tree and ordered
//! memberships. Vendor-neutral by design: callers (the application layer) map
//! adapter previews into these types; this crate never depends on an adapter.
//! No audio is read or written.

use super::{Catalog, CatalogError};

/// One imported track's metadata; `location` is kept as supplied by the caller.
/// Optional fields are `None` when the source supplied no value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryTrack {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub tempo_hundredths_bpm: u32,
    pub duration_seconds: u32,
    pub location: String,
    pub genre: Option<String>,
    pub label: Option<String>,
    /// Musical key as text (e.g. a key name or Camelot code), stored verbatim.
    pub key: Option<String>,
    pub comments: Option<String>,
    /// `YYYY-MM-DD`.
    pub date_added: Option<String>,
    pub year: Option<u32>,
    /// Kilobits per second.
    pub bit_rate: Option<u32>,
    /// Hertz.
    pub sample_rate: Option<u32>,
    /// Bytes.
    pub file_size: Option<u64>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    /// 1..=5.
    pub rating_stars: Option<u8>,
}

/// A playlist-tree node. Children and members are ordered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryNode {
    Folder {
        name: String,
        children: Vec<LibraryNode>,
    },
    /// `members` are indexes into [`LibrarySnapshot::tracks`]; duplicates are retained.
    Playlist { name: String, members: Vec<usize> },
}

/// A complete imported library: ordered tracks plus the ordered root nodes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibrarySnapshot {
    pub tracks: Vec<LibraryTrack>,
    pub roots: Vec<LibraryNode>,
}

/// Aggregate counts of a snapshot (folders exclude the implicit root).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryCounts {
    pub tracks: usize,
    pub folders: usize,
    pub playlists: usize,
    pub memberships: usize,
}

impl LibrarySnapshot {
    pub fn counts(&self) -> LibraryCounts {
        fn walk(nodes: &[LibraryNode], counts: &mut LibraryCounts) {
            for node in nodes {
                match node {
                    LibraryNode::Folder { children, .. } => {
                        counts.folders += 1;
                        walk(children, counts);
                    }
                    LibraryNode::Playlist { members, .. } => {
                        counts.playlists += 1;
                        counts.memberships += members.len();
                    }
                }
            }
        }
        let mut counts = LibraryCounts {
            tracks: self.tracks.len(),
            ..LibraryCounts::default()
        };
        walk(&self.roots, &mut counts);
        counts
    }
}

fn to_i64(value: usize) -> Result<i64, CatalogError> {
    i64::try_from(value).map_err(|_| CatalogError::InvalidRecord("library index exceeds range"))
}

impl Catalog {
    /// Store a complete imported library in one transaction. Fails closed (and
    /// stores nothing) when the catalog already holds an imported library, a
    /// membership references a missing track, or the track cap is exceeded.
    pub fn import_library(
        &mut self,
        snapshot: &LibrarySnapshot,
    ) -> Result<LibraryCounts, CatalogError> {
        if snapshot.tracks.len() > 10_000 {
            return Err(CatalogError::TrackLimit);
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM library_tracks) OR EXISTS(SELECT 1 FROM library_nodes)",
            [],
            |row| row.get(0),
        )?;
        if existing {
            return Err(CatalogError::InvalidRecord(
                "catalog already holds an imported library",
            ));
        }
        for (position, track) in snapshot.tracks.iter().enumerate() {
            tx.execute(
                "INSERT INTO library_tracks(position,title,artist,album,tempo_hundredths_bpm,\
                 duration_seconds,location,genre,label,musical_key,comments,date_added,year,\
                 bit_rate,sample_rate,file_size,track_number,disc_number,rating_stars) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
                rusqlite::params![
                    to_i64(position)?,
                    track.title,
                    track.artist,
                    track.album,
                    track.tempo_hundredths_bpm,
                    track.duration_seconds,
                    track.location,
                    track.genre,
                    track.label,
                    track.key,
                    track.comments,
                    track.date_added,
                    track.year,
                    track.bit_rate,
                    track.sample_rate,
                    track
                        .file_size
                        .map(|size| i64::try_from(size)
                            .map_err(|_| CatalogError::InvalidRecord("file size exceeds range")))
                        .transpose()?,
                    track.track_number,
                    track.disc_number,
                    track.rating_stars
                ],
            )?;
        }
        let mut next_id = 1_i64;
        fn insert(
            tx: &rusqlite::Transaction<'_>,
            nodes: &[LibraryNode],
            parent: Option<i64>,
            next_id: &mut i64,
            track_count: usize,
        ) -> Result<(), CatalogError> {
            for (order, node) in nodes.iter().enumerate() {
                let id = *next_id;
                *next_id += 1;
                let (name, is_folder) = match node {
                    LibraryNode::Folder { name, .. } => (name, true),
                    LibraryNode::Playlist { name, .. } => (name, false),
                };
                tx.execute(
                    "INSERT INTO library_nodes(id,parent_id,sibling_order,name,is_folder) \
                     VALUES(?1,?2,?3,?4,?5)",
                    rusqlite::params![id, parent, to_i64(order)?, name, is_folder],
                )?;
                match node {
                    LibraryNode::Folder { children, .. } => {
                        insert(tx, children, Some(id), next_id, track_count)?;
                    }
                    LibraryNode::Playlist { members, .. } => {
                        for (position, &track) in members.iter().enumerate() {
                            if track >= track_count {
                                return Err(CatalogError::InvalidRecord(
                                    "playlist member references a missing track",
                                ));
                            }
                            tx.execute(
                                "INSERT INTO library_members(node_id,position,track_position) \
                                 VALUES(?1,?2,?3)",
                                rusqlite::params![id, to_i64(position)?, to_i64(track)?],
                            )?;
                        }
                    }
                }
            }
            Ok(())
        }
        insert(
            &tx,
            &snapshot.roots,
            None,
            &mut next_id,
            snapshot.tracks.len(),
        )?;
        tx.commit()?;
        Ok(snapshot.counts())
    }

    /// Read the stored imported library back in stored order.
    pub fn library_snapshot(&self) -> Result<LibrarySnapshot, CatalogError> {
        let mut statement = self.connection.prepare(
            "SELECT position,title,artist,album,tempo_hundredths_bpm,duration_seconds,location,\
             genre,label,musical_key,comments,date_added,year,bit_rate,sample_rate,file_size,\
             track_number,disc_number,rating_stars FROM library_tracks ORDER BY position",
        )?;
        let mut tracks = Vec::new();
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                LibraryTrack {
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    album: row.get(3)?,
                    tempo_hundredths_bpm: row.get(4)?,
                    duration_seconds: row.get(5)?,
                    location: row.get(6)?,
                    genre: row.get(7)?,
                    label: row.get(8)?,
                    key: row.get(9)?,
                    comments: row.get(10)?,
                    date_added: row.get(11)?,
                    year: row.get(12)?,
                    bit_rate: row.get(13)?,
                    sample_rate: row.get(14)?,
                    file_size: row
                        .get::<_, Option<i64>>(15)?
                        .map(u64::try_from)
                        .transpose()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    track_number: row.get(16)?,
                    disc_number: row.get(17)?,
                    rating_stars: row.get(18)?,
                },
            ))
        })?;
        for row in rows {
            let (position, track) = row?;
            if position != to_i64(tracks.len())? {
                return Err(CatalogError::InvalidRecord(
                    "library track positions are not dense",
                ));
            }
            tracks.push(track);
        }
        let mut statement = self.connection.prepare(
            "SELECT id,parent_id,name,is_folder FROM library_nodes ORDER BY parent_id,sibling_order",
        )?;
        let nodes: Vec<(i64, Option<i64>, String, bool)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<_, _>>()?;
        let mut statement = self.connection.prepare(
            "SELECT track_position FROM library_members WHERE node_id=?1 ORDER BY position",
        )?;
        fn build(
            nodes: &[(i64, Option<i64>, String, bool)],
            parent: Option<i64>,
            statement: &mut rusqlite::Statement<'_>,
            track_count: usize,
            depth: usize,
        ) -> Result<Vec<LibraryNode>, CatalogError> {
            if depth > 256 {
                return Err(CatalogError::InvalidRecord("library tree is too deep"));
            }
            let mut out = Vec::new();
            for (id, _, name, is_folder) in nodes.iter().filter(|node| node.1 == parent) {
                if *is_folder {
                    out.push(LibraryNode::Folder {
                        name: name.clone(),
                        children: build(nodes, Some(*id), statement, track_count, depth + 1)?,
                    });
                } else {
                    let members = statement
                        .query_map([id], |row| row.get::<_, i64>(0))?
                        .map(|value| {
                            usize::try_from(value?)
                                .ok()
                                .filter(|&track| track < track_count)
                                .ok_or(CatalogError::InvalidRecord(
                                    "playlist member references a missing track",
                                ))
                        })
                        .collect::<Result<_, _>>()?;
                    out.push(LibraryNode::Playlist {
                        name: name.clone(),
                        members,
                    });
                }
            }
            Ok(out)
        }
        let roots = build(&nodes, None, &mut statement, tracks.len(), 0)?;
        Ok(LibrarySnapshot { tracks, roots })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "oxidj-imported-library-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn track(title: &str) -> LibraryTrack {
        LibraryTrack {
            title: title.into(),
            artist: None,
            album: Some("Album".into()),
            tempo_hundredths_bpm: 12_800,
            duration_seconds: 180,
            location: format!("file://localhost/{title}.wav"),
            ..LibraryTrack::default()
        }
    }

    fn snapshot() -> LibrarySnapshot {
        let full = LibraryTrack {
            genre: Some("Genre".into()),
            label: Some("Label".into()),
            key: Some("8A".into()),
            comments: Some("".into()),
            date_added: Some("2026-01-02".into()),
            year: Some(1999),
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            file_size: Some(u64::from(u32::MAX) + 1),
            track_number: Some(3),
            disc_number: Some(1),
            rating_stars: Some(5),
            ..track("A")
        };
        LibrarySnapshot {
            tracks: vec![full, track("B"), track("C")],
            roots: vec![
                LibraryNode::Playlist {
                    name: "Zeta".into(),
                    members: vec![2, 0, 2],
                },
                LibraryNode::Folder {
                    name: "Folder".into(),
                    children: vec![
                        LibraryNode::Playlist {
                            name: "Inner".into(),
                            members: vec![1],
                        },
                        LibraryNode::Folder {
                            name: "Empty".into(),
                            children: vec![],
                        },
                        LibraryNode::Playlist {
                            name: "Alpha".into(),
                            members: vec![],
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn imported_library_round_trips_order_duplicates_and_counts() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        let counts = catalog.import_library(&snapshot()).unwrap();
        assert_eq!(
            counts,
            LibraryCounts {
                tracks: 3,
                folders: 2,
                playlists: 3,
                memberships: 4
            }
        );
        drop(catalog);
        let reopened = Catalog::open(&path).unwrap();
        assert_eq!(reopened.library_snapshot().unwrap(), snapshot());
    }

    #[test]
    fn second_import_and_missing_member_fail_closed_without_partial_rows() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        let mut bad = snapshot();
        bad.roots.push(LibraryNode::Playlist {
            name: "Broken".into(),
            members: vec![3],
        });
        assert!(matches!(
            catalog.import_library(&bad),
            Err(CatalogError::InvalidRecord(
                "playlist member references a missing track"
            ))
        ));
        assert_eq!(
            catalog.library_snapshot().unwrap(),
            LibrarySnapshot::default()
        );
        catalog.import_library(&snapshot()).unwrap();
        assert!(matches!(
            catalog.import_library(&snapshot()),
            Err(CatalogError::InvalidRecord(
                "catalog already holds an imported library"
            ))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), snapshot());
    }
}
