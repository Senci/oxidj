const M7_TRACKS: usize = 1_330;
const M7_PLAYLISTS: usize = 20;
const M7_MEMBERSHIPS: usize = 901;
/// Tracks whose ANLZ analysis files exist; the rest have no analysis path.
const M7_ANLZ_TRACKS: usize = 100;

/// Split `rows` into DeviceSQL pages of at most 16 rows that also fit the page:
/// 0x28 header + row bytes + one 0x24 row-offset group (the format stores offsets
/// from the end of the page, so oversized rows must start a new page).
fn pack_rows(rows: &[Vec<u8>]) -> Vec<(usize, usize)> {
    let mut pages = Vec::new();
    let mut index = 0usize;
    while index < rows.len() {
        let start = index;
        let mut count = 0usize;
        let mut used = 0x28usize;
        while index < rows.len() && count < 16 {
            let padded = (rows[index].len() + 3) & !3;
            if used + padded + 0x24 > 4096 {
                break;
            }
            used += padded;
            count += 1;
            index += 1;
        }
        pages.push((start, index));
    }
    pages
}

fn table_page_count(rows: &[Vec<u8>]) -> usize {
    pack_rows(rows).len()
}

/// Multi-page DeviceSQL table writer. `page_offset` is the next free page index
/// (page 0 is the file header) and advances by the number of pages written.
fn add_table_pages(
    bytes: &mut [u8],
    table_index: usize,
    table_type: u32,
    rows: &[Vec<u8>],
    page_offset: &mut usize,
) {
    let pages = pack_rows(rows);
    if pages.is_empty() {
        return;
    }
    let first_page = *page_offset;
    let last_page = *page_offset + pages.len() - 1;
    let dir = 0x1c + table_index * 16;
    bytes[dir..dir + 4].copy_from_slice(&table_type.to_le_bytes());
    bytes[dir + 8..dir + 12].copy_from_slice(&(first_page as u32).to_le_bytes());
    bytes[dir + 12..dir + 16].copy_from_slice(&(last_page as u32).to_le_bytes());
    for (page_index, (start, end)) in pages.iter().enumerate() {
        let page = (first_page + page_index) * 4096;
        let page_rows = &rows[*start..*end];
        bytes[page + 0x04..page + 0x08]
            .copy_from_slice(&((first_page + page_index) as u32).to_le_bytes());
        bytes[page + 0x08..page + 0x0c].copy_from_slice(&table_type.to_le_bytes());
        let next_page = if page_index + 1 < pages.len() {
            first_page + page_index + 1
        } else {
            0
        };
        bytes[page + 0x0c..page + 0x10].copy_from_slice(&(next_page as u32).to_le_bytes());
        bytes[page + 0x18] = page_rows.len() as u8;
        let mut heap = 0usize;
        for (i, row) in page_rows.iter().enumerate() {
            bytes[page + 0x28 + heap..page + 0x28 + heap + row.len()].copy_from_slice(row);
            let slot = page + 4096 - 6 - 2 * i;
            bytes[slot..slot + 2].copy_from_slice(&(heap as u16).to_le_bytes());
            heap += (row.len() + 3) & !3;
        }
        let present = ((1u32 << page_rows.len()) - 1) as u16;
        bytes[page + 4092..page + 4094].copy_from_slice(&present.to_le_bytes());
    }
    *page_offset += pages.len();
}

/// Track row with dynamic strings (the small `track_row_with` uses `&'static str`).
#[allow(clippy::too_many_arguments)]
fn m7_track_row(
    id: u32,
    artist_id: u32,
    tempo: u32,
    duration: u16,
    title: &str,
    path: &str,
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
    date_added: &str,
    comment: &str,
    analyze_path: &str,
) -> Vec<u8> {
    let mut row = vec![0u8; 0x88];
    for (at, value) in [
        (0x08, sample_rate),
        (0x10, file_size),
        (0x20, key_id),
        (0x28, label_id),
        (0x30, bit_rate),
        (0x34, track_number),
        (0x3c, genre_id),
    ] {
        row[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    row[0x4c..0x4e].copy_from_slice(&disc_number.to_le_bytes());
    row[0x50..0x52].copy_from_slice(&year.to_le_bytes());
    row[0x59] = rating;
    row[0x48..0x4c].copy_from_slice(&id.to_le_bytes());
    row[0x44..0x48].copy_from_slice(&artist_id.to_le_bytes());
    row[0x38..0x3c].copy_from_slice(&tempo.to_le_bytes());
    row[0x54..0x56].copy_from_slice(&duration.to_le_bytes());
    let mut strings = vec![String::new(); 21];
    strings[10] = date_added.to_string();
    strings[14] = analyze_path.to_string();
    strings[16] = comment.to_string();
    strings[17] = title.to_string();
    strings[20] = path.to_string();
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

/// Deterministic large library: 1,330 tracks, 20 playlists (18 at ROOT, one in
/// each of two folders), 901 ordered memberships. Every audited field resolves;
/// the first 100 tracks carry ANLZ analysis, the rest have no analysis path.
fn synthetic_pdb_m7() -> Vec<u8> {
    let tracks: Vec<Vec<u8>> = (0..M7_TRACKS)
        .map(|i| {
            let id = (i + 1) as u32;
            let analyze_path = if i < M7_ANLZ_TRACKS {
                format!("/PIONEER/USBANLZ/P001/{id:08X}/ANLZ0000.DAT")
            } else {
                String::new()
            };
            m7_track_row(
                id,
                ((i % 10) + 1) as u32,
                12_000 + ((i % 100) * 100) as u32,
                (180 + i % 60) as u16,
                &format!("Track {id}"),
                &format!("/Contents/Track {id}.mp3"),
                44_100,
                5_000_000 + id,
                ((i % 8) + 1) as u32,
                ((i % 4) + 1) as u32,
                320,
                ((i % 12) + 1) as u32,
                ((i % 3) + 1) as u32,
                1,
                (2000 + i % 25) as u16,
                ((i % 5) + 1) as u8,
                "2025-01-01",
                &format!("Comment {id}"),
                &analyze_path,
            )
        })
        .collect();

    let artists: Vec<Vec<u8>> = (1..=10)
        .map(|id| artist_row(id, &format!("Artist {id}")))
        .collect();

    let genres: Vec<Vec<u8>> = (1..=3)
        .map(|id| id_name_row(id, 4, &format!("Genre {id}")))
        .collect();

    let labels: Vec<Vec<u8>> = (1..=4)
        .map(|id| id_name_row(id, 4, &format!("Label {id}")))
        .collect();

    let keys: Vec<Vec<u8>> = (1..=8)
        .map(|id| id_name_row(id, 8, &format!("Key {id}")))
        .collect();

    let mut nodes_rows = Vec::new();
    for j in 0..18 {
        nodes_rows.push(node_row(
            0,
            (100 + j) as u32,
            j as u32,
            false,
            &format!("Playlist {}", j + 1),
        ));
    }
    nodes_rows.push(node_row(0, 200, 18, true, "Folder A"));
    nodes_rows.push(node_row(0, 201, 19, true, "Folder B"));
    nodes_rows.push(node_row(200, 118, 0, false, "Playlist 19"));
    nodes_rows.push(node_row(201, 119, 0, false, "Playlist 20"));

    let mut entries = Vec::new();
    for j in 0..18u32 {
        for k in 0..45u32 {
            entries.push(entry_row(k, j * 45 + k + 1, 100 + j));
        }
    }
    for k in 0..46u32 {
        entries.push(entry_row(k, 811 + k, 118));
    }
    for k in 0..45u32 {
        entries.push(entry_row(k, 857 + k, 119));
    }

    let total_pages = 1
        + table_page_count(&tracks)
        + table_page_count(&artists)
        + table_page_count(&genres)
        + table_page_count(&labels)
        + table_page_count(&keys)
        + table_page_count(&nodes_rows)
        + table_page_count(&entries);
    let mut bytes = vec![0u8; total_pages * 4096];
    bytes[4..8].copy_from_slice(&4096u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&7u32.to_le_bytes());
    let mut page_offset = 1usize;
    add_table_pages(&mut bytes, 0, 0, &tracks, &mut page_offset);
    add_table_pages(&mut bytes, 1, 2, &artists, &mut page_offset);
    add_table_pages(&mut bytes, 2, 1, &genres, &mut page_offset);
    add_table_pages(&mut bytes, 3, 4, &labels, &mut page_offset);
    add_table_pages(&mut bytes, 4, 5, &keys, &mut page_offset);
    add_table_pages(&mut bytes, 5, 7, &nodes_rows, &mut page_offset);
    add_table_pages(&mut bytes, 6, 8, &entries, &mut page_offset);
    bytes
}

/// ANLZ `.DAT` files for the first [`M7_ANLZ_TRACKS`] tracks (grid + one hot cue).
fn write_analysis_m7(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    (0..M7_ANLZ_TRACKS)
        .map(|i| {
            let id = (i + 1) as u32;
            let rel = format!("/PIONEER/USBANLZ/P001/{id:08X}/ANLZ0000.DAT");
            let path = root.join(rel.trim_start_matches('/'));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let bytes = anlz_file(&[
                anlz_path(&format!("/Contents/Track {id}.mp3")),
                anlz_grid(&[(1, 12_000, 0), (2, 12_000, 500)]),
                anlz_cues(1, &[(1, 1, 1_500, u32::MAX)]),
            ]);
            std::fs::write(&path, &bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

