// --- synthetic DeviceSQL builder (one page per table, up to 16 rows per page) ---

fn encoded_string(value: &str) -> Vec<u8> {
    assert!(value.is_ascii() && value.len() < 127);
    let mut encoded = vec![((value.len() + 1) * 2 + 1) as u8];
    encoded.extend_from_slice(value.as_bytes());
    encoded
}

fn add_table(bytes: &mut [u8], table_index: usize, table_type: u32, rows: &[Vec<u8>]) {
    assert!(rows.len() <= 16);
    let page_index = table_index + 1;
    let dir = 0x1c + table_index * 16;
    bytes[dir..dir + 4].copy_from_slice(&table_type.to_le_bytes());
    bytes[dir + 8..dir + 12].copy_from_slice(&(page_index as u32).to_le_bytes());
    bytes[dir + 12..dir + 16].copy_from_slice(&(page_index as u32).to_le_bytes());
    let page = page_index * 4096;
    bytes[page + 0x18] = rows.len() as u8;
    let mut heap = 0usize;
    for (i, row) in rows.iter().enumerate() {
        bytes[page + 0x28 + heap..page + 0x28 + heap + row.len()].copy_from_slice(row);
        let slot = page + 4096 - 6 - 2 * i;
        bytes[slot..slot + 2].copy_from_slice(&(heap as u16).to_le_bytes());
        heap += (row.len() + 3) & !3;
    }
    let present = ((1u32 << rows.len()) - 1) as u16;
    bytes[page + 4092..page + 4094].copy_from_slice(&present.to_le_bytes());
}

/// Optional track-row metadata (DeviceSQL track row offsets per the pinned format
/// reference; zero / empty = no value).
#[derive(Default)]
struct Meta {
    sample_rate: u32,
    file_size: u32,
    key_id: u32,
    label_id: u32,
    bit_rate: u32,
    track_number: u32,
    genre_id: u32,
    disc_number: u16,
    year: u16,
    rating: u8,
    date_added: &'static str,
    comment: &'static str,
    analyze_path: &'static str,
}

fn track_row_with(
    id: u32,
    artist_id: u32,
    tempo: u32,
    duration: u16,
    title: &str,
    path: &str,
    meta: &Meta,
) -> Vec<u8> {
    let mut row = vec![0u8; 0x88];
    for (at, value) in [
        (0x08, meta.sample_rate),
        (0x10, meta.file_size),
        (0x20, meta.key_id),
        (0x28, meta.label_id),
        (0x30, meta.bit_rate),
        (0x34, meta.track_number),
        (0x3c, meta.genre_id),
    ] {
        row[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    row[0x4c..0x4e].copy_from_slice(&meta.disc_number.to_le_bytes());
    row[0x50..0x52].copy_from_slice(&meta.year.to_le_bytes());
    row[0x59] = meta.rating;
    row[0x48..0x4c].copy_from_slice(&id.to_le_bytes());
    row[0x44..0x48].copy_from_slice(&artist_id.to_le_bytes());
    row[0x38..0x3c].copy_from_slice(&tempo.to_le_bytes());
    row[0x54..0x56].copy_from_slice(&duration.to_le_bytes());
    let mut strings = vec![String::new(); 21];
    strings[10] = meta.date_added.into();
    strings[14] = meta.analyze_path.into();
    strings[16] = meta.comment.into();
    strings[17] = title.into();
    strings[20] = path.into();
    let mut offset = 0x88u16;
    let mut data = Vec::new();
    for (index, value) in strings.iter().enumerate() {
        row[0x5e + index * 2..0x60 + index * 2].copy_from_slice(&offset.to_le_bytes());
        let encoded = encoded_string(value);
        offset += encoded.len() as u16;
        data.extend_from_slice(&encoded);
    }
    row.extend(data);
    row
}

/// Genre and label rows: id at 0, name at 4. Key rows: id at 0, name at 8.
fn id_name_row(id: u32, name_at: usize, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; name_at];
    row[0..4].copy_from_slice(&id.to_le_bytes());
    row.extend(encoded_string(name));
    row
}

fn artist_row(id: u32, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; 10];
    row[0..2].copy_from_slice(&0x60u16.to_le_bytes());
    row[4..8].copy_from_slice(&id.to_le_bytes());
    row[9] = 10;
    row.extend(encoded_string(name));
    row
}

fn node_row(parent: u32, id: u32, sort: u32, is_folder: bool, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; 20];
    row[0..4].copy_from_slice(&parent.to_le_bytes());
    row[8..12].copy_from_slice(&sort.to_le_bytes());
    row[12..16].copy_from_slice(&id.to_le_bytes());
    row[16..20].copy_from_slice(&u32::from(is_folder).to_le_bytes());
    row.extend(encoded_string(name));
    row
}

fn entry_row(index: u32, track_id: u32, playlist_id: u32) -> Vec<u8> {
    let mut row = vec![0u8; 12];
    row[0..4].copy_from_slice(&index.to_le_bytes());
    row[4..8].copy_from_slice(&track_id.to_le_bytes());
    row[8..12].copy_from_slice(&playlist_id.to_le_bytes());
    row
}

/// 3 tracks / 2 folders / 4 playlists / 6 memberships. Sibling order comes from the
/// sort keys (not ids or row order); member order from entry_index (rows shuffled).
/// Track metadata: "Song A" carries every bounded field; "Song <B>" has an unknown
/// genre id, an unknown key id, a malformed date and an out-of-range rating (all
/// unresolved, none written); "Song C" has no values and a label id naming an empty
/// label (absent, not an empty attribute).
fn synthetic_pdb(extra_entry: Option<Vec<u8>>) -> Vec<u8> {
    synthetic_pdb_with(extra_entry, "/Contents/C.flac")
}

/// `synthetic_pdb` with "Song C" (no beat grid) located at `c_path`.
fn synthetic_pdb_with(extra_entry: Option<Vec<u8>>, c_path: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; 8 * 4096];
    bytes[4..8].copy_from_slice(&4096u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&7u32.to_le_bytes());
    add_table(
        &mut bytes,
        0,
        0,
        &[
            track_row_with(
                42,
                1,
                12_800,
                180,
                "Song A",
                "/Contents/Song A.mp3",
                &Meta {
                    sample_rate: 48_000,
                    file_size: 7_340_032,
                    key_id: 3,
                    label_id: 4,
                    bit_rate: 320,
                    track_number: 7,
                    genre_id: 2,
                    disc_number: 1,
                    year: 2019,
                    rating: 5,
                    date_added: "2025-12-31",
                    comment: "Synth <c> & \"d\"",
                    analyze_path: ANLZ_A,
                },
            ),
            track_row_with(
                7,
                0,
                9_050,
                61,
                "Song <B> & \"Q\"",
                "file://localhost/b.wav",
                &Meta {
                    genre_id: 77,
                    key_id: 78,
                    rating: 9,
                    date_added: "yesterday",
                    bit_rate: 128,
                    analyze_path: ANLZ_B,
                    ..Meta::default()
                },
            ),
            track_row_with(
                9,
                1,
                0,
                0,
                "Song C",
                c_path,
                &Meta {
                    label_id: 5,
                    ..Meta::default()
                },
            ),
        ],
    );
    add_table(&mut bytes, 4, 1, &[id_name_row(2, 4, "Synth Genre")]);
    add_table(
        &mut bytes,
        5,
        4,
        &[id_name_row(4, 4, "Synth Label"), id_name_row(5, 4, "")],
    );
    add_table(&mut bytes, 6, 5, &[id_name_row(3, 8, "11B")]);
    add_table(&mut bytes, 1, 2, &[artist_row(1, "Synthetic Artist")]);
    add_table(
        &mut bytes,
        2,
        7,
        &[
            node_row(0, 10, 2, true, "Folder"),
            node_row(10, 11, 1, false, "Inner Second"),
            node_row(10, 12, 0, false, "Inner First"),
            node_row(0, 13, 1, false, "Top"),
            node_row(10, 14, 2, true, "Empty Folder"),
            node_row(0, 15, 3, false, "Empty Playlist"),
        ],
    );
    let mut entries = vec![
        entry_row(2, 42, 13),
        entry_row(0, 7, 13),
        entry_row(1, 42, 13),
        entry_row(0, 9, 12),
        entry_row(1, 7, 11),
        entry_row(0, 42, 11),
    ];
    entries.extend(extra_entry);
    add_table(&mut bytes, 3, 8, &entries);
    bytes
}
