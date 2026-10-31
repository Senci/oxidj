use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use quick_xml as _;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn source_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidj-synthetic-pdb-{}-{nonce}.pdb",
        std::process::id()
    ))
}

fn fill_page(bytes: &mut [u8], page_index: usize, row: &[u8]) {
    let page = page_index * 4096;
    bytes[page + 0x18] = 1;
    bytes[page + 0x28..page + 0x28 + row.len()].copy_from_slice(row);
    bytes[page + 4090..page + 4092].copy_from_slice(&0u16.to_le_bytes());
    bytes[page + 4092..page + 4094].copy_from_slice(&1u16.to_le_bytes());
}

fn add_page(bytes: &mut [u8], page_index: usize, table_index: usize, table_type: u32, row: &[u8]) {
    let dir = 0x1c + table_index * 16;
    bytes[dir..dir + 4].copy_from_slice(&table_type.to_le_bytes());
    bytes[dir + 8..dir + 12].copy_from_slice(&(page_index as u32).to_le_bytes());
    bytes[dir + 12..dir + 16].copy_from_slice(&(page_index as u32).to_le_bytes());
    fill_page(bytes, page_index, row);
}

fn encoded_string(value: &str) -> Vec<u8> {
    assert!(value.is_ascii() && value.len() < 127);
    let mut encoded = vec![((value.len() + 1) * 2 + 1) as u8];
    encoded.extend_from_slice(value.as_bytes());
    encoded
}

fn synthetic_track() -> Vec<u8> {
    let mut row = vec![0u8; 0x88];
    row[0x48..0x4c].copy_from_slice(&42u32.to_le_bytes());
    row[0x38..0x3c].copy_from_slice(&12800u32.to_le_bytes());
    row[0x54..0x56].copy_from_slice(&180u16.to_le_bytes());
    let mut strings = vec![String::new(); 21];
    strings[17] = "Synthetic Song".into();
    strings[20] = "file://localhost/Synthetic Song.wav".into();
    let mut data = Vec::new();
    let mut offset = 0x88u16;
    for (index, value) in strings.iter().enumerate() {
        row[0x5e + index * 2..0x60 + index * 2].copy_from_slice(&offset.to_le_bytes());
        let encoded = encoded_string(value);
        offset += encoded.len() as u16;
        data.extend_from_slice(&encoded);
    }
    row.extend(data);
    row
}

fn synthetic_node(parent: u32, id: u32, sort: u32, is_folder: bool, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; 20];
    row[0..4].copy_from_slice(&parent.to_le_bytes());
    row[8..12].copy_from_slice(&sort.to_le_bytes());
    row[12..16].copy_from_slice(&id.to_le_bytes());
    row[16..20].copy_from_slice(&u32::from(is_folder).to_le_bytes());
    row.extend(encoded_string(name));
    row
}

fn synthetic_entry(track_id: u32, playlist_id: u32) -> Vec<u8> {
    let mut row = vec![0u8; 12];
    row[4..8].copy_from_slice(&track_id.to_le_bytes());
    row[8..12].copy_from_slice(&playlist_id.to_le_bytes());
    row
}

fn synthetic_pdb() -> Vec<u8> {
    let mut bytes = vec![0u8; 5 * 4096];
    bytes[4..8].copy_from_slice(&4096u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&3u32.to_le_bytes());
    add_page(&mut bytes, 1, 0, 0, &synthetic_track());
    add_page(
        &mut bytes,
        2,
        1,
        7,
        &synthetic_node(0, 10, 0, true, "Synthetic Folder"),
    );
    bytes[0x1c + 16 + 12..0x1c + 16 + 16].copy_from_slice(&3u32.to_le_bytes());
    bytes[2 * 4096 + 12..2 * 4096 + 16].copy_from_slice(&3u32.to_le_bytes());
    fill_page(
        &mut bytes,
        3,
        &synthetic_node(10, 11, 0, false, "Synthetic Playlist"),
    );
    add_page(&mut bytes, 4, 2, 8, &synthetic_entry(42, 11));
    bytes
}

#[test]
fn pdb_preview_cli_reads_synthetic_tracks_playlist_hierarchy_and_order_without_modification() {
    let path = source_path();
    let bytes = synthetic_pdb();
    std::fs::write(&path, &bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "pdb-preview", "--pdb"])
        .arg(&path)
        .output()
        .expect("run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Track\t42\tSynthetic Song\tNone\tNone\t12800\t180"),
        "{stdout}"
    );
    assert!(stdout.contains("Synthetic Folder"), "{stdout}");
    assert!(stdout.contains("Synthetic Playlist"), "{stdout}");
    assert!(stdout.contains("Tracks=[42]"), "{stdout}");
    assert!(
        stdout.contains("Path=[\"Synthetic Folder\", \"Synthetic Playlist\"]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("file://localhost/Synthetic Song.wav"),
        "{stdout}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn pdb_preview_cli_rejects_malformed_synthetic_source() {
    let path = source_path();
    let bytes = b"malformed synthetic PDB";
    std::fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "pdb-preview", "--pdb"])
        .arg(&path)
        .output()
        .expect("run CLI");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("preview rejected input"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}
