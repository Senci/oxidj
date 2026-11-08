//! Reference-only imported DJ-library metadata: tracks, playlist tree and ordered
//! memberships. Vendor-neutral by design: callers (the application layer) map
//! adapter previews into these types; this crate never depends on an adapter.
//! No audio is read or written.

use super::{Catalog, CatalogError, VendorWaveform};

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
    /// Ordered beat grid (empty = none).
    pub beat_grid: Vec<LibraryBeat>,
    /// Ordered cues and loops (empty = none).
    pub cues: Vec<LibraryCue>,
    /// Vendor-derived waveforms (empty = none), at most one per kind, in
    /// [`VendorWaveform`] declaration order. Never computed peaks.
    pub vendor_waveforms: Vec<VendorWaveform>,
}

/// One beat-grid point as stored by the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryBeat {
    pub time_ms: u32,
    /// Positive.
    pub tempo_hundredths_bpm: u32,
    /// Position in a 4/4 bar, 1..=4.
    pub beat_in_bar: u8,
}

/// One cue point or loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryCue {
    /// `None` = memory cue; `Some(n)` = hot cue `n` (1 = A).
    pub hot_cue: Option<u32>,
    pub start_ms: u32,
    /// `Some` = loop end (after `start_ms`).
    pub end_ms: Option<u32>,
    /// Empty when the source stores no name.
    pub name: String,
    pub colour: Option<[u8; 3]>,
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

/// The fields required to render imported playlist navigation. Unlike
/// [`LibrarySnapshot`], this deliberately excludes analysis and vendor blobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryNavigationTrack {
    pub title: String,
    pub location: String,
}

/// Lightweight imported-library tree used by the UI navigation panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryNavigation {
    pub tracks: Vec<LibraryNavigationTrack>,
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

/// Playlist names must be nonblank XML 1.0 text supported by the library exporter.
/// Like the exporter, reject controls other than TAB/LF/CR before mutation.
/// Valid names are retained verbatim (including leading/trailing whitespace).
pub fn validate_library_playlist_name(name: &str) -> Result<(), CatalogError> {
    if name.trim().is_empty() {
        return Err(CatalogError::InvalidRecord(
            "playlist name must not be blank",
        ));
    }
    if !name.chars().all(|c| {
        (!c.is_control() || matches!(c, '\t' | '\n' | '\r'))
            && matches!(c,
                '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' |
                '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'
            )
    }) {
        return Err(CatalogError::InvalidRecord(
            "playlist name contains invalid XML 1.0 text",
        ));
    }
    Ok(())
}

impl Catalog {
    /// Append one root playlist atomically. Members are zero-based imported-library
    /// positions, NOT local audio IDs. Order and duplicates are retained; existing
    /// records are never changed. Return the member count only after commit.
    pub fn create_library_playlist(
        &mut self,
        name: &str,
        members: &[usize],
    ) -> Result<usize, CatalogError> {
        validate_library_playlist_name(name)?;
        if members.is_empty() {
            return Err(CatalogError::InvalidRecord(
                "playlist membership must not be empty",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let count: i64 =
            tx.query_row("SELECT COUNT(*) FROM library_tracks", [], |row| row.get(0))?;
        if count == 0 {
            return Err(CatalogError::InvalidRecord(
                "catalog holds no imported tracks",
            ));
        }
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM library_nodes WHERE parent_id IS NULL AND name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if duplicate {
            return Err(CatalogError::InvalidRecord(
                "a root node with this name already exists",
            ));
        }
        // Validate every reference before inserting anything, under the same write lock.
        for &member in members {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM library_tracks WHERE position = ?1)",
                [to_i64(member)?],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(CatalogError::InvalidRecord(
                    "playlist member references a missing track",
                ));
            }
        }
        let id: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM library_nodes",
            [],
            |row| row.get(0),
        )?;
        let order: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sibling_order), -1) + 1 FROM library_nodes WHERE parent_id IS NULL",
            [], |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO library_nodes(id,parent_id,sibling_order,name,is_folder) VALUES(?1,NULL,?2,?3,0)",
            rusqlite::params![id, order, name],
        )?;
        for (position, &member) in members.iter().enumerate() {
            tx.execute(
                "INSERT INTO library_members(node_id,position,track_position) VALUES(?1,?2,?3)",
                rusqlite::params![id, to_i64(position)?, to_i64(member)?],
            )?;
        }
        tx.commit()?;
        Ok(members.len())
    }

    /// Append imported-track positions to one uniquely named root playlist.
    /// Existing and requested order, including duplicates, is preserved.
    pub fn append_library_playlist_members(
        &mut self,
        name: &str,
        members: &[usize],
    ) -> Result<usize, CatalogError> {
        validate_library_playlist_name(name)?;
        if members.is_empty() {
            return Err(CatalogError::InvalidRecord(
                "playlist membership must not be empty",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let track_count: i64 =
            tx.query_row("SELECT COUNT(*) FROM library_tracks", [], |row| row.get(0))?;
        if track_count == 0 {
            return Err(CatalogError::InvalidRecord(
                "catalog holds no imported tracks",
            ));
        }
        let mut stmt = tx.prepare(
            "SELECT id, is_folder FROM library_nodes WHERE parent_id IS NULL AND name = ?1",
        )?;
        let mut rows = stmt.query([name])?;
        let target = rows
            .next()?
            .map(|row| -> Result<_, rusqlite::Error> {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .transpose()?;
        if rows.next()?.is_some() {
            return Err(CatalogError::InvalidRecord(
                "root playlist name is ambiguous",
            ));
        }
        drop(rows);
        drop(stmt);
        let Some((node_id, is_folder)) = target else {
            return Err(CatalogError::InvalidRecord("root playlist does not exist"));
        };
        if is_folder != 0 {
            return Err(CatalogError::InvalidRecord(
                "root name refers to a folder, not a playlist",
            ));
        }
        for &member in members {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM library_tracks WHERE position = ?1)",
                [to_i64(member)?],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(CatalogError::InvalidRecord(
                    "playlist member references a missing track",
                ));
            }
        }
        let position_start: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM library_members WHERE node_id = ?1",
            [node_id],
            |row| row.get(0),
        )?;
        for (offset, &member) in members.iter().enumerate() {
            let position =
                position_start
                    .checked_add(to_i64(offset)?)
                    .ok_or(CatalogError::InvalidRecord(
                        "playlist membership position exceeds range",
                    ))?;
            tx.execute(
                "INSERT INTO library_members(node_id,position,track_position) VALUES(?1,?2,?3)",
                rusqlite::params![node_id, position, to_i64(member)?],
            )?;
        }
        tx.commit()?;
        Ok(members.len())
    }

    /// Reorder exactly the existing membership of one unique root playlist.
    pub fn reorder_library_playlist_members(
        &mut self,
        name: &str,
        members: &[usize],
    ) -> Result<usize, CatalogError> {
        validate_library_playlist_name(name)?;
        if members.is_empty() {
            return Err(CatalogError::InvalidRecord(
                "playlist membership must not be empty",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut stmt = tx.prepare(
            "SELECT id, is_folder FROM library_nodes WHERE parent_id IS NULL AND name = ?1",
        )?;
        let mut rows = stmt.query([name])?;
        let target = rows
            .next()?
            .map(|r| Ok::<_, rusqlite::Error>((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .transpose()?;
        if rows.next()?.is_some() {
            return Err(CatalogError::InvalidRecord(
                "root playlist name is ambiguous",
            ));
        }
        drop(rows);
        drop(stmt);
        let Some((id, folder)) = target else {
            return Err(CatalogError::InvalidRecord("root playlist does not exist"));
        };
        if folder != 0 {
            return Err(CatalogError::InvalidRecord(
                "root name refers to a folder, not a playlist",
            ));
        }
        let mut stmt = tx.prepare(
            "SELECT track_position FROM library_members WHERE node_id = ?1 ORDER BY position",
        )?;
        let existing = stmt
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        let requested = members
            .iter()
            .map(|&m| to_i64(m))
            .collect::<Result<Vec<_>, _>>()?;
        let mut a = existing.clone();
        let mut b = requested.clone();
        a.sort_unstable();
        b.sort_unstable();
        if a != b {
            return Err(CatalogError::InvalidRecord(
                "playlist membership counts differ",
            ));
        }
        tx.execute("DELETE FROM library_members WHERE node_id = ?1", [id])?;
        for (position, member) in requested.iter().enumerate() {
            tx.execute(
                "INSERT INTO library_members(node_id,position,track_position) VALUES(?1,?2,?3)",
                rusqlite::params![id, to_i64(position)?, member],
            )?;
        }
        tx.commit()?;
        Ok(members.len())
    }

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
        {
            let mut beat_insert = tx.prepare(
                "INSERT INTO library_beats(track_position,position,time_ms,tempo_hundredths_bpm,\
                 beat_in_bar) VALUES(?1,?2,?3,?4,?5)",
            )?;
            let mut cue_insert = tx.prepare(
                "INSERT INTO library_cues(track_position,position,hot_cue,start_ms,end_ms,name,\
                 red,green,blue) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            )?;
            let mut waveform_insert = tx.prepare(
                "INSERT INTO library_vendor_waveforms(track_position,kind,data) VALUES(?1,?2,?3)",
            )?;
            for (position, track) in snapshot.tracks.iter().enumerate() {
                for (index, beat) in track.beat_grid.iter().enumerate() {
                    beat_insert.execute(rusqlite::params![
                        to_i64(position)?,
                        to_i64(index)?,
                        beat.time_ms,
                        beat.tempo_hundredths_bpm,
                        beat.beat_in_bar
                    ])?;
                }
                if !track
                    .vendor_waveforms
                    .windows(2)
                    .all(|w| w[0].order() < w[1].order())
                {
                    return Err(CatalogError::InvalidRecord(
                        "vendor waveforms are duplicated or out of order",
                    ));
                }
                for waveform in &track.vendor_waveforms {
                    waveform_insert.execute(rusqlite::params![
                        to_i64(position)?,
                        waveform.kind(),
                        waveform.encode()?
                    ])?;
                }
                for (index, cue) in track.cues.iter().enumerate() {
                    let colour = cue.colour.map(|[r, g, b]| (r, g, b));
                    cue_insert.execute(rusqlite::params![
                        to_i64(position)?,
                        to_i64(index)?,
                        cue.hot_cue,
                        cue.start_ms,
                        cue.end_ms,
                        cue.name,
                        colour.map(|c| c.0),
                        colour.map(|c| c.1),
                        colour.map(|c| c.2)
                    ])?;
                }
            }
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

    /// Read only the fields needed to render imported-library navigation.
    /// This does not load beat grids, cues, or vendor waveform payloads.
    pub fn library_navigation(&self) -> Result<LibraryNavigation, CatalogError> {
        let mut statement = self
            .connection
            .prepare("SELECT position,title,location FROM library_tracks ORDER BY position")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                LibraryNavigationTrack {
                    title: row.get(1)?,
                    location: row.get(2)?,
                },
            ))
        })?;
        let mut tracks = Vec::new();
        for row in rows {
            let (position, track) = row?;
            if position != to_i64(tracks.len())? {
                return Err(CatalogError::InvalidRecord(
                    "library track positions are not dense",
                ));
            }
            tracks.push(track);
        }
        let nodes: Vec<(i64, Option<i64>, String, bool)> = self.connection
            .prepare("SELECT id,parent_id,name,is_folder FROM library_nodes ORDER BY parent_id,sibling_order")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .collect::<Result<_, _>>()?;
        let mut members = self.connection.prepare(
            "SELECT track_position FROM library_members WHERE node_id=?1 ORDER BY position",
        )?;
        fn build(
            nodes: &[(i64, Option<i64>, String, bool)],
            parent: Option<i64>,
            members: &mut rusqlite::Statement<'_>,
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
                        children: build(nodes, Some(*id), members, track_count, depth + 1)?,
                    });
                } else {
                    let ordered = members
                        .query_map([id], |row| row.get::<_, i64>(0))?
                        .map(|value| {
                            usize::try_from(value?)
                                .ok()
                                .filter(|&position| position < track_count)
                                .ok_or(CatalogError::InvalidRecord(
                                    "playlist member references a missing track",
                                ))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    out.push(LibraryNode::Playlist {
                        name: name.clone(),
                        members: ordered,
                    });
                }
            }
            Ok(out)
        }
        let roots = build(&nodes, None, &mut members, tracks.len(), 0)?;
        Ok(LibraryNavigation { tracks, roots })
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
                    beat_grid: Vec::new(),
                    cues: Vec::new(),
                    vendor_waveforms: Vec::new(),
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
        fn dense(
            track: i64,
            position: i64,
            tracks: &mut [LibraryTrack],
            beats: bool,
        ) -> Result<&mut LibraryTrack, CatalogError> {
            let slot = usize::try_from(track)
                .ok()
                .and_then(|t| tracks.get_mut(t))
                .ok_or(CatalogError::InvalidRecord(
                    "library analysis references a missing track",
                ))?;
            let len = if beats {
                slot.beat_grid.len()
            } else {
                slot.cues.len()
            };
            if position != to_i64(len)? {
                return Err(CatalogError::InvalidRecord(
                    "library analysis positions are not dense",
                ));
            }
            Ok(slot)
        }
        let mut statement = self.connection.prepare(
            "SELECT track_position,position,time_ms,tempo_hundredths_bpm,beat_in_bar \
             FROM library_beats ORDER BY track_position,position",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                LibraryBeat {
                    time_ms: row.get(2)?,
                    tempo_hundredths_bpm: row.get(3)?,
                    beat_in_bar: row.get(4)?,
                },
            ))
        })?;
        for row in rows {
            let (track, position, beat) = row?;
            dense(track, position, &mut tracks, true)?
                .beat_grid
                .push(beat);
        }
        let mut statement = self.connection.prepare(
            "SELECT track_position,position,hot_cue,start_ms,end_ms,name,red,green,blue \
             FROM library_cues ORDER BY track_position,position",
        )?;
        let rows = statement.query_map([], |row| {
            let colour = match (row.get(6)?, row.get(7)?, row.get(8)?) {
                (Some(r), Some(g), Some(b)) => Some([r, g, b]),
                _ => None,
            };
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                LibraryCue {
                    hot_cue: row.get(2)?,
                    start_ms: row.get(3)?,
                    end_ms: row.get(4)?,
                    name: row.get(5)?,
                    colour,
                },
            ))
        })?;
        for row in rows {
            let (track, position, cue) = row?;
            dense(track, position, &mut tracks, false)?.cues.push(cue);
        }
        let mut statement = self.connection.prepare(
            "SELECT track_position,kind,data FROM library_vendor_waveforms \
             ORDER BY track_position,kind",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        for row in rows {
            let (track, kind, data) = row?;
            let waveform = VendorWaveform::decode(&kind, &data)?;
            let slot = usize::try_from(track)
                .ok()
                .and_then(|t| tracks.get_mut(t))
                .ok_or(CatalogError::InvalidRecord(
                    "vendor waveform references a missing track",
                ))?;
            slot.vendor_waveforms.push(waveform);
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

    /// Imported (vendor-derived) beat grid of the library track matched by
    /// `library_position_for_locations` (exact, else unambiguous
    /// device-relative suffix), in
    /// stored beat order. Empty when no such track or no beats. Read-only; never
    /// touches computed peaks.
    pub fn beat_grid_for_locations(
        &self,
        locations: &[&str],
    ) -> Result<Vec<LibraryBeat>, CatalogError> {
        let Some(position) = self.library_position_for_locations(locations)? else {
            return Ok(Vec::new());
        };
        let mut statement = self.connection.prepare(
            "SELECT time_ms,tempo_hundredths_bpm,beat_in_bar FROM library_beats \
             WHERE track_position=?1 ORDER BY position",
        )?;
        let beats = statement
            .query_map([position], |row| {
                Ok(LibraryBeat {
                    time_ms: row.get(0)?,
                    tempo_hundredths_bpm: row.get(1)?,
                    beat_in_bar: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(beats)
    }

    /// Source-supplied musical key of the imported library track matched like
    /// [`Self::beat_grid_for_locations`]. Returns `None` when no track or key
    /// exists, preserving a present empty string as `Some("")`. Read-only.
    pub fn key_for_locations(&self, locations: &[&str]) -> Result<Option<String>, CatalogError> {
        let Some(position) = self.library_position_for_locations(locations)? else {
            return Ok(None);
        };
        let key = self.connection.query_row(
            "SELECT musical_key FROM library_tracks WHERE position=?1",
            [position],
            |row| row.get::<_, Option<String>>(0),
        )?;
        Ok(key)
    }

    /// Imported (vendor-derived) cue points and loops of the library track
    /// matched like [`Self::beat_grid_for_locations`], in stored order.
    /// Empty when no such track or no cues. Read-only.
    pub fn cues_for_locations(&self, locations: &[&str]) -> Result<Vec<LibraryCue>, CatalogError> {
        let Some(position) = self.library_position_for_locations(locations)? else {
            return Ok(Vec::new());
        };
        let mut statement = self.connection.prepare(
            "SELECT hot_cue,start_ms,end_ms,name,red,green,blue FROM library_cues \
             WHERE track_position=?1 ORDER BY position",
        )?;
        let cues = statement
            .query_map([position], |row| {
                let colour = match (row.get(4)?, row.get(5)?, row.get(6)?) {
                    (Some(r), Some(g), Some(b)) => Some([r, g, b]),
                    _ => None,
                };
                Ok(LibraryCue {
                    hot_cue: row.get(0)?,
                    start_ms: row.get(1)?,
                    end_ms: row.get(2)?,
                    name: row.get(3)?,
                    colour,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(cues)
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
            beat_grid: vec![
                LibraryBeat {
                    time_ms: 0,
                    tempo_hundredths_bpm: 12_000,
                    beat_in_bar: 4,
                },
                LibraryBeat {
                    time_ms: 500,
                    tempo_hundredths_bpm: 12_850,
                    beat_in_bar: 1,
                },
            ],
            cues: vec![
                LibraryCue {
                    hot_cue: None,
                    start_ms: 0,
                    end_ms: None,
                    name: String::new(),
                    colour: None,
                },
                LibraryCue {
                    hot_cue: Some(8),
                    start_ms: 1_000,
                    end_ms: Some(3_000),
                    name: "Loop & <x>".into(),
                    colour: Some([0, 128, 255]),
                },
            ],
            vendor_waveforms: vec![
                VendorWaveform::RekordboxDetail(vec![[31, 7], [0, 0], [12, 3]]),
                VendorWaveform::RekordboxColourDetail(vec![[7, 0, 3, 31]]),
            ],
            ..track("A")
        };
        let cue_only = LibraryTrack {
            cues: vec![LibraryCue {
                hot_cue: Some(1),
                start_ms: 7,
                end_ms: None,
                name: "Drop".into(),
                colour: None,
            }],
            ..track("C")
        };
        LibrarySnapshot {
            tracks: vec![full, track("B"), cue_only],
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
    fn navigation_snapshot_reads_only_titles_locations_and_ordered_memberships() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        catalog.import_library(&snapshot()).unwrap();
        catalog
            .connection
            .execute(
                "UPDATE library_vendor_waveforms SET data=x'00' WHERE track_position=0",
                [],
            )
            .unwrap();

        let navigation = catalog.library_navigation().unwrap();
        assert_eq!(
            navigation
                .tracks
                .iter()
                .map(|track| track.title.as_str())
                .collect::<Vec<_>>(),
            ["A", "B", "C"]
        );
        assert_eq!(navigation.tracks[0].location, "file://localhost/A.wav");
        assert_eq!(navigation.roots, snapshot().roots);
        assert!(
            catalog.library_snapshot().is_err(),
            "full snapshot parses vendor waveform blobs"
        );
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

    #[test]
    fn create_library_playlist_reopens_without_changing_existing_records() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        let mut expected = snapshot();
        catalog.import_library(&expected).unwrap();
        let nodes_before: Vec<(i64, Option<i64>, i64, String, bool)> = catalog
            .connection
            .prepare(
                "SELECT id,parent_id,sibling_order,name,is_folder FROM library_nodes ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            catalog
                .create_library_playlist(" Mix & <set> \t\n\r Café 音 🎵 ", &[2, 0, 2])
                .unwrap(),
            3
        );
        expected.roots.push(LibraryNode::Playlist {
            name: " Mix & <set> \t\n\r Café 音 🎵 ".into(),
            members: vec![2, 0, 2],
        });
        drop(catalog);
        let reopened = Catalog::open(&path).unwrap();
        assert_eq!(reopened.library_snapshot().unwrap(), expected);
        let nodes_after: Vec<(i64, Option<i64>, i64, String, bool)> = reopened
            .connection
            .prepare(
                "SELECT id,parent_id,sibling_order,name,is_folder FROM library_nodes ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(&nodes_after[..nodes_before.len()], nodes_before);
        let appended = nodes_after.last().unwrap();
        assert_eq!(appended.1, None);
        assert_eq!(appended.2, 2);
        assert!(!appended.4);
    }

    #[test]
    fn create_library_playlist_refusals_leave_snapshot_unchanged() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        assert!(matches!(
            catalog.create_library_playlist("New", &[0]),
            Err(CatalogError::InvalidRecord(
                "catalog holds no imported tracks"
            ))
        ));
        assert_eq!(
            catalog.library_snapshot().unwrap(),
            LibrarySnapshot::default()
        );
        catalog.import_library(&snapshot()).unwrap();
        for (name, members, reason) in [
            ("", vec![0], "playlist name must not be blank"),
            (" \n\t", vec![0], "playlist name must not be blank"),
            (
                "bad\u{1}",
                vec![0],
                "playlist name contains invalid XML 1.0 text",
            ),
            (
                "bad\u{7f}",
                vec![0],
                "playlist name contains invalid XML 1.0 text",
            ),
            (
                "bad\u{85}",
                vec![0],
                "playlist name contains invalid XML 1.0 text",
            ),
            (
                "bad\u{fffe}",
                vec![0],
                "playlist name contains invalid XML 1.0 text",
            ),
            ("New", vec![], "playlist membership must not be empty"),
            ("Zeta", vec![0], "a root node with this name already exists"),
            (
                "Folder",
                vec![0],
                "a root node with this name already exists",
            ),
            (
                "New",
                vec![2, 0, 3],
                "playlist member references a missing track",
            ),
        ] {
            let error = catalog.create_library_playlist(name, &members).unwrap_err();
            assert!(matches!(error, CatalogError::InvalidRecord(actual) if actual == reason));
            assert_eq!(catalog.library_snapshot().unwrap(), snapshot());
        }
        // Failure after inserting the node and two valid members must roll back both.
        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER fail_playlist_member BEFORE INSERT ON library_members
             WHEN NEW.position = 2 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            catalog.create_library_playlist("New", &[2, 0, 2]),
            Err(CatalogError::Sqlite(_))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), snapshot());
        catalog
            .connection
            .execute_batch("DROP TRIGGER fail_playlist_member")
            .unwrap();
        assert_eq!(
            catalog.create_library_playlist("New", &[2, 0, 2]).unwrap(),
            3
        );
        let committed = catalog.library_snapshot().unwrap();
        assert!(matches!(
            catalog.create_library_playlist("New", &[2, 0, 2]),
            Err(CatalogError::InvalidRecord(
                "a root node with this name already exists"
            ))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), committed);
        drop(catalog);
        assert_eq!(
            Catalog::open(&path).unwrap().library_snapshot().unwrap(),
            committed
        );
    }

    #[test]
    fn append_library_playlist_members_preserves_order_and_rolls_back_failures() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        catalog.import_library(&snapshot()).unwrap();
        let before = catalog.library_snapshot().unwrap();
        assert_eq!(
            catalog
                .append_library_playlist_members("Zeta", &[2, 0, 2])
                .unwrap(),
            3
        );
        let mut expected = before.clone();
        let Some(LibraryNode::Playlist { members, .. }) = expected.roots.first_mut() else {
            panic!("fixture root")
        };
        members.extend([2, 0, 2]);
        assert_eq!(catalog.library_snapshot().unwrap(), expected);
        for bad in ["missing", "Folder"] {
            assert!(catalog.append_library_playlist_members(bad, &[0]).is_err());
            assert_eq!(catalog.library_snapshot().unwrap(), expected);
        }
        assert!(
            catalog
                .append_library_playlist_members("Zeta", &[0, 99])
                .is_err()
        );
        assert_eq!(catalog.library_snapshot().unwrap(), expected);
        catalog.connection.execute_batch("CREATE TEMP TRIGGER fail_append BEFORE INSERT ON library_members WHEN NEW.position = 7 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        assert!(matches!(
            catalog.append_library_playlist_members("Zeta", &[1, 2]),
            Err(CatalogError::Sqlite(_))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), expected);
    }

    #[test]
    fn reorder_members_preserves_multiset_and_rolls_back() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        catalog.import_library(&snapshot()).unwrap();
        catalog
            .create_library_playlist("Reorder", &[2, 0, 2])
            .unwrap();
        let before = catalog.library_snapshot().unwrap();
        for (name, values, reason) in [
            (
                "Reorder",
                vec![2, 0, 0],
                "playlist membership counts differ",
            ),
            (
                "Reorder",
                vec![2, 0, 99],
                "playlist membership counts differ",
            ),
            ("Reorder", vec![], "playlist membership must not be empty"),
            ("missing", vec![0], "root playlist does not exist"),
            (
                "Folder",
                vec![0],
                "root name refers to a folder, not a playlist",
            ),
        ] {
            assert!(
                matches!(catalog.reorder_library_playlist_members(name, &values), Err(CatalogError::InvalidRecord(actual)) if actual == reason)
            );
            assert_eq!(catalog.library_snapshot().unwrap(), before);
        }
        catalog.connection.execute("INSERT INTO library_nodes(id,parent_id,sibling_order,name,is_folder) VALUES(999,NULL,999,'Reorder',0)", []).unwrap();
        let ambiguous = catalog.library_snapshot().unwrap();
        assert!(matches!(
            catalog.reorder_library_playlist_members("Reorder", &[2, 2, 0]),
            Err(CatalogError::InvalidRecord(
                "root playlist name is ambiguous"
            ))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), ambiguous);
        catalog
            .connection
            .execute("DELETE FROM library_nodes WHERE id = 999", [])
            .unwrap();
        catalog.connection.execute_batch("CREATE TEMP TRIGGER fail_reorder BEFORE INSERT ON library_members WHEN NEW.position = 1 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        assert!(matches!(
            catalog.reorder_library_playlist_members("Reorder", &[2, 2, 0]),
            Err(CatalogError::Sqlite(_))
        ));
        assert_eq!(catalog.library_snapshot().unwrap(), before);
        catalog
            .connection
            .execute_batch("DROP TRIGGER fail_reorder")
            .unwrap();
        assert_eq!(
            catalog
                .reorder_library_playlist_members("Reorder", &[2, 2, 0])
                .unwrap(),
            3
        );
        let mut expected = before;
        let Some(LibraryNode::Playlist { members, .. }) = expected.roots.last_mut() else {
            panic!("playlist")
        };
        *members = vec![2, 2, 0];
        assert_eq!(catalog.library_snapshot().unwrap(), expected);
    }

    #[test]
    fn beat_grid_is_read_by_location_and_empty_without_one() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        assert!(
            catalog
                .beat_grid_for_locations(&["file://localhost/A.wav"])
                .unwrap()
                .is_empty(),
            "no imported library"
        );
        catalog.import_library(&snapshot()).unwrap();
        assert_eq!(
            catalog
                .beat_grid_for_locations(&["nope", "file://localhost/A.wav"])
                .unwrap(),
            snapshot().tracks[0].beat_grid
        );
        for location in ["file://localhost/B.wav", "file://localhost/missing.wav"] {
            assert!(
                catalog
                    .beat_grid_for_locations(&[location])
                    .unwrap()
                    .is_empty(),
                "{location}"
            );
        }
        let computed: i64 = catalog
            .connection
            .query_row("SELECT COUNT(*) FROM track_waveforms", [], |row| row.get(0))
            .unwrap();
        assert_eq!(computed, 0, "never touches computed peaks");
    }

    #[test]
    fn cues_are_read_by_location_in_order_and_empty_without_any() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        assert!(
            catalog
                .cues_for_locations(&["file://localhost/A.wav"])
                .unwrap()
                .is_empty()
        );
        catalog.import_library(&snapshot()).unwrap();
        assert_eq!(
            catalog
                .cues_for_locations(&["nope", "file://localhost/A.wav"])
                .unwrap(),
            snapshot().tracks[0].cues
        );
        assert_eq!(
            catalog
                .cues_for_locations(&["file://localhost/C.wav"])
                .unwrap(),
            snapshot().tracks[2].cues
        );
        assert!(
            catalog
                .cues_for_locations(&["file://localhost/B.wav"])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn vendor_waveforms_are_stored_apart_from_computed_peaks_and_fail_closed() {
        let path = path();
        let mut catalog = Catalog::open(&path).unwrap();
        let mut bad = snapshot();
        bad.tracks[1].vendor_waveforms = vec![VendorWaveform::RekordboxDetail(vec![[40, 0]])];
        assert!(catalog.import_library(&bad).is_err());
        bad.tracks[1].vendor_waveforms = vec![
            VendorWaveform::RekordboxColourDetail(vec![]),
            VendorWaveform::RekordboxDetail(vec![]),
        ];
        assert!(catalog.import_library(&bad).is_err());
        assert_eq!(
            catalog.library_snapshot().unwrap(),
            LibrarySnapshot::default()
        );
        catalog.import_library(&snapshot()).unwrap();
        let (vendor, computed): (i64, i64) = catalog
            .connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM library_vendor_waveforms),\
                 (SELECT COUNT(*) FROM track_waveforms)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((vendor, computed), (2, 0));
        let vendor_a = catalog
            .vendor_waveforms_for_locations(&["nope", "file://localhost/A.wav"])
            .unwrap();
        assert_eq!(vendor_a, snapshot().tracks[0].vendor_waveforms);
        assert!(
            catalog
                .vendor_waveforms_for_locations(&["file://localhost/B.wav"])
                .unwrap()
                .is_empty(),
            "track without vendor data"
        );
        assert!(
            catalog
                .vendor_waveforms_for_locations(&["file://localhost/missing.wav"])
                .unwrap()
                .is_empty()
        );
        catalog
            .connection
            .execute(
                "UPDATE library_vendor_waveforms SET data=x'2000' WHERE kind='rekordbox-anlz-pwv3'",
                [],
            )
            .unwrap();
        assert!(catalog.library_snapshot().is_err());
        assert!(
            catalog
                .vendor_waveforms_for_locations(&["file://localhost/A.wav"])
                .is_err(),
            "corrupt vendor rows fail closed"
        );
    }

    #[test]
    fn device_relative_locations_match_local_files_under_a_mount_and_fail_closed() {
        // Each library track carries a one-beat grid whose tempo is its index,
        // so the lookup result identifies exactly which track matched.
        let locations = [
            "file://localhost/Contents/Art/a%20b.mp3", // 0 device-relative
            "file://localhost/Contents/Dup/x.mp3",     // 1 } nested suffix:
            "file://localhost/Contents/Z/Contents/Dup/x.mp3", // 2 } ambiguous
            "file://localhost/Contents/Twin/t.mp3",    // 3 } identical
            "file://localhost/Contents/Twin/t.mp3",    // 4 } duplicate
            "file://localhost/E:/Contents/Drive/d.mp3", // 5 drive-qualified
            "file://localhost/home/u/Music/abs.mp3",   // 6 absolute
            "file://localhost/fake/mnt/Contents/Art/a%20b.mp3", // 7 exact for a mount
            "file://localhost/Contents2/q.mp3",        // 8 not device-relative
        ];
        let snapshot = LibrarySnapshot {
            tracks: locations
                .iter()
                .enumerate()
                .map(|(i, location)| LibraryTrack {
                    location: (*location).into(),
                    beat_grid: vec![LibraryBeat {
                        time_ms: 0,
                        tempo_hundredths_bpm: 10_000 + i as u32,
                        beat_in_bar: 1,
                    }],
                    ..track("T")
                })
                .collect(),
            roots: vec![],
        };
        let mut catalog = Catalog::open(path()).unwrap();
        catalog.import_library(&snapshot).unwrap();
        let hit = |candidates: &[&str]| {
            let grid = catalog.beat_grid_for_locations(candidates).unwrap();
            let vendor = catalog.vendor_waveforms_for_locations(candidates).unwrap();
            assert!(vendor.is_empty());
            grid.first().map(|b| b.tempo_hundredths_bpm - 10_000)
        };
        let uri = |p: &str| {
            format!(
                "file://localhost{}",
                p.replace('%', "%25").replace(' ', "%20")
            )
        };
        // Exact match still wins over a suffix match (track 0 would also fit).
        let mounted = "/fake/mnt/Contents/Art/a b.mp3";
        assert_eq!(hit(&[mounted, &uri(mounted)]), Some(7));
        // Suffix match under another fake mount, by local path or its URI.
        let stick = "/run/media/user/STICK/Contents/Art/a b.mp3";
        assert_eq!(hit(&[stick, &uri(stick)]), Some(0));
        assert_eq!(hit(&[stick]), Some(0));
        assert_eq!(hit(&[&uri(stick)]), Some(0));
        // Exact absolute locations still match; nothing else does.
        assert_eq!(hit(&["file://localhost/home/u/Music/abs.mp3"]), Some(6));
        for local in [
            "/m/xContents/Art/a b.mp3",         // partial component name
            "/m/Contents2/Art/a b.mp3",         // partial component name
            "/m/Contents2/q.mp3",               // stored location not /Contents/
            "/m/contents/Art/a b.mp3",          // case-sensitive
            "/m/Contents/Art/xa b.mp3",         // partial file name
            "/m/Contents/Art/a%20b.mp3",        // local name is not decoded
            "/m/Contents/Art",                  // directory, not the file
            "/m/Contents/Z/Contents/Dup/x.mp3", // matches tracks 1 and 2
            "/m/Contents/Twin/t.mp3",           // matches tracks 3 and 4
            "/m/E:/Contents/Drive/d.mp3",       // drive-qualified untouched
            "/m/Contents/Drive/d.mp3",          // drive-qualified untouched
            "/mnt/home/u/Music/abs.mp3",        // absolute location untouched
            "Contents/Art/a b.mp3",             // relative local path
        ] {
            assert_eq!(hit(&[local, &uri(local)]), None, "{local}");
        }
        // A lone match of the shorter nested suffix is not ambiguous.
        assert_eq!(hit(&["/m/Contents/Dup/x.mp3"]), Some(1));
        // Malformed escapes never decode into a match.
        assert_eq!(hit(&["file://localhost/m/Contents/Art/a%2Gb.mp3"]), None);
    }
}
