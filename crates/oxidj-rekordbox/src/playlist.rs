//! Guarded playlist, folder, and membership operations.
use super::*;

/// One optimistic, ordinal-scoped playlist rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistRenameRequest {
    pub playlist_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistRenameChange {
    pub playlist_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistRename {
    pub bytes: Vec<u8>,
    pub change: PlaylistRenameChange,
}
/// Delete one collection record and all playlist memberships under an exact guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackDeleteRequest {
    pub track_id: String,
    pub expected_memberships: Vec<(usize, Vec<String>)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackDelete {
    pub bytes: Vec<u8>,
}
/// Delete one playlist node after guarding its ordinal, name and ordered members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistDeleteRequest {
    pub playlist_index: usize,
    pub expected_name: String,
    pub expected_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistDelete {
    pub bytes: Vec<u8>,
}
/// Remove one playlist reference while retaining its collection track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberDeleteRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberDelete {
    pub bytes: Vec<u8>,
}
/// Clear playlist references without deleting collection tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistClearRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistClear {
    pub bytes: Vec<u8>,
}
/// Append one existing track to a playlist after an exact ordinal/order guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberAppendRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberAppend {
    pub bytes: Vec<u8>,
}
/// Insert one absent collection track at a zero-based playlist position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberInsertRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
    pub position: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberInsert {
    pub bytes: Vec<u8>,
}
/// Reorder an existing playlist after an exact ordinal/order optimistic guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistReorderRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub new_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistReorder {
    pub bytes: Vec<u8>,
}
/// Reorder direct Type=1 children of a guarded non-root Type=0 folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiblingPlaylistReorderRequest {
    pub parent_index: usize,
    pub expected_parent_name: String,
    pub expected_order: Vec<String>,
    pub new_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Prepared XML bytes for a guarded sibling-playlist reorder.
pub struct PreparedSiblingPlaylistReorder {
    pub bytes: Vec<u8>,
}
/// Reorder direct Type=0 children of a guarded non-root Type=0 folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiblingFolderReorderRequest {
    pub parent_index: usize,
    pub expected_parent_name: String,
    pub expected_order: Vec<String>,
    pub new_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Prepared XML bytes for a guarded sibling-folder reorder.
pub struct PreparedSiblingFolderReorder {
    pub bytes: Vec<u8>,
}
/// Move one guarded playlist to the end of another non-root folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMoveRequest {
    pub playlist_index: usize,
    pub expected_playlist_name: String,
    pub expected_members: Vec<String>,
    pub source_parent_index: usize,
    pub expected_source_parent_name: String,
    pub target_parent_index: usize,
    pub expected_target_parent_name: String,
    pub expected_target_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMove {
    pub bytes: Vec<u8>,
}
/// Move one guarded folder subtree to another non-root folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderMoveRequest {
    pub folder_index: usize,
    pub expected_folder_name: String,
    pub source_parent_index: usize,
    pub expected_source_parent_name: String,
    pub expected_source_order: Vec<String>,
    pub target_parent_index: usize,
    pub expected_target_parent_name: String,
    pub expected_target_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderMove {
    pub bytes: Vec<u8>,
}
/// One optimistic, ordinal-scoped rename of a non-root Type=0 folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRenameRequest {
    pub folder_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRenameChange {
    pub folder_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderRename {
    pub bytes: Vec<u8>,
    pub change: FolderRenameChange,
}
/// Delete one guarded empty non-root Type=0 folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDeleteRequest {
    pub folder_index: usize,
    pub expected_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderDelete {
    pub bytes: Vec<u8>,
}
/// Add one empty child folder under an exact non-root folder guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderCreateRequest {
    pub parent_index: usize,
    pub expected_parent_name: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderCreate {
    pub bytes: Vec<u8>,
}
/// Add one empty child playlist (Type=1, KeyType=0 means TrackID).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistCreateRequest {
    pub parent_index: usize,
    pub expected_parent_name: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistCreate {
    pub bytes: Vec<u8>,
}

/// Prepare one guarded playlist rename, preserving all source bytes outside Name.
pub fn prepare_playlist_rename(
    bytes: &[u8],
    request: &PlaylistRenameRequest,
) -> Result<PreparedPlaylistRename, Error> {
    let before = preflight_export(bytes)?;
    let old = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error(format!("no playlist at ordinal {}", request.playlist_index)))?;
    if old.name != request.expected_old_name {
        return Err(Error("playlist Name changed".into()));
    }
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| {
                let n = c as u32;
                n == 9
                    || n == 10
                    || n == 13
                    || (32..=0xD7FF).contains(&n)
                    || (0xE000..=0xFFFD).contains(&n)
                    || (0x10000..=0x10FFFF).contains(&n)
            })
    };
    if !valid(&request.new_name) {
        return Err(Error("invalid playlist Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut siblings: Vec<(Vec<String>, String)> = Vec::new();
    let mut playlist_no = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect();
                siblings.push((parent, name.clone()));
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    playlist_no += 1;
                }
                node_path.push(name);
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect();
                siblings.push((parent, name));
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    playlist_no += 1;
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let range = range.ok_or_else(|| Error("playlist Name location missing".into()))?;
    let parent = old.path[..old.path.len().saturating_sub(1)].to_vec();
    let matching_siblings = siblings
        .iter()
        .filter(|(sibling_parent, name)| sibling_parent == &parent && name == &request.new_name)
        .count();
    if matching_siblings > usize::from(request.expected_old_name == request.new_name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let escaped = escape_attribute(&request.new_name, false);
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index].name = request.new_name.clone();
    expected[request.playlist_index].path.pop();
    expected[request.playlist_index]
        .path
        .push(request.new_name.clone());
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("playlist rename verification failed".into()));
    }
    Ok(PreparedPlaylistRename {
        bytes: output,
        change: PlaylistRenameChange {
            playlist_index: request.playlist_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

/// Prepare deletion of exactly one guarded Type=1 playlist NODE.
pub fn prepare_playlist_delete(
    bytes: &[u8],
    request: &PlaylistDeleteRequest,
) -> Result<PreparedPlaylistDelete, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.name != request.expected_name {
        return Err(Error("playlist Name changed".into()));
    }
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist ordered members changed".into()));
    }
    if before
        .selected_playlists
        .iter()
        .filter(|candidate| {
            candidate.name == request.expected_name && candidate.tracks == request.expected_order
        })
        .count()
        != 1
    {
        return Err(Error("playlist deletion guard is ambiguous".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut ordinal = 0usize;
    let mut nodes: Vec<(String, Option<std::ops::Range<usize>>, usize)> = Vec::new();
    let mut target_depth = None;
    let mut target_range = None;
    let mut parent_count_range = None;
    let mut parent_count = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let count_field = if ty == "0" { "Count" } else { "Entries" };
                let raw_count = attr(&e, count_field)?;
                let declared_count = raw_count
                    .parse::<usize>()
                    .map_err(|_| Error(format!("invalid NODE {count_field}")))?;
                if ty == "1" {
                    if ordinal == request.playlist_index {
                        if attr(&e, "Name")? != request.expected_name {
                            return Err(Error("playlist Name changed".into()));
                        }
                        let Some((_, count_range, folder_count)) =
                            nodes.last().filter(|(parent_ty, _, _)| parent_ty == "0")
                        else {
                            return Err(Error("playlist has no parent folder NODE".into()));
                        };
                        if *folder_count == 0 {
                            return Err(Error("parent folder NODE Count is zero".into()));
                        }
                        parent_count_range = count_range.clone();
                        parent_count = Some(folder_count - 1);
                        target_depth = Some(nodes.len() + 1);
                        target_range = Some(start..end);
                    }
                    ordinal += 1;
                }
                let count_range = if ty == "0" {
                    Some(attribute_value_range(&e, start, count_field, &raw_count)?)
                } else {
                    None
                };
                nodes.push((ty, count_range, declared_count));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "1" {
                    if ordinal == request.playlist_index {
                        if attr(&e, "Name")? != request.expected_name {
                            return Err(Error("playlist Name changed".into()));
                        }
                        let Some((_, count_range, folder_count)) =
                            nodes.last().filter(|(parent_ty, _, _)| parent_ty == "0")
                        else {
                            return Err(Error("playlist has no parent folder NODE".into()));
                        };
                        if *folder_count == 0 {
                            return Err(Error("parent folder NODE Count is zero".into()));
                        }
                        parent_count_range = count_range.clone();
                        parent_count = Some(folder_count - 1);
                        target_range = Some(start..end);
                    }
                    ordinal += 1;
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if target_depth == Some(nodes.len()) {
                    let r = target_range
                        .as_mut()
                        .ok_or_else(|| Error("playlist NODE range missing".into()))?;
                    r.end = end;
                    target_depth = None;
                }
                nodes
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let target_range =
        target_range.ok_or_else(|| Error("playlist NODE location missing".into()))?;
    if target_depth.is_some() {
        return Err(Error("malformed playlist NODE".into()));
    }
    let mut output = bytes.to_vec();
    let parent_count_range =
        parent_count_range.ok_or_else(|| Error("playlist parent Count location missing".into()))?;
    let parent_count = parent_count.ok_or_else(|| Error("playlist parent Count missing".into()))?;
    let mut replacements = vec![
        (target_range, Vec::new()),
        (parent_count_range, parent_count.to_string().into_bytes()),
    ];
    replacements.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, replacement) in replacements {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected.remove(request.playlist_index);
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error(
            "playlist deletion semantic verification failed".into(),
        ));
    }
    Ok(PreparedPlaylistDelete { bytes: output })
}

/// Prepare deletion of one collection track and all of its guarded playlist references.
pub fn prepare_track_delete(
    bytes: &[u8],
    request: &TrackDeleteRequest,
) -> Result<PreparedTrackDelete, Error> {
    let before = preflight_export(bytes)?;
    let track = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .count();
    if track != 1 {
        return Err(Error(
            "target collection track missing or duplicated".into(),
        ));
    }
    let actual_memberships: Vec<_> = before
        .selected_playlists
        .iter()
        .enumerate()
        .filter(|(_, p)| p.tracks.iter().any(|id| id == &request.track_id))
        .map(|(i, p)| (i, p.tracks.clone()))
        .collect();
    if request.expected_memberships != actual_memberships
        || actual_memberships.is_empty()
        || actual_memberships
            .iter()
            .any(|(_, ids)| ids.iter().filter(|id| *id == &request.track_id).count() != 1)
    {
        return Err(Error(
            "playlist membership guard is stale, incomplete, or duplicated".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut playlist_index = None;
    let mut next_playlist = 0usize;
    let mut collection_spans = Vec::new();
    let mut membership_spans: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
    let mut playlist_tag_spans = Vec::new();
    let mut collection_tag_span = None;
    let mut target_open: Option<usize> = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let eof = matches!(&event, Event::Eof);
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"COLLECTION" => {
                collection_tag_span = Some(start..end);
                stack.push("COLLECTION".into());
            }
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "1" {
                    playlist_index = Some(next_playlist);
                    playlist_tag_spans.push((next_playlist, start..end));
                    next_playlist += 1;
                }
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    playlist_tag_spans.push((next_playlist, start..end));
                    next_playlist += 1;
                }
            }
            Event::Start(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.iter().any(|p| p == "COLLECTION")
                    && attr(&e, "TrackID")? == request.track_id =>
            {
                target_open = Some(start);
                stack.push("TRACK".into());
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.iter().any(|p| p == "COLLECTION")
                    && attr(&e, "TrackID")? == request.track_id =>
            {
                collection_spans.push(start..end)
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && playlist_index.is_some()
                    && attr(&e, "Key")? == request.track_id =>
            {
                membership_spans.push((playlist_index.unwrap(), start..end))
            }
            Event::End(e) => {
                if e.name().as_ref() == b"TRACK"
                    && let Some(open) = target_open.take()
                {
                    collection_spans.push(open..end);
                }
                if e.name().as_ref() == b"NODE" {
                    playlist_index = None;
                }
                stack.pop();
            }
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|_| Error("invalid element name".into()))?
                    .to_owned();
                stack.push(name);
            }
            Event::Empty(_) | Event::Eof => {}
            _ => {}
        }
        buf.clear();
        if eof {
            break;
        }
    }
    if collection_spans.len() != 1
        || membership_spans.len()
            != actual_memberships
                .iter()
                .map(|(_, ids)| ids.iter().filter(|id| *id == &request.track_id).count())
                .sum::<usize>()
    {
        return Err(Error(
            "target references are missing, duplicated, or structurally ambiguous".into(),
        ));
    }
    let collection_tag = collection_tag_span.ok_or_else(|| Error("COLLECTION missing".into()))?;
    let mut edits: Vec<(std::ops::Range<usize>, Vec<u8>)> = Vec::new();
    edits.extend(collection_spans.into_iter().map(|r| (r, Vec::new())));
    edits.extend(membership_spans.into_iter().map(|(_, r)| (r, Vec::new())));
    for (span, key) in std::iter::once((collection_tag, "Entries".to_owned())).chain(
        playlist_tag_spans
            .into_iter()
            .filter(|(i, _)| actual_memberships.iter().any(|(j, _)| i == j))
            .map(|(_, span)| (span, "Entries".to_owned())),
    ) {
        let current = bytes[span.clone()].to_vec();
        let count = count_attribute_value_range(&current, &key)?;
        let old = std::str::from_utf8(&current[count.clone()])
            .map_err(|_| Error("invalid count".into()))?
            .parse::<usize>()
            .map_err(|_| Error("malformed entry count".into()))?;
        let replacement = old
            .checked_sub(1)
            .ok_or_else(|| Error("entry count underflow".into()))?
            .to_string()
            .into_bytes();
        edits.push((
            span.start + count.start..span.start + count.end,
            replacement,
        ));
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_tracks = before.selected_tracks.clone();
    expected_tracks.retain(|t| t.id != request.track_id);
    let mut expected_playlists = before.selected_playlists.clone();
    for (i, playlist) in expected_playlists.iter_mut().enumerate() {
        if actual_memberships.iter().any(|(j, _)| *j == i) {
            playlist.tracks.retain(|id| id != &request.track_id);
        }
    }
    if after.selected_tracks != expected_tracks || after.selected_playlists != expected_playlists {
        return Err(Error("track deletion semantic verification failed".into()));
    }
    Ok(PreparedTrackDelete { bytes: output })
}

pub(crate) fn count_attribute_value_range(
    tag: &[u8],
    key: &str,
) -> Result<std::ops::Range<usize>, Error> {
    let needle = key.as_bytes();
    let mut found = tag
        .windows(needle.len())
        .enumerate()
        .filter(|(_, w)| *w == needle);
    let (at, _) = found
        .next()
        .ok_or_else(|| Error(format!("{key} attribute missing")))?;
    if found.next().is_some() {
        return Err(Error(format!("duplicate {key} attribute")));
    }
    let mut i = at + needle.len();
    while tag.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    if tag.get(i) != Some(&b'=') {
        return Err(Error(format!("malformed {key} attribute")));
    }
    i += 1;
    while tag.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let quote = *tag
        .get(i)
        .ok_or_else(|| Error(format!("malformed {key} attribute")))?;
    if quote != b'\'' && quote != b'"' {
        return Err(Error(format!("malformed {key} attribute")));
    }
    let begin = i + 1;
    let end = tag[begin..]
        .iter()
        .position(|b| *b == quote)
        .map(|n| begin + n)
        .ok_or_else(|| Error(format!("malformed {key} attribute")))?;
    Ok(begin..end)
}

/// Prepare removal of one guarded playlist reference without deleting its collection track.
pub fn prepare_playlist_member_delete(
    bytes: &[u8],
    request: &PlaylistMemberDeleteRequest,
) -> Result<PreparedPlaylistMemberDelete, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if playlist
        .tracks
        .iter()
        .filter(|id| *id == &request.track_id)
        .count()
        != 1
    {
        return Err(Error(
            "selected playlist membership missing or ambiguous".into(),
        ));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut node_stack = Vec::<Option<usize>>::new();
    let mut playlist_no = 0usize;
    let mut target = false;
    let mut member = None;
    let mut node_tag = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "1" {
                    target = playlist_no == request.playlist_index;
                    playlist_no += 1;
                }
                node_stack.push(target.then_some(start));
                if target && ty == "1" {
                    node_tag = Some(start..end);
                }
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    if playlist_no == request.playlist_index {
                        target = true;
                        node_tag = Some(start..end);
                    }
                    playlist_no += 1;
                }
                if target && attr(&e, "Type")? == "1" {
                    node_stack.push(None);
                }
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && target
                    && attr(&e, "Key")? == request.track_id =>
            {
                if member.replace(start..end).is_some() {
                    return Err(Error("membership is ambiguous".into()));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                node_stack.pop();
                if node_stack.is_empty() {
                    target = false;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let member = member.ok_or_else(|| Error("membership location missing".into()))?;
    let tag = node_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    let new = old
        .checked_sub(1)
        .ok_or_else(|| Error("Entries underflow".into()))?
        .to_string();
    let mut edits = vec![
        (
            tag.start + count.start..tag.start + count.end,
            new.into_bytes(),
        ),
        (member, Vec::new()),
    ];
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index]
        .tracks
        .retain(|id| id != &request.track_id);
    if after.selected_tracks != before.selected_tracks || after.selected_playlists != expected {
        return Err(Error("playlist member removal verification failed".into()));
    }
    Ok(PreparedPlaylistMemberDelete { bytes: output })
}

/// Prepare clearing every member from one guarded playlist.
pub fn prepare_playlist_clear(
    bytes: &[u8],
    request: &PlaylistClearRequest,
) -> Result<PreparedPlaylistClear, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut next_playlist = 0usize;
    let mut selected_depth = None;
    let mut selected_tag = None;
    let mut member_spans = Vec::new();
    let mut member_start = None;
    let mut member_ids = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(element) => {
                if element.name().as_ref() == b"NODE" && attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        selected_tag = Some(start..end);
                        selected_depth = Some(stack.len() + 1);
                    }
                    next_playlist += 1;
                }
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|tag| tag == "NODE")
                    && selected_depth == Some(stack.len())
                {
                    member_ids.push(attr(&element, "Key")?);
                    member_start = Some(start);
                }
                stack.push(String::from_utf8_lossy(element.name().as_ref()).into_owned());
            }
            Event::Empty(element) => {
                if element.name().as_ref() == b"NODE" && attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        selected_tag = Some(start..end);
                    }
                    next_playlist += 1;
                } else if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|tag| tag == "NODE")
                    && selected_depth == Some(stack.len())
                {
                    member_ids.push(attr(&element, "Key")?);
                    member_spans.push(start..end);
                }
            }
            Event::End(element) => {
                if element.name().as_ref() == b"TRACK"
                    && let Some(member_start) = member_start.take()
                {
                    member_spans.push(member_start..end);
                }
                if element.name().as_ref() == b"NODE" && selected_depth == Some(stack.len()) {
                    selected_depth = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    let tag = selected_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    if member_ids != request.expected_order || member_spans.len() != request.expected_order.len() {
        return Err(Error("playlist membership structure changed".into()));
    }
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old_count = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    if old_count != request.expected_order.len() {
        return Err(Error("playlist Entries count mismatch".into()));
    }
    let mut edits = vec![(
        tag.start + count.start..tag.start + count.end,
        b"0".to_vec(),
    )];
    edits.extend(member_spans.into_iter().map(|span| (span, Vec::new())));
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index].tracks.clear();
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error("playlist clear verification failed".into()));
    }
    Ok(PreparedPlaylistClear { bytes: output })
}

/// Prepare append of one existing collection track to a guarded playlist.
pub fn prepare_playlist_member_append(
    bytes: &[u8],
    request: &PlaylistMemberAppendRequest,
) -> Result<PreparedPlaylistMemberAppend, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if playlist.tracks.iter().any(|id| id == &request.track_id) {
        return Err(Error("playlist already contains TrackID".into()));
    }
    if before
        .selected_tracks
        .iter()
        .filter(|track| track.id == request.track_id)
        .count()
        != 1
    {
        return Err(Error(
            "target collection track missing or duplicated".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut track_key_value = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|parent| parent == "COLLECTION")
                    && attr(&element, "TrackID")? == request.track_id
                {
                    if track_key_value.is_some() {
                        return Err(Error("target collection track is ambiguous".into()));
                    }
                    let range =
                        attribute_value_range(&element, start, "TrackID", &request.track_id)?;
                    track_key_value = Some(bytes[range].to_vec());
                }
                stack.push(String::from_utf8_lossy(element.name().as_ref()).into_owned());
            }
            Event::Empty(element) => {
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|parent| parent == "COLLECTION")
                    && attr(&element, "TrackID")? == request.track_id
                {
                    if track_key_value.is_some() {
                        return Err(Error("target collection track is ambiguous".into()));
                    }
                    let range =
                        attribute_value_range(&element, start, "TrackID", &request.track_id)?;
                    track_key_value = Some(bytes[range].to_vec());
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let track_key_value = track_key_value
        .ok_or_else(|| Error("target collection TrackID location missing".into()))?;

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut node_stack = Vec::<Option<usize>>::new();
    let mut next_playlist = 0usize;
    let mut target_tag = None;
    let mut target_close = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(element) if element.name().as_ref() == b"NODE" => {
                let playlist = if attr(&element, "Type")? == "1" {
                    let current = next_playlist;
                    next_playlist += 1;
                    Some(current)
                } else {
                    None
                };
                if playlist == Some(request.playlist_index) {
                    target_tag = Some((start..end, false));
                }
                node_stack.push(playlist);
            }
            Event::Empty(element) if element.name().as_ref() == b"NODE" => {
                if attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        target_tag = Some((start..end, true));
                    }
                    next_playlist += 1;
                }
            }
            Event::End(element) if element.name().as_ref() == b"NODE" => {
                if node_stack.pop().flatten() == Some(request.playlist_index) {
                    target_close = Some(start);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let (tag, self_closing) = target_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old_count = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    if old_count != request.expected_order.len() {
        return Err(Error("playlist Entries count mismatch".into()));
    }
    let new_count = old_count
        .checked_add(1)
        .ok_or_else(|| Error("Entries overflow".into()))?
        .to_string();
    let mut member = b"<TRACK Key=\"".to_vec();
    member.extend_from_slice(&track_key_value);
    member.extend_from_slice(b"\"/>");

    let mut edits = Vec::new();
    if self_closing {
        let mut replacement = bytes[tag.clone()].to_vec();
        replacement.splice(count, new_count.bytes());
        if !replacement.ends_with(b"/>") {
            return Err(Error("empty playlist NODE is not self-closing".into()));
        }
        replacement.splice(replacement.len() - 2.., b">".iter().copied());
        replacement.extend_from_slice(&member);
        replacement.extend_from_slice(b"</NODE>");
        edits.push((tag, replacement));
    } else {
        let close = target_close.ok_or_else(|| Error("playlist closing NODE missing".into()))?;
        if close < tag.end {
            return Err(Error("playlist NODE span is invalid".into()));
        }
        edits.push((
            tag.start + count.start..tag.start + count.end,
            new_count.into_bytes(),
        ));
        edits.push((close..close, member));
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index]
        .tracks
        .push(request.track_id.clone());
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error("playlist member append verification failed".into()));
    }
    Ok(PreparedPlaylistMemberAppend { bytes: output })
}

/// Prepare insertion at one zero-based position under an exact playlist guard.
pub fn prepare_playlist_member_insert(
    bytes: &[u8],
    request: &PlaylistMemberInsertRequest,
) -> Result<PreparedPlaylistMemberInsert, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if request.position > request.expected_order.len() {
        return Err(Error("playlist insertion position is out of range".into()));
    }
    if request.expected_order.iter().collect::<HashSet<_>>().len() != request.expected_order.len() {
        return Err(Error(
            "playlist insertion does not support duplicate existing memberships".into(),
        ));
    }

    let appended = prepare_playlist_member_append(
        bytes,
        &PlaylistMemberAppendRequest {
            playlist_index: request.playlist_index,
            expected_order: request.expected_order.clone(),
            track_id: request.track_id.clone(),
        },
    )?;
    let mut new_order = request.expected_order.clone();
    new_order.insert(request.position, request.track_id.clone());
    let mut after_append = request.expected_order.clone();
    after_append.push(request.track_id.clone());
    let output = if request.position == request.expected_order.len() {
        appended.bytes
    } else {
        prepare_playlist_reorder(
            &appended.bytes,
            &PlaylistReorderRequest {
                playlist_index: request.playlist_index,
                expected_order: after_append,
                new_order,
            },
        )?
        .bytes
    };
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index].tracks = request.expected_order.clone();
    expected_playlists[request.playlist_index]
        .tracks
        .insert(request.position, request.track_id.clone());
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error(
            "playlist member insertion verification failed".into(),
        ));
    }
    Ok(PreparedPlaylistMemberInsert { bytes: output })
}

/// Prepare a byte-preserving reorder of playlist membership elements.
pub fn prepare_playlist_reorder(
    bytes: &[u8],
    request: &PlaylistReorderRequest,
) -> Result<PreparedPlaylistReorder, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error(format!("no playlist at ordinal {}", request.playlist_index)))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    let existing: HashSet<_> = playlist.tracks.iter().collect();
    let desired: HashSet<_> = request.new_order.iter().collect();
    if request.new_order.len() != playlist.tracks.len()
        || desired.len() != request.new_order.len()
        || desired != existing
    {
        return Err(Error(
            "new order must be a unique permutation of existing TrackIDs".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut playlist_no = 0usize;
    let mut node_ordinal = 0usize;
    let mut node_ids: Vec<usize> = Vec::new();
    let mut target_node: Option<usize> = None;
    let mut spans: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let this_node = node_ordinal;
                node_ordinal += 1;
                node_path.push(name);
                node_ids.push(this_node);
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        target_node = Some(this_node);
                    }
                    playlist_no += 1;
                }
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    let this_node = node_ordinal;
                    node_ordinal += 1;
                    if playlist_no == request.playlist_index {
                        target_node = Some(this_node);
                    }
                    playlist_no += 1;
                } else {
                    node_ordinal += 1;
                }
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && target_node.is_some_and(|target| node_ids.last() == Some(&target)) =>
            {
                spans.push((attr(&e, "Key")?, start..end));
            }
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                    node_ids.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let mut ordered = spans.clone();
    ordered.sort_by_key(|(id, _)| {
        request
            .new_order
            .iter()
            .position(|v| v == id)
            .unwrap_or(usize::MAX)
    });
    if spans.iter().map(|(id, _)| id).collect::<Vec<_>>()
        != playlist.tracks.iter().collect::<Vec<_>>()
    {
        return Err(Error(
            "playlist membership locations could not be verified".into(),
        ));
    }
    let mut output = bytes.to_vec();
    // Replace each membership slot with the requested element while retaining all inter-element bytes.
    for (i, (_, slot)) in spans.iter().enumerate().rev() {
        let element = &bytes[ordered[i].1.clone()];
        output.splice(slot.clone(), element.iter().copied());
    }
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index].tracks = request.new_order.clone();
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("playlist reorder verification failed".into()));
    }
    Ok(PreparedPlaylistReorder { bytes: output })
}

/// Prepare a guarded reorder of direct playlists within one non-root folder.
pub fn prepare_sibling_playlist_reorder(
    bytes: &[u8],
    request: &SiblingPlaylistReorderRequest,
) -> Result<PreparedSiblingPlaylistReorder, Error> {
    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<(String, String, usize)> = Vec::new();
    let mut folder_ordinal = 0usize;
    let mut selected_depth = None;
    let mut selected_parent_closed = false;
    let mut playlists: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !stack.is_empty() {
                    if folder_ordinal == request.parent_index {
                        if name != request.expected_parent_name {
                            return Err(Error("parent folder Name changed".into()));
                        }
                        if stack
                            .last()
                            .is_none_or(|(parent_ty, _, _)| parent_ty != "0")
                        {
                            return Err(Error(
                                "parent folder is root or not nested under folder".into(),
                            ));
                        }
                        selected_depth = Some(stack.len());
                    }
                    folder_ordinal += 1;
                }
                stack.push((ty, name, start));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !stack.is_empty() {
                    if folder_ordinal == request.parent_index {
                        return Err(Error("selected parent folder is empty node".into()));
                    }
                    folder_ordinal += 1;
                }
                if ty == "1" && selected_depth.is_some_and(|depth| stack.len() == depth + 1) {
                    playlists.push((name, start..end));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                let (ty, name, node_start) = stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                if ty == "1" && selected_depth.is_some_and(|depth| stack.len() == depth + 1) {
                    playlists.push((name, node_start..end));
                }
                if ty == "0" && selected_depth == Some(stack.len()) {
                    selected_parent_closed = true;
                    selected_depth = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if !selected_parent_closed {
        return Err(Error(format!(
            "no paired non-root folder at ordinal {}",
            request.parent_index
        )));
    }
    let current: Vec<_> = playlists.iter().map(|(name, _)| name.clone()).collect();
    let unique: HashSet<_> = current.iter().collect();
    if unique.len() != current.len() {
        return Err(Error("ambiguous duplicate direct playlist Names".into()));
    }
    if current != request.expected_order {
        return Err(Error("direct playlist order changed".into()));
    }
    let desired: HashSet<_> = request.new_order.iter().collect();
    if request.new_order.len() != current.len()
        || desired.len() != request.new_order.len()
        || desired != unique
    {
        return Err(Error(
            "new order must be a unique permutation of direct playlist Names".into(),
        ));
    }
    let mut output = bytes.to_vec();
    for (index, (_, slot)) in playlists.iter().enumerate().rev() {
        let name = &request.new_order[index];
        let source = playlists
            .iter()
            .find(|(candidate, _)| candidate == name)
            .unwrap();
        output.splice(slot.clone(), bytes[source.1.clone()].iter().copied());
    }
    let after = preflight_export(&output)?;
    if after.selected_tracks != before.selected_tracks {
        return Err(Error("sibling playlist reorder changed tracks".into()));
    }
    let mut verify = Reader::from_reader(output.as_slice());
    verify.config_mut().check_end_names = true;
    let mut vbuf = Vec::new();
    let mut vstack: Vec<(String, String)> = Vec::new();
    let mut ordinal = 0usize;
    let mut depth = None;
    let mut observed = Vec::new();
    loop {
        match verify
            .read_event_into(&mut vbuf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !vstack.is_empty() {
                    if ordinal == request.parent_index {
                        depth = Some(vstack.len());
                    }
                    ordinal += 1;
                }
                if ty == "1" && depth.is_some_and(|d| vstack.len() == d + 1) {
                    observed.push(name.clone());
                }
                vstack.push((ty, name));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "0" && !vstack.is_empty() {
                    if ordinal == request.parent_index {
                        depth = Some(vstack.len());
                    }
                    ordinal += 1;
                }
                if ty == "1" && depth.is_some_and(|d| vstack.len() == d + 1) {
                    observed.push(attr(&e, "Name")?);
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                let (ty, _) = vstack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                if ty == "0" && depth == Some(vstack.len()) {
                    depth = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        vbuf.clear();
    }
    if observed != request.new_order {
        return Err(Error("sibling playlist reorder verification failed".into()));
    }
    Ok(PreparedSiblingPlaylistReorder { bytes: output })
}

pub fn prepare_sibling_folder_reorder(
    bytes: &[u8],
    request: &SiblingFolderReorderRequest,
) -> Result<PreparedSiblingFolderReorder, Error> {
    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<(String, String, usize)> = Vec::new();
    let mut folder_ordinal = 0usize;
    let mut selected_depth = None;
    let mut selected_parent_closed = false;
    let mut folders: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !stack.is_empty() {
                    if folder_ordinal == request.parent_index {
                        if name != request.expected_parent_name {
                            return Err(Error("parent folder Name changed".into()));
                        }
                        if stack
                            .last()
                            .is_none_or(|(parent_ty, _, _)| parent_ty != "0")
                        {
                            return Err(Error(
                                "parent folder is root or not nested under folder".into(),
                            ));
                        }
                        selected_depth = Some(stack.len());
                    }
                    folder_ordinal += 1;
                }
                stack.push((ty, name, start));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !stack.is_empty() {
                    if folder_ordinal == request.parent_index {
                        return Err(Error("selected parent folder is empty node".into()));
                    }
                    folder_ordinal += 1;
                }
                if ty == "0" && selected_depth.is_some_and(|depth| stack.len() == depth + 1) {
                    folders.push((name, start..end));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                let (ty, name, node_start) = stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                if ty == "0" && selected_depth.is_some_and(|depth| stack.len() == depth + 1) {
                    folders.push((name, node_start..end));
                }
                if ty == "0" && selected_depth == Some(stack.len()) {
                    selected_parent_closed = true;
                    selected_depth = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if !selected_parent_closed {
        return Err(Error(format!(
            "no paired non-root folder at ordinal {}",
            request.parent_index
        )));
    }
    let current: Vec<_> = folders.iter().map(|(name, _)| name.clone()).collect();
    let unique: HashSet<_> = current.iter().collect();
    if unique.len() != current.len() {
        return Err(Error("ambiguous duplicate direct folder Names".into()));
    }
    if current.is_empty() {
        return Err(Error("selected parent has no direct child folders".into()));
    }
    if current != request.expected_order {
        return Err(Error("direct folder order changed".into()));
    }
    let desired: HashSet<_> = request.new_order.iter().collect();
    if request.new_order.len() != current.len()
        || desired.len() != request.new_order.len()
        || desired != unique
    {
        return Err(Error(
            "new order must be a unique permutation of direct folder Names".into(),
        ));
    }
    let mut output = bytes.to_vec();
    for (index, (_, slot)) in folders.iter().enumerate().rev() {
        let name = &request.new_order[index];
        let source = folders
            .iter()
            .find(|(candidate, _)| candidate == name)
            .unwrap();
        output.splice(slot.clone(), bytes[source.1.clone()].iter().copied());
    }
    let after = preflight_export(&output)?;
    let mut before_playlists: Vec<_> = before
        .selected_playlists
        .iter()
        .map(|p| (p.path.clone(), p.name.clone(), p.tracks.clone()))
        .collect();
    let mut after_playlists: Vec<_> = after
        .selected_playlists
        .iter()
        .map(|p| (p.path.clone(), p.name.clone(), p.tracks.clone()))
        .collect();
    before_playlists.sort_unstable();
    after_playlists.sort_unstable();
    if after.selected_tracks != before.selected_tracks || after_playlists != before_playlists {
        return Err(Error(
            "sibling folder reorder changed tracks or playlist semantics".into(),
        ));
    }
    let mut verify = Reader::from_reader(output.as_slice());
    verify.config_mut().check_end_names = true;
    let mut vbuf = Vec::new();
    let mut vstack: Vec<(String, String)> = Vec::new();
    let mut ordinal = 0usize;
    let mut depth = None;
    let mut observed = Vec::new();
    loop {
        match verify
            .read_event_into(&mut vbuf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                if ty == "0" && !vstack.is_empty() {
                    if ordinal == request.parent_index {
                        depth = Some(vstack.len());
                    }
                    ordinal += 1;
                }
                if ty == "0" && depth.is_some_and(|d| vstack.len() == d + 1) {
                    observed.push(name.clone());
                }
                vstack.push((ty, name));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "0" && !vstack.is_empty() {
                    if ordinal == request.parent_index {
                        depth = Some(vstack.len());
                    }
                    ordinal += 1;
                }
                if ty == "0" && depth.is_some_and(|d| vstack.len() == d + 1) {
                    observed.push(attr(&e, "Name")?);
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                let (ty, _) = vstack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                if ty == "0" && depth == Some(vstack.len()) {
                    depth = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        vbuf.clear();
    }
    if observed != request.new_order {
        return Err(Error("sibling folder reorder verification failed".into()));
    }
    Ok(PreparedSiblingFolderReorder { bytes: output })
}

/// Prepare one guarded cross-folder subtree move; only its two parent Counts change.
pub fn prepare_folder_move(
    bytes: &[u8],
    request: &FolderMoveRequest,
) -> Result<PreparedFolderMove, Error> {
    #[derive(Clone)]
    struct Folder {
        name: String,
        parent: Option<usize>,
        range: Option<std::ops::Range<usize>>,
        close_start: Option<usize>,
        empty_range: Option<std::ops::Range<usize>>,
        count: usize,
        count_range: std::ops::Range<usize>,
        children: Vec<String>,
    }
    struct Frame {
        ty: String,
        folder: Option<usize>,
    }
    fn scan(bytes: &[u8]) -> Result<Vec<Folder>, Error> {
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().check_end_names = true;
        let mut buf = Vec::new();
        let mut stack: Vec<Frame> = Vec::new();
        let mut folders: Vec<Folder> = Vec::new();
        loop {
            let start = reader.buffer_position() as usize;
            let event = reader
                .read_event_into(&mut buf)
                .map_err(|error| Error(format!("malformed XML: {error}")))?;
            let end = reader.buffer_position() as usize;
            match event {
                Event::Start(element) if element.name().as_ref() == b"NODE" => {
                    let ty = attr(&element, "Type")?;
                    let name = attr(&element, "Name")?;
                    let parent = stack
                        .last()
                        .filter(|frame| frame.ty == "0")
                        .and_then(|frame| frame.folder);
                    let root = ty == "0" && stack.is_empty();
                    let folder = if ty == "0" && !root {
                        if let Some(parent) = parent {
                            folders[parent].children.push(format!("0:{name}"));
                        }
                        let raw_count = attr(&element, "Count")?;
                        let count = raw_count
                            .parse::<usize>()
                            .map_err(|_| Error("invalid folder Count".into()))?;
                        let count_range =
                            attribute_value_range(&element, start, "Count", &raw_count)?;
                        let index = folders.len();
                        folders.push(Folder {
                            name,
                            parent,
                            range: Some(start..end),
                            close_start: None,
                            empty_range: None,
                            count,
                            count_range,
                            children: Vec::new(),
                        });
                        Some(index)
                    } else {
                        if ty == "1"
                            && let Some(parent) = parent
                        {
                            folders[parent].children.push(format!("1:{name}"));
                        }
                        None
                    };
                    stack.push(Frame { ty, folder });
                }
                Event::Empty(element) if element.name().as_ref() == b"NODE" => {
                    let ty = attr(&element, "Type")?;
                    let name = attr(&element, "Name")?;
                    let parent = stack
                        .last()
                        .filter(|frame| frame.ty == "0")
                        .and_then(|frame| frame.folder);
                    if ty == "1" {
                        if let Some(parent) = parent {
                            folders[parent].children.push(format!("1:{name}"));
                        }
                    } else if ty == "0" && !stack.is_empty() {
                        if let Some(parent) = parent {
                            folders[parent].children.push(format!("0:{name}"));
                        }
                        let raw_count = attr(&element, "Count")?;
                        let count = raw_count
                            .parse::<usize>()
                            .map_err(|_| Error("invalid folder Count".into()))?;
                        let count_range =
                            attribute_value_range(&element, start, "Count", &raw_count)?;
                        folders.push(Folder {
                            name,
                            parent,
                            range: Some(start..end),
                            close_start: None,
                            empty_range: Some(start..end),
                            count,
                            count_range,
                            children: Vec::new(),
                        });
                    }
                }
                Event::End(element) if element.name().as_ref() == b"NODE" => {
                    let frame = stack
                        .pop()
                        .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                    if let Some(index) = frame.folder {
                        folders[index].range.as_mut().unwrap().end = end;
                        folders[index].close_start = Some(start);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        if !stack.is_empty() {
            return Err(Error("malformed folder NODE".into()));
        }
        Ok(folders)
    }

    let before = preflight_export(bytes)?;
    let folders = scan(bytes)?;
    let folder = folders
        .get(request.folder_index)
        .ok_or_else(|| Error("no source folder at requested ordinal".into()))?;
    if folder.name != request.expected_folder_name {
        return Err(Error("source folder Name changed".into()));
    }
    let source_parent_index = folder
        .parent
        .ok_or_else(|| Error("source folder must have a non-root parent".into()))?;
    if source_parent_index != request.source_parent_index {
        return Err(Error("source parent folder changed".into()));
    }
    let source_parent = &folders[source_parent_index];
    let target_parent = folders
        .get(request.target_parent_index)
        .ok_or_else(|| Error("no target parent folder at requested ordinal".into()))?;
    if source_parent.name != request.expected_source_parent_name
        || target_parent.name != request.expected_target_parent_name
    {
        return Err(Error("source or target parent Name changed".into()));
    }
    if source_parent_index == request.target_parent_index {
        return Err(Error("source and target parents are the same".into()));
    }
    let mut ancestor = Some(request.target_parent_index);
    while let Some(index) = ancestor {
        if index == request.folder_index {
            return Err(Error(
                "target parent is inside source folder subtree".into(),
            ));
        }
        ancestor = folders[index].parent;
    }
    let unique = |children: &[String]| {
        let names: HashSet<_> = children.iter().collect();
        names.len() == children.len()
    };
    if !unique(&source_parent.children) || !unique(&target_parent.children) {
        return Err(Error("ambiguous duplicate direct child Names".into()));
    }
    if source_parent.children != request.expected_source_order {
        return Err(Error("source parent child order changed".into()));
    }
    if target_parent.children != request.expected_target_order {
        return Err(Error("target parent child order changed".into()));
    }
    if source_parent.count != source_parent.children.len()
        || target_parent.count != target_parent.children.len()
    {
        return Err(Error(
            "source or target parent Count does not match children".into(),
        ));
    }
    let source_count = source_parent
        .count
        .checked_sub(1)
        .ok_or_else(|| Error("source parent Count is zero".into()))?;
    let target_count = target_parent
        .count
        .checked_add(1)
        .ok_or_else(|| Error("target parent Count overflow".into()))?;
    let source_range = folder
        .range
        .clone()
        .ok_or_else(|| Error("source folder byte range missing".into()))?;
    let moved_folder_count = folders
        .iter()
        .filter(|candidate| {
            candidate.range.as_ref().is_some_and(|range| {
                range.start >= source_range.start && range.end <= source_range.end
            })
        })
        .count();
    let source_child = format!("0:{}", folder.name);
    let moved_bytes = bytes[source_range.clone()].to_vec();

    let mut changes = vec![
        (source_range, Vec::new()),
        (
            source_parent.count_range.clone(),
            source_count.to_string().into_bytes(),
        ),
    ];
    if let Some(target_range) = target_parent.empty_range.as_ref() {
        let mut expanded = bytes[target_range.clone()].to_vec();
        let count_range = target_parent.count_range.start - target_range.start
            ..target_parent.count_range.end - target_range.start;
        expanded.splice(count_range, target_count.to_string().bytes());
        let slash = expanded
            .windows(2)
            .rposition(|pair| pair == b"/>")
            .ok_or_else(|| Error("empty target parent is not self-closing".into()))?;
        expanded.splice(slash..slash + 2, *b">");
        let line_start = bytes[..target_range.start]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let indent = &bytes[line_start..target_range.start];
        let indent = if indent.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
            indent
        } else {
            &[]
        };
        expanded.extend_from_slice(b"\n");
        expanded.extend_from_slice(indent);
        expanded.extend_from_slice(&moved_bytes);
        expanded.extend_from_slice(b"\n");
        expanded.extend_from_slice(indent);
        expanded.extend_from_slice(b"</NODE>");
        changes.push((target_range.clone(), expanded));
    } else {
        let close_start = target_parent
            .close_start
            .ok_or_else(|| Error("target parent is not paired".into()))?;
        let line_start = bytes[..close_start]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let indent = &bytes[line_start..close_start];
        let indent = if indent.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
            indent
        } else {
            &[]
        };
        let mut insertion = Vec::new();
        insertion.extend_from_slice(b"\n");
        insertion.extend_from_slice(indent);
        insertion.extend_from_slice(&moved_bytes);
        insertion.extend_from_slice(b"\n");
        insertion.extend_from_slice(indent);
        changes.push((
            target_parent.count_range.clone(),
            target_count.to_string().into_bytes(),
        ));
        changes.push((close_start..close_start, insertion));
    }
    changes.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in changes {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let after_folders = scan(&output)?;
    let after_target_parent_index = if request.target_parent_index > request.folder_index {
        request
            .target_parent_index
            .checked_sub(moved_folder_count)
            .ok_or_else(|| Error("target parent ordinal underflow".into()))?
    } else {
        request.target_parent_index
    };
    let after_source_parent = after_folders
        .get(request.source_parent_index)
        .ok_or_else(|| Error("source parent missing after move".into()))?;
    let after_target_parent = after_folders
        .get(after_target_parent_index)
        .ok_or_else(|| Error("target parent missing after move".into()))?;
    let expected_source_order: Vec<_> = request
        .expected_source_order
        .iter()
        .filter(|child| **child != source_child)
        .cloned()
        .collect();
    let mut expected_target_order = request.expected_target_order.clone();
    expected_target_order.push(source_child);
    if after_source_parent.children != expected_source_order
        || after_target_parent.children != expected_target_order
        || after_source_parent.count != after_source_parent.children.len()
        || after_target_parent.count != after_target_parent.children.len()
    {
        return Err(Error(
            "folder move child order or Count verification failed".into(),
        ));
    }
    let mut before_playlists: Vec<_> = before
        .selected_playlists
        .iter()
        .map(|playlist| (playlist.name.clone(), playlist.tracks.clone()))
        .collect();
    let mut after_playlists: Vec<_> = after
        .selected_playlists
        .iter()
        .map(|playlist| (playlist.name.clone(), playlist.tracks.clone()))
        .collect();
    before_playlists.sort_unstable();
    after_playlists.sort_unstable();
    if after.selected_tracks != before.selected_tracks || before_playlists != after_playlists {
        return Err(Error(
            "folder move changed track or playlist membership semantics".into(),
        ));
    }
    Ok(PreparedFolderMove { bytes: output })
}

/// Prepare one guarded non-root folder rename, preserving every other source byte.
pub fn prepare_folder_rename(
    bytes: &[u8],
    request: &FolderRenameRequest,
) -> Result<PreparedFolderRename, Error> {
    let before = preflight_export(bytes)?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| {
                let n = c as u32;
                n == 9
                    || n == 10
                    || n == 13
                    || (32..=0xD7FF).contains(&n)
                    || (0xE000..=0xFFFD).contains(&n)
                    || (0x10000..=0x10FFFF).contains(&n)
            })
    };
    if !valid(&request.new_name) {
        return Err(Error("invalid folder Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut folders: Vec<(String, Vec<String>)> = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut siblings: Vec<(Vec<String>, String)> = Vec::new();
    let mut folder_no = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect::<Vec<_>>();
                siblings.push((parent.clone(), name.clone()));
                if ty == "0" && !node_path.is_empty() {
                    if folder_no == request.folder_index {
                        if name != request.expected_old_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    folders.push((name.clone(), node_path.iter().skip(1).cloned().collect()));
                    folder_no += 1;
                }
                node_path.push(name);
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect::<Vec<_>>();
                siblings.push((parent, name.clone()));
                if ty == "0" && !node_path.is_empty() {
                    if folder_no == request.folder_index {
                        if name != request.expected_old_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    folders.push((name, node_path.iter().skip(1).cloned().collect()));
                    folder_no += 1;
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let old = folders.get(request.folder_index).ok_or_else(|| {
        Error(format!(
            "no non-root folder at ordinal {}",
            request.folder_index
        ))
    })?;
    if old.0 != request.expected_old_name {
        return Err(Error("folder Name changed".into()));
    }
    let range = range.ok_or_else(|| Error("folder Name location missing".into()))?;
    let parent = old.1.clone();
    let matches = siblings
        .iter()
        .filter(|(p, n)| p == &parent && n == &request.new_name)
        .count();
    if matches > usize::from(request.expected_old_name == request.new_name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let escaped = escape_attribute(&request.new_name, false);
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    for playlist in &mut expected {
        if playlist.path.len() > parent.len()
            && playlist.path[..parent.len()] == parent[..]
            && playlist.path[parent.len()] == request.expected_old_name
        {
            playlist.path[parent.len()] = request.new_name.clone();
        }
    }
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("folder rename verification failed".into()));
    }
    Ok(PreparedFolderRename {
        bytes: output,
        change: FolderRenameChange {
            folder_index: request.folder_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

/// Prepare deletion of one empty folder; only the immediate parent's Count changes.
pub fn prepare_folder_delete(
    bytes: &[u8],
    request: &FolderDeleteRequest,
) -> Result<PreparedFolderDelete, Error> {
    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<(String, Option<std::ops::Range<usize>>, usize)> = Vec::new();
    let mut ordinal = 0usize;
    let mut target_depth = None;
    let mut target_range = None;
    let mut parent_range = None;
    let mut parent_value = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let raw = attr(&e, if ty == "0" { "Count" } else { "Entries" })?;
                let value = raw
                    .parse::<usize>()
                    .map_err(|_| Error("invalid NODE Count/Entries".into()))?;
                if ty == "0" && !stack.is_empty() {
                    if ordinal == request.folder_index {
                        if attr(&e, "Name")? != request.expected_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        if value != 0 {
                            return Err(Error("folder Count is not zero".into()));
                        }
                        let (pty, range, count) = stack.last().unwrap();
                        if pty != "0" {
                            return Err(Error("folder has no parent folder NODE".into()));
                        }
                        if *count == 0 {
                            return Err(Error("parent folder NODE Count is zero".into()));
                        }
                        parent_range = range.clone();
                        parent_value = Some(count - 1);
                        target_depth = Some(stack.len() + 1);
                        target_range = Some(start..end);
                    }
                    ordinal += 1;
                }
                let count_range = if ty == "0" {
                    Some(attribute_value_range(&e, start, "Count", &raw)?)
                } else {
                    None
                };
                stack.push((ty, count_range, value));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" && attr(&e, "Type")? == "0" => {
                let raw = attr(&e, "Count")?;
                let count = raw
                    .parse::<usize>()
                    .map_err(|_| Error("invalid NODE Count".into()))?;
                if !stack.is_empty() {
                    if ordinal == request.folder_index {
                        if attr(&e, "Name")? != request.expected_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        if count != 0 {
                            return Err(Error("folder Count is not zero".into()));
                        }
                        let (ty, range, pcount) = stack.last().unwrap();
                        if ty != "0" {
                            return Err(Error("folder has no parent folder NODE".into()));
                        }
                        if *pcount == 0 {
                            return Err(Error("parent folder NODE Count is zero".into()));
                        }
                        parent_range = range.clone();
                        parent_value = Some(pcount - 1);
                        target_range = Some(start..end);
                    }
                    ordinal += 1;
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if target_depth == Some(stack.len()) {
                    if target_range.as_ref().is_none_or(|r| r.end != start) {
                        return Err(Error("folder NODE has child elements".into()));
                    }
                    target_range.as_mut().unwrap().end = end;
                    target_depth = None;
                }
                stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let target = target_range.ok_or_else(|| {
        Error(format!(
            "no non-root folder at ordinal {}",
            request.folder_index
        ))
    })?;
    if target_depth.is_some() {
        return Err(Error("malformed folder NODE".into()));
    }
    let mut output = bytes.to_vec();
    let count_range =
        parent_range.ok_or_else(|| Error("parent folder Count location missing".into()))?;
    let count = parent_value.ok_or_else(|| Error("parent folder Count missing".into()))?;
    let mut replacements = vec![
        (target, Vec::new()),
        (count_range, count.to_string().into_bytes()),
    ];
    replacements.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, replacement) in replacements {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != before.selected_playlists
    {
        return Err(Error("folder deletion semantic verification failed".into()));
    }
    Ok(PreparedFolderDelete { bytes: output })
}

/// Prepare creation of one empty child folder; only the selected parent's Count changes.
pub fn prepare_folder_create(
    bytes: &[u8],
    request: &FolderCreateRequest,
) -> Result<PreparedFolderCreate, Error> {
    if request.name.is_empty() || !valid_xml_text(&request.name) {
        return Err(Error("invalid folder Name XML text".into()));
    }
    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<(String, Option<std::ops::Range<usize>>, usize)> = Vec::new();
    let mut ordinal = 0usize;
    let mut selected: Option<(usize, std::ops::Range<usize>, usize, usize, String)> = None;
    let mut closing_pos = None;
    let mut siblings = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let raw = attr(&e, if ty == "0" { "Count" } else { "Entries" })?;
                let count = raw
                    .parse::<usize>()
                    .map_err(|_| Error("invalid NODE Count/Entries".into()))?;
                if selected.as_ref().is_some_and(|s| stack.len() == s.0 + 1) {
                    siblings.push(attr(&e, "Name")?);
                }
                if ty == "0" && !stack.is_empty() {
                    let name = attr(&e, "Name")?;
                    if ordinal == request.parent_index {
                        if name != request.expected_parent_name {
                            return Err(Error("parent folder Name changed".into()));
                        }
                        if stack.last().is_none_or(|(t, _, _)| t != "0") {
                            return Err(Error(
                                "parent folder is root or not nested under folder".into(),
                            ));
                        }
                        if count == usize::MAX {
                            return Err(Error("parent folder Count overflow".into()));
                        }
                        let range = attribute_value_range(&e, start, "Count", &raw)?;
                        selected = Some((stack.len(), range, count + 1, start, name));
                    }
                    ordinal += 1;
                }
                let range = if ty == "0" {
                    Some(attribute_value_range(&e, start, "Count", &raw)?)
                } else {
                    None
                };
                stack.push((ty, range, count));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if selected.as_ref().is_some_and(|s| stack.len() == s.0 + 1) {
                    siblings.push(attr(&e, "Name")?);
                }
                if ty == "0" {
                    let raw = attr(&e, "Count")?;
                    raw.parse::<usize>()
                        .map_err(|_| Error("invalid NODE Count".into()))?;
                    if !stack.is_empty() {
                        if ordinal == request.parent_index {
                            return Err(Error("selected parent folder is empty node".into()));
                        }
                        ordinal += 1;
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if let Some((depth, _, _, _, _)) = selected.as_ref()
                    && stack.len() == *depth + 1
                {
                    closing_pos = Some(start);
                }
                stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let (_, count_range, new_count, _, parent_name) = selected.ok_or_else(|| {
        Error(format!(
            "no non-root folder at ordinal {}",
            request.parent_index
        ))
    })?;
    if siblings.iter().any(|n| n == &request.name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let insert_at = closing_pos.ok_or_else(|| Error("malformed selected parent folder".into()))?;
    let escaped = escape_attribute(&request.name, false);
    let node = format!("<NODE Type=\"0\" Name=\"{escaped}\" Count=\"0\"/>\n");
    let mut output = bytes.to_vec();
    let mut changes = vec![
        (count_range, new_count.to_string().into_bytes()),
        (insert_at..insert_at, node.into_bytes()),
    ];
    changes.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, replacement) in changes {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != before.selected_playlists
    {
        return Err(Error("folder creation changed tracks/playlists".into()));
    }
    let _ = parent_name;
    Ok(PreparedFolderCreate { bytes: output })
}

/// Prepare creation of an empty child playlist; parent Count changes, KeyType=0 denotes TrackID.
pub fn prepare_playlist_create(
    bytes: &[u8],
    request: &PlaylistCreateRequest,
) -> Result<PreparedPlaylistCreate, Error> {
    if request.name.is_empty() || !valid_xml_text(&request.name) {
        return Err(Error("invalid playlist Name XML text".into()));
    }
    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<(String, Option<std::ops::Range<usize>>, usize)> = Vec::new();
    let mut ordinal = 0usize;
    let mut selected: Option<(usize, std::ops::Range<usize>, usize, usize, String)> = None;
    let mut closing_pos = None;
    let mut siblings = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let raw = attr(&e, if ty == "0" { "Count" } else { "Entries" })?;
                let count = raw
                    .parse::<usize>()
                    .map_err(|_| Error("invalid NODE Count/Entries".into()))?;
                if selected.as_ref().is_some_and(|s| stack.len() == s.0 + 1) {
                    siblings.push(attr(&e, "Name")?);
                }
                if ty == "0" && !stack.is_empty() {
                    let name = attr(&e, "Name")?;
                    if ordinal == request.parent_index {
                        if name != request.expected_parent_name {
                            return Err(Error("parent folder Name changed".into()));
                        }
                        if stack.last().is_none_or(|(t, _, _)| t != "0") {
                            return Err(Error(
                                "parent folder is root or not nested under folder".into(),
                            ));
                        }
                        if count == usize::MAX {
                            return Err(Error("parent folder Count overflow".into()));
                        }
                        let range = attribute_value_range(&e, start, "Count", &raw)?;
                        selected = Some((stack.len(), range, count + 1, start, name));
                    }
                    ordinal += 1;
                }
                let range = if ty == "0" {
                    Some(attribute_value_range(&e, start, "Count", &raw)?)
                } else {
                    None
                };
                stack.push((ty, range, count));
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if selected.as_ref().is_some_and(|s| stack.len() == s.0 + 1) {
                    siblings.push(attr(&e, "Name")?);
                }
                if ty == "0" {
                    let raw = attr(&e, "Count")?;
                    raw.parse::<usize>()
                        .map_err(|_| Error("invalid NODE Count".into()))?;
                    if !stack.is_empty() {
                        if ordinal == request.parent_index {
                            return Err(Error("selected parent folder is empty node".into()));
                        }
                        ordinal += 1;
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if let Some((depth, _, _, _, _)) = selected.as_ref()
                    && stack.len() == *depth + 1
                {
                    closing_pos = Some(start);
                }
                stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let (_, count_range, new_count, _, parent_name) = selected.ok_or_else(|| {
        Error(format!(
            "no non-root folder at ordinal {}",
            request.parent_index
        ))
    })?;
    if siblings.iter().any(|n| n == &request.name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let insert_at = closing_pos.ok_or_else(|| Error("malformed selected parent folder".into()))?;
    let escaped = escape_attribute(&request.name, false);
    let node = format!("<NODE Type=\"1\" Name=\"{escaped}\" KeyType=\"0\" Entries=\"0\"/>\n");
    let mut output = bytes.to_vec();
    let mut changes = vec![
        (count_range, new_count.to_string().into_bytes()),
        (insert_at..insert_at, node.into_bytes()),
    ];
    changes.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, replacement) in changes {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    if after.selected_tracks != before.selected_tracks
        || after
            .selected_playlists
            .get(..before.selected_playlists.len())
            != Some(before.selected_playlists.as_slice())
    {
        return Err(Error(
            "playlist creation changed tracks/existing playlists".into(),
        ));
    }
    let _ = parent_name;
    Ok(PreparedPlaylistCreate { bytes: output })
}

/// Move one guarded playlist to the end of a different non-root folder.
pub fn prepare_playlist_move(
    bytes: &[u8],
    request: &PlaylistMoveRequest,
) -> Result<PreparedPlaylistMove, Error> {
    #[derive(Clone)]
    struct Folder {
        name: String,
        path: Vec<String>,
        count: usize,
        count_range: std::ops::Range<usize>,
        children: Vec<String>,
        close_start: Option<usize>,
        playlist_insert_index: Option<usize>,
        empty_range: Option<std::ops::Range<usize>>,
    }
    #[derive(Clone)]
    struct PlaylistNode {
        name: String,
        parent: Option<usize>,
        range: Option<std::ops::Range<usize>>,
    }
    struct Frame {
        kind: String,
        folder: Option<usize>,
        playlist: Option<usize>,
        start: usize,
    }

    let before = preflight_export(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut folders: Vec<Folder> = Vec::new();
    let mut playlists: Vec<PlaylistNode> = Vec::new();
    let mut playlist_count = 0usize;

    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let kind = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = stack
                    .last()
                    .and_then(|frame| (frame.kind == "0").then_some(frame.folder).flatten());
                if kind == "0" {
                    let raw_count = attr(&e, "Count")?;
                    let count = raw_count
                        .parse::<usize>()
                        .map_err(|_| Error("invalid folder Count".into()))?;
                    let folder = if stack.is_empty() {
                        None
                    } else {
                        let index = folders.len();
                        let mut path = parent
                            .map(|parent| folders[parent].path.clone())
                            .unwrap_or_default();
                        path.push(name.clone());
                        let count_range = attribute_value_range(&e, start, "Count", &raw_count)?;
                        folders.push(Folder {
                            name: name.clone(),
                            path,
                            count,
                            count_range,
                            children: Vec::new(),
                            close_start: None,
                            playlist_insert_index: None,
                            empty_range: None,
                        });
                        if let Some(parent) = parent {
                            folders[parent].children.push(name.clone());
                        }
                        Some(index)
                    };
                    stack.push(Frame {
                        kind,
                        folder,
                        playlist: None,
                        start,
                    });
                } else if kind == "1" {
                    if stack.last().is_none_or(|frame| frame.kind != "0") {
                        return Err(Error("playlist is not a direct folder child".into()));
                    }
                    if let Some(parent) = parent {
                        folders[parent].children.push(name.clone());
                    }
                    let index = playlist_count;
                    playlist_count += 1;
                    playlists.push(PlaylistNode {
                        name: name.clone(),
                        parent,
                        range: None,
                    });
                    stack.push(Frame {
                        kind,
                        folder: None,
                        playlist: Some(index),
                        start,
                    });
                } else {
                    return Err(Error(format!("unsupported playlist NODE Type={kind}")));
                }
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let kind = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = stack
                    .last()
                    .and_then(|frame| (frame.kind == "0").then_some(frame.folder).flatten());
                if kind == "0" {
                    let raw_count = attr(&e, "Count")?;
                    let count = raw_count
                        .parse::<usize>()
                        .map_err(|_| Error("invalid folder Count".into()))?;
                    if count != 0 {
                        return Err(Error("empty folder Count is not zero".into()));
                    }
                    if let Some(parent) = parent {
                        folders[parent].children.push(name.clone());
                    }
                    if !stack.is_empty() {
                        let mut path = parent
                            .map(|parent| folders[parent].path.clone())
                            .unwrap_or_default();
                        path.push(name.clone());
                        let count_range = attribute_value_range(&e, start, "Count", &raw_count)?;
                        folders.push(Folder {
                            name: name.clone(),
                            path,
                            count,
                            count_range,
                            children: Vec::new(),
                            close_start: None,
                            playlist_insert_index: Some(playlist_count),
                            empty_range: Some(start..end),
                        });
                    }
                } else if kind == "1" {
                    if let Some(parent) = parent {
                        folders[parent].children.push(name.clone());
                    }
                    playlists.push(PlaylistNode {
                        name,
                        parent,
                        range: Some(start..end),
                    });
                    playlist_count += 1;
                } else {
                    return Err(Error(format!("unsupported playlist NODE Type={kind}")));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                let frame = stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing NODE".into()))?;
                if let Some(index) = frame.playlist {
                    playlists[index].range = Some(frame.start..end);
                }
                if let Some(index) = frame.folder {
                    folders[index].close_start = Some(start);
                    folders[index].playlist_insert_index = Some(playlist_count);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if request.source_parent_index == request.target_parent_index {
        return Err(Error("source and target folders must differ".into()));
    }
    let source = playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is out of range".into()))?;
    if source.name != request.expected_playlist_name
        || source.parent != Some(request.source_parent_index)
    {
        return Err(Error("playlist source guard changed".into()));
    }
    let expected_playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist is not a paired supported NODE".into()))?;
    if expected_playlist.name != request.expected_playlist_name
        || expected_playlist.tracks != request.expected_members
    {
        return Err(Error("playlist name or membership guard changed".into()));
    }
    let source_folder = folders
        .get(request.source_parent_index)
        .ok_or_else(|| Error("source folder ordinal is out of range".into()))?;
    let target_folder = folders
        .get(request.target_parent_index)
        .ok_or_else(|| Error("target folder ordinal is out of range".into()))?;
    if source_folder.name != request.expected_source_parent_name
        || target_folder.name != request.expected_target_parent_name
        || source_folder.close_start.is_none()
        || (target_folder.close_start.is_none() && target_folder.empty_range.is_none())
    {
        return Err(Error("source or target folder guard changed".into()));
    }
    for folder in [source_folder, target_folder] {
        if folder.count != folder.children.len() {
            return Err(Error("folder Count does not match direct children".into()));
        }
        let unique: HashSet<_> = folder.children.iter().collect();
        if unique.len() != folder.children.len() {
            return Err(Error("ambiguous duplicate sibling NODE Names".into()));
        }
    }
    if target_folder.children != request.expected_target_order {
        return Err(Error("target folder child order changed".into()));
    }
    if target_folder
        .children
        .contains(&request.expected_playlist_name)
    {
        return Err(Error(
            "target already has a child with playlist Name".into(),
        ));
    }
    let source_node = source
        .range
        .as_ref()
        .ok_or_else(|| Error("playlist node range is missing".into()))?;
    let target_insert_index = target_folder
        .playlist_insert_index
        .ok_or_else(|| Error("target folder close position is missing".into()))?;
    let source_count = source_folder
        .count
        .checked_sub(1)
        .ok_or_else(|| Error("source folder Count is zero".into()))?;
    let target_count = target_folder
        .count
        .checked_add(1)
        .ok_or_else(|| Error("target folder Count overflow".into()))?;

    let mut changes = vec![
        (source_node.clone(), Vec::new()),
        (
            source_folder.count_range.clone(),
            source_count.to_string().into_bytes(),
        ),
    ];
    if let Some(target_range) = target_folder.empty_range.as_ref() {
        let line_start = bytes[..target_range.start]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let indent = &bytes[line_start..target_range.start];
        let indent = if indent.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
            indent
        } else {
            &[]
        };
        let mut expanded = bytes[target_range.clone()].to_vec();
        let count_range = target_folder.count_range.start - target_range.start
            ..target_folder.count_range.end - target_range.start;
        expanded.splice(count_range, target_count.to_string().bytes());
        let slash = expanded
            .windows(2)
            .rposition(|pair| pair == b"/>")
            .ok_or_else(|| Error("empty target folder is not self-closing".into()))?;
        expanded.splice(slash..slash + 2, *b">");
        expanded.extend_from_slice(b"\n");
        expanded.extend_from_slice(indent);
        expanded.extend_from_slice(&bytes[source_node.clone()]);
        expanded.extend_from_slice(b"\n");
        expanded.extend_from_slice(indent);
        expanded.extend_from_slice(b"</NODE>");
        changes.push((target_range.clone(), expanded));
    } else {
        let target_close = target_folder.close_start.unwrap();
        let line_start = bytes[..target_close]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let parent_indent = &bytes[line_start..target_close];
        let parent_indent = if parent_indent
            .iter()
            .all(|byte| matches!(byte, b' ' | b'\t'))
        {
            parent_indent
        } else {
            &[]
        };
        let mut moved_node = Vec::new();
        moved_node.extend_from_slice(b"\n");
        moved_node.extend_from_slice(parent_indent);
        moved_node.extend_from_slice(&bytes[source_node.clone()]);
        moved_node.extend_from_slice(b"\n");
        moved_node.extend_from_slice(parent_indent);
        changes.push((
            target_folder.count_range.clone(),
            target_count.to_string().into_bytes(),
        ));
        changes.push((target_close..target_close, moved_node));
    }
    changes.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in changes {
        output.splice(range, replacement);
    }

    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    let mut moved = expected.remove(request.playlist_index);
    moved.path = target_folder.path.clone();
    moved.path.push(moved.name.clone());
    let insertion_index =
        target_insert_index - usize::from(request.playlist_index < target_insert_index);
    expected.insert(insertion_index, moved);
    if after.selected_tracks != before.selected_tracks || after.selected_playlists != expected {
        return Err(Error(
            "playlist move changed unrelated XML semantics".into(),
        ));
    }
    Ok(PreparedPlaylistMove { bytes: output })
}
