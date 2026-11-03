//! M7 slice, synthetic-only: PDB → `catalog import-library` → new catalog →
//! `catalog export-library-xml` → new XML, checked by an independent quick-xml decoder
//! against values hard-coded from the fixture definition below.
use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use quick_xml::{Reader, events::Event};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch() -> PathBuf {
    // Parallel tests can read the same clock value; the counter keeps names unique.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "oxidj-library-import-{}-{nonce}-{seq}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

fn s(value: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(value)
}

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
                "/Contents/C.flac",
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

// --- synthetic ANLZ builder (big-endian tagged sections per the public format
// reference; independent of the production decoder) ---

const ANLZ_A: &str = "/PIONEER/USBANLZ/P001/0000002A/ANLZ0000.DAT";
const ANLZ_B: &str = "/PIONEER/USBANLZ/P001/00000007/ANLZ0000.DAT";

fn be(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

fn anlz_section(tag: &[u8; 4], header_rest: &[u8], body: &[u8]) -> Vec<u8> {
    let header = 12 + header_rest.len() as u32;
    [
        &tag[..],
        &be(header),
        &be(header + body.len() as u32),
        header_rest,
        body,
    ]
    .concat()
}

fn anlz_file(sections: &[Vec<u8>]) -> Vec<u8> {
    let body = sections.concat();
    [
        &b"PMAI"[..],
        &be(28),
        &be(28 + body.len() as u32),
        &[0; 16],
        &body,
    ]
    .concat()
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_be_bytes)
        .collect()
}

fn anlz_path(path: &str) -> Vec<u8> {
    let body = utf16(path);
    anlz_section(b"PPTH", &be(body.len() as u32), &body)
}

/// (beat 1..=4, BPM×100, time ms).
fn anlz_grid(beats: &[(u16, u16, u32)]) -> Vec<u8> {
    let rest = [be(0), be(0x0008_0000), be(beats.len() as u32)].concat();
    let body: Vec<u8> = beats
        .iter()
        .flat_map(|(b, t, ms)| [&b.to_be_bytes()[..], &t.to_be_bytes(), &be(*ms)].concat())
        .collect();
    anlz_section(b"PQTZ", &rest, &body)
}

/// PCOB list of PCPT entries (hot, type 1 point / 2 loop, time ms, loop end ms).
fn anlz_cues(list: u32, cues: &[(u32, u8, u32, u32)]) -> Vec<u8> {
    let entries: Vec<u8> = cues
        .iter()
        .flat_map(|&(hot, kind, ms, end)| {
            let mut e = [
                &b"PCPT"[..],
                &be(28),
                &be(56),
                &be(hot),
                &be(0),
                &be(0x10000),
            ]
            .concat();
            e.extend([0xff, 0xff, 0xff, 0xff, kind, 0, 0x03, 0xe8]);
            e.extend(be(ms));
            e.extend(be(end));
            e.resize(56, 0);
            e
        })
        .collect();
    let rest = [
        &be(list)[..],
        &0u16.to_be_bytes(),
        &(cues.len() as u16).to_be_bytes(),
        &be(u32::MAX),
    ]
    .concat();
    anlz_section(b"PCOB", &rest, &entries)
}

/// PCO2 list of PCP2 entries (hot, type, time ms, loop end ms, comment, rgb).
/// (hot, type, time ms, loop end ms, comment, rgb).
type ExtCue<'a> = (u32, u8, u32, u32, &'a str, Option<[u8; 3]>);

fn anlz_ext_cues(list: u32, cues: &[ExtCue<'_>]) -> Vec<u8> {
    let entries: Vec<u8> = cues
        .iter()
        .flat_map(|&(hot, kind, ms, end, comment, rgb)| {
            let text = if comment.is_empty() {
                vec![]
            } else {
                utf16(comment)
            };
            let mut e = [&be(hot)[..], &[kind, 0, 0, 0], &be(ms), &be(end), &[0; 12]].concat();
            e.extend(be(text.len() as u32));
            e.extend(text);
            if let Some([r, g, b]) = rgb {
                e.extend([1, r, g, b]);
            }
            [&b"PCP2"[..], &be(16), &be(12 + e.len() as u32), &e].concat()
        })
        .collect();
    let rest = [&be(list)[..], &(cues.len() as u16).to_be_bytes(), &[0, 0]].concat();
    anlz_section(b"PCO2", &rest, &entries)
}

/// Writes the analysis files referenced by `synthetic_pdb` under `root`:
/// "Song A" has a 3-beat grid with a tempo change, DAT hot cue A, and an EXT
/// superset (named, coloured hot cue A; memory cue; hot cue C loop); "Song <B>"
/// has an analysis file with an unknown section (unresolved); "Song C" has none.
fn write_analysis(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let a_path = anlz_path("/Contents/Song A.mp3");
    let files = vec![
        (
            ANLZ_A.to_string(),
            anlz_file(&[
                a_path.clone(),
                anlz_grid(&[(1, 12_800, 25), (2, 12_800, 494), (3, 13_050, 963)]),
                anlz_cues(1, &[(1, 1, 1_500, u32::MAX)]),
                anlz_cues(0, &[]),
            ]),
        ),
        (
            ANLZ_A.replace(".DAT", ".EXT"),
            anlz_file(&[
                a_path,
                anlz_ext_cues(
                    1,
                    &[
                        (1, 1, 1_500, u32::MAX, "Drop & <go>", Some([40, 226, 20])),
                        (3, 2, 64_000, 72_250, "", Some([255, 0, 128])),
                    ],
                ),
                anlz_ext_cues(0, &[(0, 1, 30_001, u32::MAX, "", None)]),
            ]),
        ),
        (
            ANLZ_B.to_string(),
            anlz_file(&[
                anlz_path("file://localhost/b.wav"),
                anlz_grid(&[(1, 9_050, 0)]),
                anlz_section(b"PZZZ", &[], &[1, 2, 3]),
            ]),
        ),
    ];
    files
        .into_iter()
        .map(|(rel, bytes)| {
            let path = root.join(rel.trim_start_matches('/'));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

// --- independent decoder of the exported XML ---

#[derive(Debug, PartialEq)]
enum Node {
    Folder(String, Vec<Node>),
    Playlist(String, Vec<String>),
}

/// (child element name, attributes).
type Child = (String, Vec<(String, String)>);

#[derive(Debug, PartialEq)]
struct Decoded {
    collection_entries: String,
    tracks: Vec<Vec<(String, String)>>,
    /// Per collection track: ordered (child element name, attributes).
    children: Vec<Vec<Child>>,
    root: Node,
}

fn attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .map(|a| {
            let a = a.unwrap();
            (
                String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                a.decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, e.decoder())
                    .unwrap()
                    .into_owned(),
            )
        })
        .collect()
}

fn get(attrs: &[(String, String)], key: &str) -> String {
    attrs.iter().find(|(k, _)| k == key).unwrap().1.clone()
}

fn decode(bytes: &[u8]) -> Decoded {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut tracks = Vec::new();
    let mut children: Vec<Vec<Child>> = Vec::new();
    let mut entries = String::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut in_collection = false;
    loop {
        let event = reader.read_event_into(&mut buf).unwrap();
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let a = attrs(e);
                match e.name().as_ref() {
                    b"COLLECTION" => {
                        entries = get(&a, "Entries");
                        in_collection = !empty;
                    }
                    b"TRACK" if in_collection => {
                        tracks.push(a);
                        children.push(vec![]);
                    }
                    name @ (b"TEMPO" | b"POSITION_MARK") if in_collection => children
                        .last_mut()
                        .expect("child outside track")
                        .push((String::from_utf8(name.to_vec()).unwrap(), a)),
                    b"TRACK" => match stack.last_mut() {
                        Some(Node::Playlist(_, keys)) => keys.push(get(&a, "Key")),
                        _ => panic!("member outside playlist"),
                    },
                    b"NODE" => {
                        let node = if get(&a, "Type") == "0" {
                            Node::Folder(get(&a, "Name"), vec![])
                        } else {
                            Node::Playlist(get(&a, "Name"), vec![])
                        };
                        if empty {
                            match stack.last_mut() {
                                Some(Node::Folder(_, children)) => children.push(node),
                                _ => panic!("orphan node"),
                            }
                        } else {
                            stack.push(node);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(ref e) => match e.name().as_ref() {
                b"COLLECTION" => in_collection = false,
                b"NODE" => {
                    let node = stack.pop().unwrap();
                    match stack.last_mut() {
                        Some(Node::Folder(_, children)) => children.push(node),
                        None => root = Some(node),
                        _ => panic!("node inside playlist"),
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Decoded {
        collection_entries: entries,
        tracks,
        children,
        root: root.unwrap(),
    }
}

fn kv(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn tempo(start: &str, bpm: &str, beat: &str) -> Child {
    (
        "TEMPO".into(),
        kv(&[
            ("Inizio", start),
            ("Bpm", bpm),
            ("Metro", "4/4"),
            ("Battito", beat),
        ]),
    )
}

fn expected() -> Decoded {
    let p = |name: &str, keys: &[&str]| {
        Node::Playlist(name.into(), keys.iter().map(|k| k.to_string()).collect())
    };
    Decoded {
        collection_entries: "3".into(),
        tracks: vec![
            kv(&[
                ("TrackID", "1"),
                ("Name", "Song A"),
                ("Artist", "Synthetic Artist"),
                ("Genre", "Synth Genre"),
                ("Size", "7340032"),
                ("TotalTime", "180"),
                ("DiscNumber", "1"),
                ("TrackNumber", "7"),
                ("Year", "2019"),
                ("AverageBpm", "128.00"),
                ("DateAdded", "2025-12-31"),
                ("BitRate", "320"),
                ("SampleRate", "48000"),
                ("Comments", "Synth <c> & \"d\""),
                ("Rating", "255"),
                ("Location", "file://localhost/Contents/Song%20A.mp3"),
                ("Tonality", "11B"),
                ("Label", "Synth Label"),
            ]),
            kv(&[
                ("TrackID", "2"),
                ("Name", "Song <B> & \"Q\""),
                ("TotalTime", "61"),
                ("AverageBpm", "90.50"),
                ("BitRate", "128"),
                ("Location", "file://localhost/b.wav"),
            ]),
            kv(&[
                ("TrackID", "3"),
                ("Name", "Song C"),
                ("Artist", "Synthetic Artist"),
                ("TotalTime", "0"),
                ("AverageBpm", "0.00"),
                ("Location", "file://localhost/Contents/C.flac"),
            ]),
        ],
        children: vec![
            vec![
                tempo("0.025", "128.00", "1"),
                tempo("0.494", "128.00", "2"),
                tempo("0.963", "130.50", "3"),
                (
                    "POSITION_MARK".into(),
                    kv(&[
                        ("Name", "Drop & <go>"),
                        ("Type", "0"),
                        ("Start", "1.500"),
                        ("Num", "0"),
                        ("Red", "40"),
                        ("Green", "226"),
                        ("Blue", "20"),
                    ]),
                ),
                (
                    "POSITION_MARK".into(),
                    kv(&[
                        ("Name", ""),
                        ("Type", "4"),
                        ("Start", "64.000"),
                        ("End", "72.250"),
                        ("Num", "2"),
                        ("Red", "255"),
                        ("Green", "0"),
                        ("Blue", "128"),
                    ]),
                ),
                (
                    "POSITION_MARK".into(),
                    kv(&[
                        ("Name", ""),
                        ("Type", "0"),
                        ("Start", "30.001"),
                        ("Num", "-1"),
                    ]),
                ),
            ],
            vec![],
            vec![],
        ],
        root: Node::Folder(
            "ROOT".into(),
            vec![
                p("Top", &["2", "1", "1"]),
                Node::Folder(
                    "Folder".into(),
                    vec![
                        p("Inner First", &["3"]),
                        p("Inner Second", &["1", "2"]),
                        Node::Folder("Empty Folder".into(), vec![]),
                    ],
                ),
                p("Empty Playlist", &[]),
            ],
        ),
    }
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn assert_aggregate_only(text: &str) {
    for private in [
        "Song",
        "Synthetic Artist",
        "Synth",
        "11B",
        "2025-12-31",
        "yesterday",
        "Empty Folder",
        "Empty Playlist",
        "Contents",
        "PIONEER",
        "USBANLZ",
        "Drop",
        "Inner",
        ".mp3",
        "oxidj-library-import",
    ] {
        assert!(
            !text.contains(private),
            "report leaked a source value: {text}"
        );
    }
}

#[test]
fn pdb_library_imports_into_new_catalog_and_exports_verified_xml_without_touching_source() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    let xml = dir.join("out.xml");

    let import = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]);
    let report = stdout(&import);
    assert_aggregate_only(&report);
    for line in [
        "Tracks: 3",
        "Folders: 2",
        "Playlists: 4",
        "Memberships: 6",
        "LocationsUriEncoded: 2",
        "SourceUnchanged: true",
        "CatalogMatchesSource: true",
        "NotCarried\tTRACK@Composer\t3",
        "NotCarried\tTRACK@PlayCount\t3",
        "NotCarried\tTRACK@Colour\t3",
        "Carried\tTRACK/TEMPO\t1",
        "AbsentInSource\tTEMPO\t2",
        "UnresolvedInSource\tTEMPO\t1",
        "Carried\tTRACK/POSITION_MARK\t1",
        "AbsentInSource\tPOSITION_MARK\t2",
        "UnresolvedInSource\tPOSITION_MARK\t1",
        "Carried\tTRACK@Artist\t2",
        "AbsentInSource\tArtist\t1",
        "Carried\tTRACK@Album\t0",
        "AbsentInSource\tAlbum\t3",
        "Carried\tTRACK@Genre\t1",
        "AbsentInSource\tGenre\t2",
        "UnresolvedInSource\tGenre\t1",
        "Carried\tTRACK@Label\t1",
        "AbsentInSource\tLabel\t2",
        "UnresolvedInSource\tLabel\t0",
        "Carried\tTRACK@Tonality\t1",
        "UnresolvedInSource\tTonality\t1",
        "Carried\tTRACK@BitRate\t2",
        "AbsentInSource\tBitRate\t1",
        "Carried\tTRACK@DateAdded\t1",
        "UnresolvedInSource\tDateAdded\t1",
        "Carried\tTRACK@Rating\t1",
        "UnresolvedInSource\tRating\t1",
        "Carried\tTRACK@Comments\t1",
        "Carried\tTRACK@Size\t1",
        "Carried\tTRACK@SampleRate\t1",
        "Carried\tTRACK@TrackNumber\t1",
        "Carried\tTRACK@DiscNumber\t1",
        "Carried\tTRACK@Year\t1",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }

    let export = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    let report = stdout(&export);
    assert_aggregate_only(&report);
    for line in [
        "Tracks: 3",
        "Folders: 2",
        "Playlists: 4",
        "Memberships: 6",
        "PreviewVerified: true",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }
    for carried in [
        "Genre",
        "Label",
        "Tonality",
        "Comments",
        "DateAdded",
        "Year",
        "Size",
        "BitRate",
        "SampleRate",
        "TrackNumber",
        "DiscNumber",
        "Rating",
    ] {
        assert!(
            !report
                .lines()
                .any(|l| l.starts_with(&format!("NotCarried\tTRACK@{carried}\t"))),
            "{carried} is carried and must not be reported as not carried"
        );
        assert!(
            report
                .lines()
                .any(|l| l.starts_with(&format!("Carried\tTRACK@{carried}\t"))),
            "missing carried count for {carried}"
        );
    }
    let bytes = std::fs::read(&xml).unwrap();
    assert_eq!(decode(&bytes), expected());

    // The existing read-only XML preview agrees on aggregates.
    let preview = oxidj_application::preview_rekordbox_xml(&xml).unwrap();
    assert_eq!(preview.tracks.len(), 3);
    assert_eq!(preview.playlists.len(), 4);
    assert_eq!(preview.folders.len(), 3); // ROOT + 2
    assert_eq!(
        preview
            .playlists
            .iter()
            .map(|p| p.tracks.len())
            .sum::<usize>(),
        6
    );

    assert_eq!(
        std::fs::read(&pdb).unwrap(),
        source,
        "source must be byte-identical"
    );
    for (path, bytes) in &analysis {
        assert_eq!(
            &std::fs::read(path).unwrap(),
            bytes,
            "analysis must be byte-identical"
        );
    }
    assert!(
        !report.lines().any(|l| l.starts_with("NotCarried\tTRACK/")),
        "TEMPO/POSITION_MARK are carried: {report}"
    );

    // New-file-only: a second export to the same path is refused and leaves it intact.
    let again = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!again.status.success());
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("refusing to overwrite existing output")
    );
    assert_eq!(std::fs::read(&xml).unwrap(), bytes);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn import_and_export_fail_closed_without_creating_or_replacing_files() {
    let dir = scratch();
    let pdb = dir.join("export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();

    // Existing catalog path: refused, untouched.
    let existing = dir.join("existing.sqlite");
    std::fs::write(&existing, b"sentinel").unwrap();
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        existing.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("refusing to reuse an existing catalog path")
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"sentinel");

    // Malformed source and dangling membership: refused, no catalog created.
    let malformed = dir.join("malformed.pdb");
    std::fs::write(&malformed, b"not a pdb").unwrap();
    let dangling = dir.join("dangling.pdb");
    std::fs::write(&dangling, synthetic_pdb(Some(entry_row(3, 99, 13)))).unwrap();
    for (src, reason) in [
        (
            &malformed,
            "PDB source was rejected by the read-only preview",
        ),
        (&dangling, "playlist member references a missing track"),
    ] {
        let catalog = dir.join("never.sqlite");
        let out = cli(&[
            s("catalog"),
            s("import-library"),
            s("--pdb"),
            src.as_os_str(),
            s("--catalog"),
            catalog.as_os_str(),
        ]);
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(reason), "{stderr}");
        assert_aggregate_only(&stderr);
        assert!(!catalog.exists());
        assert!(!Path::new(&format!("{}-wal", catalog.display())).exists());
    }

    // Requested OneLibrary cross-check that cannot be read: refused before any catalog.
    let catalog = dir.join("never.sqlite");
    let missing = dir.join("exportLibrary.db");
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
        s("--onelibrary-db"),
        missing.as_os_str(),
        s("--key-stdin"),
    ]);
    assert!(!out.status.success());
    assert!(!catalog.exists());

    // Export: missing catalog is refused and not created; existing output untouched.
    let xml = dir.join("out.xml");
    let out = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("catalog does not exist"));
    assert!(!catalog.exists() && !xml.exists());

    let good = dir.join("good.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        good.as_os_str(),
    ]));
    std::fs::write(&xml, b"keep").unwrap();
    let out = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        good.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert_eq!(std::fs::read(&xml).unwrap(), b"keep");

    // A second import into the populated catalog is refused (new catalog only).
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        good.as_os_str(),
    ]);
    assert!(!out.status.success());

    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    std::fs::remove_dir_all(dir).unwrap();
}

fn import_synthetic(dir: &Path) -> PathBuf {
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    std::fs::write(&pdb, synthetic_pdb(None)).unwrap();
    write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    catalog
}

fn export_with(catalog: &Path, xml: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ];
    args.extend(extra.iter().map(|a| s(a)));
    cli(&args)
}

/// `--drive X` qualifies only device-relative `file://localhost/Contents/` locations;
/// every other byte equals the no-option export.
#[test]
fn drive_option_qualifies_device_relative_locations_only() {
    let dir = scratch();
    let catalog = import_synthetic(&dir);
    let plain = dir.join("plain.xml");
    stdout(&export_with(&catalog, &plain, &[]));
    let plain_bytes = std::fs::read(&plain).unwrap();
    // Regression: without --drive the export is today's oracle exactly.
    assert_eq!(decode(&plain_bytes), expected());

    for (arg, letter) in [("D", "D"), ("d", "D"), ("z", "Z")] {
        let xml = dir.join(format!("drive-{arg}.xml"));
        let report = stdout(&export_with(&catalog, &xml, &["--drive", arg]));
        assert!(
            report.lines().any(|l| l == "PreviewVerified: true"),
            "{report}"
        );
        let bytes = std::fs::read(&xml).unwrap();

        // Byte comparison: only the Location prefix of device-relative tracks differs.
        let plain_text = String::from_utf8(plain_bytes.clone()).unwrap();
        let from = "Location=\"file://localhost/Contents/";
        assert_eq!(plain_text.matches(from).count(), 2);
        let want = plain_text.replace(
            from,
            &format!("Location=\"file://localhost/{letter}:/Contents/"),
        );
        assert_eq!(String::from_utf8(bytes.clone()).unwrap(), want);

        // Independent oracle: exactly these Location values, nothing else changes.
        let mut oracle = expected();
        let locations = [
            format!("file://localhost/{letter}:/Contents/Song%20A.mp3"),
            "file://localhost/b.wav".to_string(), // already absolute: unchanged
            format!("file://localhost/{letter}:/Contents/C.flac"),
        ];
        for (track, location) in oracle.tracks.iter_mut().zip(&locations) {
            track.iter_mut().find(|(k, _)| k == "Location").unwrap().1 = location.clone();
        }
        assert_eq!(decode(&bytes), oracle);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_drive_is_a_usage_error_without_output() {
    let dir = scratch();
    let catalog = import_synthetic(&dir);
    let xml = dir.join("never.xml");
    for bad in ["", "DD", "C:", "1", "-", "\u{e9}", "D:/"] {
        let out = export_with(&catalog, &xml, &["--drive", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--drive expects a single drive letter A-Z"),
            "{bad:?}: {stderr}"
        );
        assert!(!xml.exists(), "{bad:?} created output");
    }
    // Missing value or unknown trailing flag: usage error, no output.
    for extra in [&["--drive"][..], &["--letter", "D"][..]] {
        let out = export_with(&catalog, &xml, extra);
        assert_eq!(out.status.code(), Some(2), "{extra:?}");
        assert!(!xml.exists());
    }
    std::fs::remove_dir_all(dir).unwrap();
}
