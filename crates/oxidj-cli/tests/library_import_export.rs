//! M7 slice, synthetic-only: PDB → `catalog import-library` → new catalog →
//! `catalog export-library-xml` → new XML, checked by an independent quick-xml decoder
//! against values hard-coded from the fixture definition below.
use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use quick_xml::{Reader, events::Event};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[test]
fn export_refuses_unrelated_sqlite_without_modifying_it() {
    let dir = scratch();
    let catalog = dir.join("unrelated.sqlite");
    let script = "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('create table sentinel(value text)'); c.execute(\"insert into sentinel values ('keep')\"); c.execute('pragma user_version=7'); c.commit()";
    assert!(
        Command::new("python3")
            .args(["-c", script])
            .arg(&catalog)
            .status()
            .unwrap()
            .success()
    );
    let before = std::fs::read(&catalog).unwrap();
    let output = dir.join("out.xml");
    let result = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        output.as_os_str(),
    ]);
    assert_ne!(result.status.code(), Some(0));
    assert!(!output.exists());
    assert_eq!(std::fs::read(&catalog).unwrap(), before);
}

#[test]
fn step_inspection_is_explicit_and_leaves_catalog_sources_and_exports_unchanged() {
    let dir = scratch();
    let audio = dir.join("a.wav");
    let rate = 8000u32;
    let render = |variable: bool| {
        let mut samples = vec![0.0f32; 322400];
        let mut hits = Vec::new();
        for n in 0..40 {
            hits.push(0.3 + n as f64 * 0.5);
        }
        let period = if variable { 0.4 } else { 0.5 };
        for n in 0.. {
            let t = 20.3 + n as f64 * period;
            if t >= 40.3 {
                break;
            }
            hits.push(t);
        }
        for hit in hits {
            let start = (hit * f64::from(rate)).round() as usize;
            for i in 0..2000 {
                let Some(slot) = samples.get_mut(start + i) else {
                    break;
                };
                let t = i as f64 / f64::from(rate);
                *slot += (0.6 * (std::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp()
                    + 0.3 * (std::f64::consts::TAU * 1800.0 * t).sin() * (-t / 0.004).exp())
                    as f32;
            }
        }
        let pcm: Vec<_> = samples
            .iter()
            .flat_map(|x| ((*x * 30000.0) as i16).to_le_bytes())
            .collect();
        let mut wav = b"RIFF".to_vec();
        wav.extend((36 + pcm.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(rate.to_le_bytes());
        wav.extend((rate * 2).to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((pcm.len() as u32).to_le_bytes());
        wav.extend(pcm);
        wav
    };
    std::fs::write(&audio, render(false)).unwrap();
    let location = format!("file://localhost{}", audio.display());
    let pdb = dir.join("export.pdb");
    let source = synthetic_pdb_with(None, &location);
    std::fs::write(&pdb, &source).unwrap();
    let catalog = dir.join("catalog.sqlite");
    assert_eq!(
        cli(&[
            s("catalog"),
            s("import-library"),
            s("--pdb"),
            pdb.as_os_str(),
            s("--catalog"),
            catalog.as_os_str()
        ])
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        cli(&[
            s("catalog"),
            s("analyze-tempo"),
            s("--db"),
            catalog.as_os_str()
        ])
        .status
        .code(),
        Some(0)
    );
    let db = oxidj_library::Catalog::open(&catalog).unwrap();
    let snapshot = db.library_snapshot().unwrap();
    let stored = db
        .computed_beat_grid_for_library_location(&location)
        .unwrap();
    assert!(matches!(
        stored,
        oxidj_library::LibraryComputedGrid::Found(_)
    ));
    drop(db);
    let export = |suffix: &str, computed: bool| {
        let path = dir.join(format!("{suffix}.xml"));
        let mut args = vec![
            s("catalog"),
            s("export-library-xml"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--output"),
            path.as_os_str(),
        ];
        if computed {
            args.push(s("--computed-grids"));
        }
        let out = cli(&args);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        std::fs::read(path).unwrap()
    };
    let normal = export("before", false);
    let computed = export("before-computed", true);
    // Snapshot all persistent SQL values, including versioned computed rows,
    // cues/playlists/vendor payloads. The inspector never receives this path.
    let dump = || {
        let out=Command::new("python3").args(["-c","import sqlite3,sys; c=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); print(c.execute('pragma user_version').fetchone()); print('\\n'.join(c.iterdump()))"]).arg(&catalog).output().unwrap();
        assert!(out.status.success());
        out.stdout
    };
    let before = dump();
    let variable = dir.join("step.wav");
    std::fs::write(&variable, render(true)).unwrap();
    let bytes = std::fs::read(&variable).unwrap();
    let out = cli(&[s("inspect-step-tempo"), s("--file"), variable.as_os_str()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = stdout(&out);
    assert!(text.contains("Classification: Variable"), "{text}");
    assert_eq!(
        text.lines().filter(|l| l.starts_with("Segment\t")).count(),
        2
    );
    assert!(text.contains("Frames: 322400"));
    assert!(text.contains("SupportEndMs: 40300"));
    assert_eq!(std::fs::read(&variable).unwrap(), bytes);
    assert_eq!(std::fs::read(&audio).unwrap(), render(false));
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    assert_eq!(dump(), before);
    assert_eq!(export("after", false), normal);
    assert_eq!(export("after-computed", true), computed);
    let db = oxidj_library::Catalog::open(&catalog).unwrap();
    assert_eq!(db.library_snapshot().unwrap(), snapshot);
    assert_eq!(
        db.computed_beat_grid_for_library_location(&location)
            .unwrap(),
        stored
    );
    drop(db);
    for args in [
        vec![s("inspect-step-tempo")],
        vec![
            s("inspect-step-tempo"),
            s("--file"),
            variable.as_os_str(),
            s("--db"),
            catalog.as_os_str(),
        ],
    ] {
        assert_eq!(cli(&args).status.code(), Some(2));
    }
    assert_eq!(
        cli(&[
            s("inspect-step-tempo"),
            s("--file"),
            dir.join("absent").as_os_str()
        ])
        .status
        .code(),
        Some(1)
    );
    let constant_out = cli(&[s("inspect-step-tempo"), s("--file"), audio.as_os_str()]);
    assert_eq!(constant_out.status.code(), Some(0));
    assert!(stdout(&constant_out).contains("Classification: Constant"));
    let silent = dir.join("silent.wav");
    let mut wav = render(false);
    wav[44..].fill(0);
    std::fs::write(&silent, &wav).unwrap();
    let out = cli(&[s("inspect-step-tempo"), s("--file"), silent.as_os_str()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("NoMap: Silent"));
    let capped = dir.join("over-cap.wav");
    wav.resize(44 + (rate as usize * 180 + 1) * 2, 0);
    let len = wav.len() as u32;
    wav[4..8].copy_from_slice(&(len - 8).to_le_bytes());
    wav[40..44].copy_from_slice(&(len - 44).to_le_bytes());
    std::fs::write(&capped, &wav).unwrap();
    let out = cli(&[s("inspect-step-tempo"), s("--file"), capped.as_os_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("ResourceLimit"));
    assert!(out.stdout.is_empty());
    let unsupported = dir.join("unsupported-rate.wav");
    let mut unsupported_bytes = wav.clone();
    unsupported_bytes[24..28].copy_from_slice(&48001u32.to_le_bytes());
    unsupported_bytes[28..32].copy_from_slice(&(48001u32 * 2).to_le_bytes());
    std::fs::write(&unsupported, &unsupported_bytes).unwrap();
    let out = cli(&[
        s("inspect-step-tempo"),
        s("--file"),
        unsupported.as_os_str(),
    ]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(stdout(&out).contains("NoMap: UnsupportedRate"));
    assert!(!stdout(&out).contains("Segment\t"));
    assert!(out.stderr.is_empty());
    assert_eq!(std::fs::read(&unsupported).unwrap(), unsupported_bytes);
    assert_eq!(std::fs::read(&capped).unwrap(), wav);
    assert_eq!(std::fs::read(&silent).unwrap(), {
        let mut expected = render(false);
        expected[44..].fill(0);
        expected
    });
    let corrupt = dir.join("corrupt.wav");
    std::fs::write(&corrupt, &wav[..100]).unwrap();
    assert_eq!(
        cli(&[s("inspect-step-tempo"), s("--file"), corrupt.as_os_str()])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(dump(), before);
    std::fs::remove_dir_all(dir).unwrap();
}

fn scratch() -> PathBuf {
    // Parallel tests can read the same clock value; the counter keeps names unique.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Keep DeviceSQL's encoded file URL below its 127-byte string limit even
    // when TMPDIR is unusually long. Use a deterministic short path under the
    // repository-owned scratch root instead of depending on the process ID.
    let root = std::env::var_os("OXIDJ_TEST_SCRATCH_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/m6"));
    std::fs::create_dir_all(&root).unwrap();
    let dir = root.join(format!("{nonce:x}-{seq:x}"));
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

include!("support/library_pdb_fixture.rs");

// --- synthetic ANLZ builder (big-endian tagged sections per the public format
// reference; independent of the production decoder) ---

const ANLZ_A: &str = "/PIONEER/USBANLZ/P001/0000002A/ANLZ0000.DAT";
const ANLZ_B: &str = "/PIONEER/USBANLZ/P001/00000007/ANLZ0000.DAT";

include!("support/library_anlz_fixture.rs");

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
fn pdb_imported_library_audio_is_analyzed_and_grid_persisted() {
    let dir = scratch();
    let audio = dir.join("known.wav");
    let rate = 22_050u32;
    let seconds = 24.0;
    let bpm = 120.0;
    let mut samples = vec![0.0f64; (seconds * f64::from(rate)) as usize];
    for (semitone, amplitude) in [(261.63, 0.18), (329.63, 0.14), (392.0, 0.12)] {
        for (i, sample) in samples.iter_mut().enumerate() {
            *sample +=
                amplitude * (std::f64::consts::TAU * semitone * i as f64 / f64::from(rate)).sin();
        }
    }
    let mut beat = 0.2;
    while beat < seconds {
        let start = (beat * f64::from(rate)) as usize;
        for (i, sample) in samples
            .iter_mut()
            .skip(start)
            .take((0.08 * f64::from(rate)) as usize)
            .enumerate()
        {
            let t = i as f64 / f64::from(rate);
            *sample += 0.55 * (std::f64::consts::TAU * 1_800.0 * t).sin() * (-t / 0.008).exp();
        }
        beat += 60.0 / bpm;
    }
    let pcm: Vec<u8> = samples
        .iter()
        .flat_map(|x| ((*x * 30_000.0) as i16).to_le_bytes())
        .collect();
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + pcm.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * 2).to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((pcm.len() as u32).to_le_bytes());
    wav.extend(pcm);
    std::fs::write(&audio, wav).unwrap();

    // Source device and output catalog are deliberately separate namespaces.
    let pdb_dir = dir.join("device/PIONEER/rekordbox");
    std::fs::create_dir_all(&pdb_dir).unwrap();
    let pdb = pdb_dir.join("export.pdb");
    let location = format!("file://localhost{}", audio.display());
    std::fs::write(&pdb, synthetic_pdb_with(None, &location)).unwrap();
    let catalog = dir.join("catalog.sqlite");
    let import = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]);
    assert_eq!(
        import.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    // Exercise the CLI boundary, not just Catalog's error classification. The
    // trigger rejects the first readable imported audio reference; missing
    // locations must not turn this storage failure into a successful report.
    let sql = |statement: &str| {
        let result = Command::new("python3")
            .args([
                "-c",
                "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(sys.argv[2]); c.commit()",
            ])
            .arg(&catalog)
            .arg(statement)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
    };
    sql(
        "CREATE TRIGGER reject_reference BEFORE INSERT ON tracks BEGIN SELECT RAISE(ABORT, 'test reference storage failure'); END",
    );
    let rejected = cli(&[
        s("catalog"),
        s("analyze-tempo"),
        s("--db"),
        catalog.as_os_str(),
    ]);
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    let error = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        error.contains("catalog database error: test reference storage failure"),
        "{error}"
    );
    assert!(
        !String::from_utf8_lossy(&rejected.stdout).contains("UnavailableLibraryAudio:"),
        "{rejected:?}"
    );
    {
        let db = oxidj_library::Catalog::open(&catalog).unwrap();
        assert!(db.list_tracks().unwrap().is_empty());
        assert_eq!(db.library_snapshot().unwrap().tracks[2].location, location);
    }
    sql("DROP TRIGGER reject_reference");
    // Same imported catalog and audio now succeed; the two unavailable
    // locations retain their distinct, successful classification.
    let analyze = cli(&[
        s("catalog"),
        s("analyze-tempo"),
        s("--db"),
        catalog.as_os_str(),
    ]);
    let output = stdout(&analyze);
    assert_eq!(analyze.status.code(), Some(0), "{output}");
    assert!(output.contains("Analyzed: 1"), "{output}");
    assert!(output.contains("Key\t1\tMajor(0)"), "{output}");
    assert!(output.contains("UnavailableLibraryAudio: 2"), "{output}");
    let db = oxidj_library::Catalog::open(&catalog).unwrap();
    let imported = db.library_snapshot().unwrap();
    assert_eq!(imported.tracks[2].title, "Song C");
    assert_eq!(imported.tracks[2].location, location);
    let grid = db
        .computed_beat_grid_for_library_location(&location)
        .unwrap();
    let oxidj_library::LibraryComputedGrid::Found(grid) = grid else {
        panic!("computed grid absent: {grid:?}; output={output}");
    };
    assert!((grid.grid.bpm() - bpm).abs() < 0.1);
    assert!(!grid.grid.beats_ms().is_empty());
    assert!((i64::from(grid.grid.beats_ms()[0]) - 200).abs() <= 5);
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
fn catalog_playlist_creation_persists_order_and_preserves_the_exported_library() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    let baseline_xml = dir.join("baseline.xml");
    let export = |path: &Path| {
        cli(&[
            s("catalog"),
            s("export-library-xml"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--output"),
            path.as_os_str(),
        ])
    };
    stdout(&export(&baseline_xml));
    let mut baseline = decode(&std::fs::read(&baseline_xml).unwrap());
    assert_eq!(baseline, expected());
    let original = oxidj_library::Catalog::open(&catalog)
        .unwrap()
        .library_snapshot()
        .unwrap();
    for (name, ids, reason) in [
        ("New", "", "playlist membership must not be empty"),
        (
            "New",
            "1,nope,3",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "1,,3",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "1,",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "-1",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "+1",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "1, 2",
            "track IDs must be comma-separated positive integers",
        ),
        (
            "New",
            "0",
            "track IDs must be positive integers within range",
        ),
        (
            "New",
            "184467440737095516160",
            "track IDs must be positive integers within range",
        ),
        ("New", "3,1,4", "playlist member references a missing track"),
        ("  \t", "1", "playlist name must not be blank"),
        (
            "bad\u{1}",
            "1",
            "playlist name contains invalid XML 1.0 text",
        ),
        (
            "bad\u{7f}",
            "1",
            "playlist name contains invalid XML 1.0 text",
        ),
        (
            "bad\u{85}",
            "1",
            "playlist name contains invalid XML 1.0 text",
        ),
        (
            "bad\u{ffff}",
            "1",
            "playlist name contains invalid XML 1.0 text",
        ),
        ("Top", "1", "a root node with this name already exists"),
        ("Folder", "1", "a root node with this name already exists"),
    ] {
        let refused = cli(&[
            s("catalog"),
            s("create-library-playlist"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--name"),
            s(name),
            s("--track-ids"),
            s(ids),
        ]);
        assert!(!refused.status.success());
        assert!(
            refused.stdout.is_empty(),
            "must not acknowledge failed commit"
        );
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(stderr.contains(reason), "{stderr}");
        assert_eq!(
            oxidj_library::Catalog::open(&catalog)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            original
        );
    }
    // Refused names must not poison the next fresh-process export.
    let after_refusals = dir.join("after-refusals.xml");
    stdout(&export(&after_refusals));
    assert_eq!(decode(&std::fs::read(after_refusals).unwrap()), baseline);
    for tail in [
        vec![s("--unknown"), s("value")],
        vec![s("--name"), s("Again")],
        vec![s("--track-ids")],
    ] {
        let mut args = vec![
            s("catalog"),
            s("create-library-playlist"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--name"),
            s("New"),
            s("--track-ids"),
            s("3,1,3"),
        ];
        args.extend(tail);
        let refused = cli(&args);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&refused.stderr)
                .contains("expected catalog create-library-playlist")
        );
        assert_eq!(
            oxidj_library::Catalog::open(&catalog)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            original
        );
    }
    let created = cli(&[
        s("catalog"),
        s("create-library-playlist"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--name"),
        s("New & <ordered> \"mix\"\t\n\r Café 音 🎵"),
        s("--track-ids"),
        s("3,1,3"),
    ]);
    assert_eq!(stdout(&created), "CreatedPlaylists: 1\nMemberships: 3\n");
    assert!(created.stderr.is_empty());
    let committed = oxidj_library::Catalog::open(&catalog)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let repeated = cli(&[
        s("catalog"),
        s("create-library-playlist"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--name"),
        s("New & <ordered> \"mix\"\t\n\r Café 音 🎵"),
        s("--track-ids"),
        s("3,1,3"),
    ]);
    assert!(!repeated.status.success());
    assert!(repeated.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&repeated.stderr)
            .contains("a root node with this name already exists")
    );
    assert_eq!(
        oxidj_library::Catalog::open(&catalog)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        committed
    );
    // Each CLI invocation is a fresh process; export must observe committed data.
    let xml = dir.join("after.xml");
    stdout(&export(&xml));
    let Node::Folder(_, roots) = &mut baseline.root else {
        panic!("missing root")
    };
    roots.push(Node::Playlist(
        "New & <ordered> \"mix\"\t\n\r Café 音 🎵".into(),
        vec!["3".into(), "1".into(), "3".into()],
    ));
    let bytes = std::fs::read(&xml).unwrap();
    assert_eq!(decode(&bytes), baseline);
    let again = export(&xml);
    assert!(!again.status.success());
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("refusing to overwrite existing output")
    );
    assert_eq!(std::fs::read(&xml).unwrap(), bytes);
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_rename_preserves_complete_baseline() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    let before = dir.join("before.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        before.as_os_str(),
    ]));
    let mut expected = expected();
    assert_eq!(decode(&std::fs::read(before).unwrap()), expected);
    let rename = |old: &str, new: &str| {
        cli(&[
            s("catalog"),
            s("rename-library-playlist"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s(old),
            s("--new-name"),
            s(new),
        ])
    };
    for (old, new) in [("missing", "New"), ("Top", ""), ("Top", "bad\u{1}")] {
        assert!(!rename(old, new).status.success());
    }
    let result = rename("Top", "Renamed & Café");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let Node::Folder(_, roots) = &mut expected.root else {
        panic!("fixture root")
    };
    let Node::Playlist(name, _) = roots
        .iter_mut()
        .find(|n| matches!(n, Node::Playlist(name, _) if name == "Top"))
        .unwrap()
    else {
        panic!("playlist")
    };
    *name = "Renamed & Café".into();
    let after = dir.join("after.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        after.as_os_str(),
    ]));
    assert_eq!(decode(&std::fs::read(after).unwrap()), expected);
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_remove_one_occurrence_preserves_baseline() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    let before = dir.join("before.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        before.as_os_str(),
    ]));
    let mut expected = expected();
    assert_eq!(decode(&std::fs::read(before).unwrap()), expected);
    for (ids, index, id, reason) in [
        ("1,2,1", "0", "2", "playlist membership order is stale"),
        ("2,1,1", "3", "1", "playlist member index out of range"),
        ("2,1,1", "1", "2", "playlist member ID does not match"),
    ] {
        let refused = cli(&[
            s("catalog"),
            s("remove-library-playlist-member"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s("Top"),
            s("--expected-track-ids"),
            s(ids),
            s("--index"),
            s(index),
            s("--expected-track-id"),
            s(id),
        ]);
        assert_eq!(refused.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&refused.stderr).contains(reason));
        let check = dir.join(format!("refusal-{index}-{id}.xml"));
        stdout(&cli(&[
            s("catalog"),
            s("export-library-xml"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--output"),
            check.as_os_str(),
        ]));
        assert_eq!(decode(&std::fs::read(check).unwrap()), expected);
    }
    let result = cli(&[
        s("catalog"),
        s("remove-library-playlist-member"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--playlist-name"),
        s("Top"),
        s("--expected-track-ids"),
        s("2,1,1"),
        s("--index"),
        s("1"),
        s("--expected-track-id"),
        s("1"),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let Node::Folder(_, roots) = &mut expected.root else {
        panic!("fixture root")
    };
    let Node::Playlist(_, members) = roots
        .iter_mut()
        .find(|node| matches!(node, Node::Playlist(name, _) if name == "Top"))
        .unwrap()
    else {
        panic!("playlist")
    };
    *members = vec!["2".into(), "1".into()];
    let after = dir.join("after.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        after.as_os_str(),
    ]));
    assert_eq!(decode(&std::fs::read(after).unwrap()), expected);
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_reorder_preserves_everything_but_member_order() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    stdout(&cli(&[
        s("catalog"),
        s("create-library-playlist"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--name"),
        s("Reorder"),
        s("--track-ids"),
        s("3,1"),
    ]));
    stdout(&cli(&[
        s("catalog"),
        s("append-library-playlist-members"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--playlist-name"),
        s("Reorder"),
        s("--track-ids"),
        s("3"),
    ]));
    let before = dir.join("before.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        before.as_os_str(),
    ]));
    let mut expected = expected();
    let Node::Folder(_, roots) = &mut expected.root else {
        panic!("fixture root")
    };
    roots.push(Node::Playlist(
        "Reorder".into(),
        vec!["3".into(), "1".into(), "3".into()],
    ));
    assert_eq!(decode(&std::fs::read(before).unwrap()), expected);
    let result = cli(&[
        s("catalog"),
        s("reorder-library-playlist-members"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--playlist-name"),
        s("Reorder"),
        s("--track-ids"),
        s("3,3,1"),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "ReorderedMemberships: 3\n"
    );
    let Node::Folder(_, roots) = &mut expected.root else {
        panic!("fixture root")
    };
    let Node::Playlist(_, members) = roots.last_mut().unwrap() else {
        panic!("playlist")
    };
    *members = vec!["3".into(), "3".into(), "1".into()];
    let after = dir.join("after.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        after.as_os_str(),
    ]));
    assert_eq!(decode(&std::fs::read(after).unwrap()), expected);
    for (target, ids, reason) in [
        ("Reorder", "", "membership must not be empty"),
        ("Reorder", "3,,1", "comma-separated positive integers"),
        ("Reorder", "0", "positive integers within range"),
        ("Reorder", "-1", "comma-separated positive integers"),
        ("Reorder", "1.0", "comma-separated positive integers"),
        ("Reorder", " 1", "comma-separated positive integers"),
        ("Reorder", "1,", "comma-separated positive integers"),
        (
            "Reorder",
            "99999999999999999999999999999999999999",
            "positive integers within range",
        ),
        ("Reorder", "3,3,99", "membership counts differ"),
        ("Reorder", "3,1,1", "membership counts differ"),
        ("Reorder", "3,1", "membership counts differ"),
        ("Reorder", "3,3,1,1", "membership counts differ"),
        ("absent", "1", "root playlist does not exist"),
        ("Inner First", "1", "root playlist does not exist"),
        ("Folder", "1", "root name refers to a folder"),
    ] {
        let refused = cli(&[
            s("catalog"),
            s("reorder-library-playlist-members"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s(target),
            s("--track-ids"),
            s(ids),
        ]);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        assert!(String::from_utf8_lossy(&refused.stderr).contains(reason));
    }
    let unchanged = dir.join("unchanged.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        unchanged.as_os_str(),
    ]));
    assert_eq!(decode(&std::fs::read(unchanged).unwrap()), expected);
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_members_append_updates_only_the_exact_root_membership() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis(&device);
    let analysis_source = analysis.clone();
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    let baseline_path = dir.join("before.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        baseline_path.as_os_str(),
    ]));
    let baseline = decode(&std::fs::read(baseline_path).unwrap());
    let result = cli(&[
        s("catalog"),
        s("append-library-playlist-members"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--playlist-name"),
        s("Top"),
        s("--track-ids"),
        s("3,1,3"),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "AppendedMemberships: 3\n"
    );
    let reopened = oxidj_library::Catalog::open(&catalog)
        .unwrap()
        .library_snapshot()
        .unwrap();
    assert_eq!(reopened.tracks.len(), 3);
    let after_path = dir.join("after.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        after_path.as_os_str(),
    ]));
    let mut expected = baseline;
    let Node::Folder(_, roots) = &mut expected.root else {
        panic!("fixture root")
    };
    let Node::Playlist(_, members) = &mut roots[0] else {
        panic!("expected root playlist")
    };
    members.extend(["3".into(), "1".into(), "3".into()]);
    assert_eq!(decode(&std::fs::read(after_path).unwrap()), expected);
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis_source {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    for (target, ids) in [("Top", "1,,3"), ("Top", "3,99"), ("absent", "1")] {
        let refused = cli(&[
            s("catalog"),
            s("append-library-playlist-members"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s(target),
            s("--track-ids"),
            s(ids),
        ]);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
    }
    let invalid_flags = cli(&[
        s("catalog"),
        s("append-library-playlist-members"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--playlist-name"),
        s("Top"),
        s("--track-ids"),
        s("1"),
        s("--unexpected"),
        s("value"),
    ]);
    assert!(!invalid_flags.status.success());
    assert!(invalid_flags.stdout.is_empty());
    let unchanged_path = dir.join("after-refusals.xml");
    stdout(&cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        unchanged_path.as_os_str(),
    ]));
    assert_eq!(decode(&std::fs::read(unchanged_path).unwrap()), expected);
}

#[test]
fn catalog_playlist_append_refuses_non_imported_sqlite_without_mutation() {
    let dir = scratch();
    let catalog = dir.join("not-imported.sqlite");
    let sql = |statement: &str| {
        let result = Command::new("python3")
            .args([
                "-c",
                "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.commit(); print(c.execute('PRAGMA user_version').fetchone()[0]); print('\\n'.join(c.iterdump()))",
            ])
            .arg(&catalog)
            .arg(statement)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        result.stdout
    };
    // Both an unrelated database and one with a coincidentally current version
    // must fail before the mutation-capable opener can create OxiDJ tables.
    // Derive the current version from a real catalog instead of freezing a
    // migration number (artwork adds another schema migration).
    drop(oxidj_library::Catalog::open(&catalog).unwrap());
    let current = String::from_utf8(sql(""))
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    std::fs::remove_file(&catalog).unwrap();
    for version in [0, current] {
        let before = sql(&format!(
            "CREATE TABLE IF NOT EXISTS sentinel(value TEXT); DELETE FROM sentinel; \
             INSERT INTO sentinel VALUES ('synthetic sentinel'); PRAGMA user_version={version};"
        ));
        let refused = cli(&[
            s("catalog"),
            s("remove-library-playlist-member"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s("Top"),
            s("--expected-track-ids"),
            s("1"),
            s("--index"),
            s("0"),
            s("--expected-track-id"),
            s("1"),
        ]);
        assert_eq!(refused.status.code(), Some(1));
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(
            if version == 0 {
                error.contains("catalog schema version 0 is newer than supported")
            } else {
                error.contains("no such table: library_tracks")
            },
            "{error}"
        );
        assert_eq!(sql(""), before, "schema/version/data changed on refusal");
        let refused = cli(&[
            s("catalog"),
            s("rename-library-playlist"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s("Top"),
            s("--new-name"),
            s("New"),
        ]);
        assert_eq!(refused.status.code(), Some(1));
        assert!(refused.stdout.is_empty());
        assert_eq!(
            sql(""),
            before,
            "rename changed schema/version/data on refusal"
        );
        for action in [
            "append-library-playlist-members",
            "reorder-library-playlist-members",
        ] {
            let refused = cli(&[
                s("catalog"),
                s(action),
                s("--catalog"),
                catalog.as_os_str(),
                s("--playlist-name"),
                s("Top"),
                s("--track-ids"),
                s("1"),
            ]);
            assert_eq!(refused.status.code(), Some(1), "{refused:?}");
            assert!(refused.stdout.is_empty());
            let error = String::from_utf8_lossy(&refused.stderr);
            assert!(
                if version == 0 {
                    error.contains("catalog schema version 0 is newer than supported")
                } else {
                    error.contains("no such table: library_tracks")
                },
                "{error}"
            );
            assert_eq!(sql(""), before, "schema/version/data changed on refusal");
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_reorder_requires_an_existing_imported_catalog() {
    let dir = scratch();
    let catalog = dir.join("catalog.sqlite");
    let reorder = || {
        cli(&[
            s("catalog"),
            s("reorder-library-playlist-members"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--playlist-name"),
            s("Top"),
            s("--track-ids"),
            s("1"),
        ])
    };
    let missing = reorder();
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("catalog does not exist"));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    drop(oxidj_library::Catalog::open(&catalog).unwrap());
    let before = std::fs::read(&catalog).unwrap();
    let empty = reorder();
    assert_eq!(empty.status.code(), Some(1));
    assert!(empty.stdout.is_empty());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("catalog holds no imported tracks"));
    assert_eq!(std::fs::read(&catalog).unwrap(), before);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_playlist_creation_requires_an_existing_imported_catalog() {
    let dir = scratch();
    let catalog = dir.join("catalog.sqlite");
    let create = || {
        cli(&[
            s("catalog"),
            s("create-library-playlist"),
            s("--catalog"),
            catalog.as_os_str(),
            s("--name"),
            s("New"),
            s("--track-ids"),
            s("1"),
        ])
    };
    let missing = create();
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("catalog does not exist"));
    assert!(!catalog.exists());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    drop(oxidj_library::Catalog::open(&catalog).unwrap());
    let empty = create();
    assert!(!empty.status.success());
    assert!(empty.stdout.is_empty());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("catalog holds no imported tracks"));
    assert_eq!(
        oxidj_library::Catalog::open(&catalog)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        oxidj_library::LibrarySnapshot::default()
    );
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

// Independent decoder for the documented escaped TSV, not the production formatter.
fn listed_tracks(output: &Output) -> Vec<(String, String, Option<String>)> {
    fn field(raw: &str) -> String {
        let mut chars = raw.chars();
        let mut decoded = String::new();
        while let Some(c) = chars.next() {
            decoded.push(if c == '\\' {
                match chars.next().expect("unfinished escape") {
                    '\\' => '\\',
                    't' => '\t',
                    'n' => '\n',
                    'r' => '\r',
                    _ => panic!("unknown escape"),
                }
            } else {
                c
            });
        }
        decoded
    }
    assert!(output.stderr.is_empty());
    let text = stdout(output);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("ExportedTrackID\tTitle\tArtist"));
    lines
        .map(|line| {
            let parts: Vec<_> = line.split('\t').collect();
            assert_eq!(parts.len(), 3);
            (
                parts[0].into(),
                field(parts[1]),
                if parts[2] == "\\N" {
                    None
                } else {
                    Some(field(parts[2]))
                },
            )
        })
        .collect()
}

#[test]
fn listed_track_ids_match_export_and_create_an_ordered_playlist_without_listing_mutations() {
    let dir = scratch();
    let catalog = import_synthetic(&dir);
    let pdb = dir.join("device/PIONEER/rekordbox/export.pdb");
    let source = std::fs::read(&pdb).unwrap();
    let analysis: Vec<_> = [
        ANLZ_A.to_owned(),
        ANLZ_A.replace(".DAT", ".EXT"),
        ANLZ_B.to_owned(),
    ]
    .into_iter()
    .map(|rel| {
        let path = dir.join("device").join(rel.trim_start_matches('/'));
        let bytes = std::fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect();
    let baseline_xml = dir.join("before.xml");
    stdout(&export_with(&catalog, &baseline_xml, &[]));
    let baseline_bytes = std::fs::read(&baseline_xml).unwrap();
    let mut baseline = decode(&baseline_bytes);
    assert_eq!(baseline, expected());
    let original_bytes = std::fs::read(&catalog).unwrap();
    let original = oxidj_library::Catalog::open_read_only(&catalog)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let list = || {
        cli(&[
            s("catalog"),
            s("list-library-tracks"),
            s("--catalog"),
            catalog.as_os_str(),
        ])
    };
    let listed = listed_tracks(&list());
    let exported: Vec<_> = baseline
        .tracks
        .iter()
        .map(|attrs| {
            (
                get(attrs, "TrackID"),
                get(attrs, "Name"),
                attrs
                    .iter()
                    .find(|(k, _)| k == "Artist")
                    .map(|(_, v)| v.clone()),
            )
        })
        .collect();
    assert_eq!(listed, exported);
    assert_eq!(
        listed.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
    assert_eq!(listed_tracks(&list()), listed);
    assert_eq!(std::fs::read(&catalog).unwrap(), original_bytes);
    assert_eq!(
        oxidj_library::Catalog::open_read_only(&catalog)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        original
    );
    assert_eq!(std::fs::read(&baseline_xml).unwrap(), baseline_bytes);
    let ids = [
        listed[2].0.clone(),
        listed[0].0.clone(),
        listed[2].0.clone(),
    ];
    stdout(&cli(&[
        s("catalog"),
        s("create-library-playlist"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--name"),
        s("Listed set"),
        s("--track-ids"),
        s(&ids.join(",")),
    ]));
    let after = dir.join("after.xml");
    stdout(&export_with(&catalog, &after, &[]));
    let Node::Folder(_, children) = &mut baseline.root else {
        panic!("missing root")
    };
    children.push(Node::Playlist("Listed set".into(), ids.to_vec()));
    assert_eq!(decode(&std::fs::read(after).unwrap()), baseline);
    assert_eq!(listed_tracks(&list()), listed);
    assert_eq!(std::fs::read(pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert_eq!(std::fs::read(baseline_xml).unwrap(), baseline_bytes);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn list_tracks_refuses_missing_catalog_and_handles_empty_and_escaped_metadata() {
    let dir = scratch();
    let catalog = dir.join("catalog.sqlite");
    let list = || {
        cli(&[
            s("catalog"),
            s("list-library-tracks"),
            s("--catalog"),
            catalog.as_os_str(),
        ])
    };
    let missing = list();
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("catalog does not exist"));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    drop(oxidj_library::Catalog::open(&catalog).unwrap());
    let bytes = std::fs::read(&catalog).unwrap();
    assert_eq!(
        stdout(&list()),
        "ExportedTrackID\tTitle\tArtist\n(no imported tracks)\n"
    );
    assert_eq!(std::fs::read(&catalog).unwrap(), bytes);
    let title = "Café 音 🎵\tline\nreturn\r\"quoted\"\\N";
    let artists = [
        None,
        Some(String::new()),
        Some(title.into()),
        Some("\\N".into()),
    ];
    let snapshot = oxidj_library::LibrarySnapshot {
        tracks: artists
            .iter()
            .map(|artist| oxidj_library::LibraryTrack {
                title: title.into(),
                artist: artist.clone(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    oxidj_library::Catalog::open(&catalog)
        .unwrap()
        .import_library(&snapshot)
        .unwrap();
    let output = list();
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 5);
    assert_eq!(
        listed_tracks(&output),
        artists
            .into_iter()
            .enumerate()
            .map(|(i, artist)| ((i + 1).to_string(), title.into(), artist))
            .collect::<Vec<_>>()
    );
    std::fs::remove_dir_all(dir).unwrap();
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

// ---------------------------------------------------------------------------
// M7 MVP-first proof: a large synthetic library (1,330 tracks / 20 playlists /
// 901 memberships) exercised through the real import → manage → export path.
// ---------------------------------------------------------------------------

include!("support/library_m7_fixture.rs");

fn m7_python(pdb: &Path, xml: &Path, step: usize) -> Output {
    Command::new("python3")
        .arg("-B")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py"))
        .arg(pdb)
        .arg(xml)
        .args(["--step", &step.to_string()])
        .output()
        .expect("Python 3 is required for the independent M7 baseline oracle")
}

fn m7_cli(args: &[&std::ffi::OsStr]) -> Output {
    cli(args)
}

/// Whole-tree structural oracle over the independent `decode` output: collection
/// count, track count, playlist count, ordered membership count and that every
/// member key names a written track.
fn m7_verify_tree(bytes: &[u8], tracks: usize, playlists: usize, memberships: usize) -> Decoded {
    let decoded = decode(bytes);
    assert_eq!(
        decoded.collection_entries,
        tracks.to_string(),
        "COLLECTION Entries drift"
    );
    assert_eq!(decoded.tracks.len(), tracks, "collection track count drift");
    let mut written = std::collections::HashSet::new();
    for attrs in &decoded.tracks {
        written.insert(get(attrs, "TrackID"));
    }
    fn walk(
        node: &Node,
        playlists: &mut usize,
        memberships: &mut usize,
        written: &std::collections::HashSet<String>,
    ) {
        match node {
            Node::Folder(_, children) => {
                for child in children {
                    walk(child, playlists, memberships, written);
                }
            }
            Node::Playlist(_, keys) => {
                *playlists += 1;
                *memberships += keys.len();
                for key in keys {
                    assert!(written.contains(key), "member {key} names no written track");
                }
            }
        }
    }
    let (mut actual_playlists, mut actual_memberships) = (0, 0);
    walk(
        &decoded.root,
        &mut actual_playlists,
        &mut actual_memberships,
        &written,
    );
    assert_eq!(actual_playlists, playlists, "playlist count drift");
    assert_eq!(actual_memberships, memberships, "membership count drift");
    decoded
}

/// M7 MVP-first proof: import a 1,330-track / 20-playlist / 901-membership
/// synthetic Rekordbox library, manage it through the guarded XML operations,
/// re-export, and verify the whole tree with an independent decoder.
#[test]
fn m7_proof_import_manage_export_large_library() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    let source = synthetic_pdb_m7();
    std::fs::write(&pdb, &source).unwrap();
    let analysis = write_analysis_m7(&device);
    let catalog = dir.join("catalog.sqlite");

    // --- import ---
    let import = m7_cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]);
    let report = stdout(&import);
    for line in [
        "Tracks: 1330",
        "Playlists: 20",
        "Memberships: 901",
        "SourceUnchanged: true",
        "CatalogMatchesSource: true",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }
    for field in [
        "Artist",
        "Album",
        "Genre",
        "Label",
        "Tonality",
        "DateAdded",
        "Rating",
        "TEMPO",
        "POSITION_MARK",
    ] {
        let unresolved = report
            .lines()
            .find_map(|l| {
                l.strip_prefix(&format!("UnresolvedInSource\t{field}\t"))
                    .and_then(|count| count.parse::<u64>().ok())
            })
            .unwrap_or(0);
        assert_eq!(unresolved, 0, "unexpected unresolved {field}: {report}");
    }

    // --- baseline export ---
    let baseline = dir.join("baseline.xml");
    let export = m7_cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        baseline.as_os_str(),
    ]);
    let report = stdout(&export);
    for line in [
        "Tracks: 1330",
        "Playlists: 20",
        "Memberships: 901",
        "PreviewVerified: true",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }
    let baseline_bytes = std::fs::read(&baseline).unwrap();
    m7_verify_tree(&baseline_bytes, M7_TRACKS, M7_PLAYLISTS, M7_MEMBERSHIPS);
    stdout(&m7_python(&pdb, &baseline, 0));
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

    // --- manage: insert, append, clear, remove, rename, move to subfolder ---
    let order = |from: usize, to: usize| -> String {
        (from..=to)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    fn manage_step(current: &Path, next: &Path, action: &str, args: &[&str]) {
        let mut cmd = vec![
            s("rekordbox"),
            s(action),
            s("--xml"),
            current.as_os_str(),
            s("--output"),
            next.as_os_str(),
        ];
        cmd.extend(args.iter().map(|a| s(a)));
        let out = cli(&cmd);
        assert!(
            out.status.success(),
            "manage {action} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let mut current = baseline.clone();

    // 1. Insert TrackID 1330 at position 0 of Playlist 1 (index 0).
    let next = dir.join("managed-1.xml");
    manage_step(
        &current,
        &next,
        "insert-playlist-member",
        &[
            "--playlist-index",
            "0",
            "--expected-order",
            &order(1, 45),
            "--track-id",
            "1330",
            "--position",
            "0",
        ],
    );
    current = next;
    m7_verify_tree(
        &std::fs::read(&current).unwrap(),
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 1,
    );

    stdout(&m7_python(&pdb, &current, 1));

    // 2. Append TrackID 1330 to Playlist 2 (index 1).
    let next = dir.join("managed-2.xml");
    manage_step(
        &current,
        &next,
        "append-playlist-member",
        &[
            "--playlist-index",
            "1",
            "--expected-order",
            &order(46, 90),
            "--track-id",
            "1330",
        ],
    );
    current = next;
    m7_verify_tree(
        &std::fs::read(&current).unwrap(),
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 2,
    );

    stdout(&m7_python(&pdb, &current, 2));

    // 3. Clear Playlist 3 (index 2).
    let next = dir.join("managed-3.xml");
    manage_step(
        &current,
        &next,
        "clear-playlist",
        &["--playlist-index", "2", "--expected-order", &order(91, 135)],
    );
    current = next;
    m7_verify_tree(
        &std::fs::read(&current).unwrap(),
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 2 - 45,
    );

    stdout(&m7_python(&pdb, &current, 3));

    // 4. Remove TrackID 136 from Playlist 4 (index 3).
    let next = dir.join("managed-4.xml");
    manage_step(
        &current,
        &next,
        "remove-playlist-member",
        &[
            "--playlist-index",
            "3",
            "--expected-order",
            &order(136, 180),
            "--track-id",
            "136",
        ],
    );
    current = next;
    m7_verify_tree(
        &std::fs::read(&current).unwrap(),
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 2 - 45 - 1,
    );

    stdout(&m7_python(&pdb, &current, 4));

    // 5. Rename Playlist 5 (index 4).
    let next = dir.join("managed-5.xml");
    manage_step(
        &current,
        &next,
        "rename-playlist",
        &[
            "--playlist-index",
            "4",
            "--expected-name",
            "Playlist 5",
            "--name",
            "Renamed Playlist",
        ],
    );
    current = next;
    m7_verify_tree(
        &std::fs::read(&current).unwrap(),
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 2 - 45 - 1,
    );

    stdout(&m7_python(&pdb, &current, 5));

    // 6. Move Playlist 19 (index 18) from Folder A to Folder B.
    let next = dir.join("managed-6.xml");
    manage_step(
        &current,
        &next,
        "move-playlist",
        &[
            "--playlist-index",
            "18",
            "--expected-name",
            "Playlist 19",
            "--expected-members",
            &order(811, 856),
            "--source-parent-index",
            "0",
            "--source-parent-name",
            "Folder A",
            "--target-parent-index",
            "1",
            "--target-parent-name",
            "Folder B",
            "--expected-target-order",
            "Playlist 20",
        ],
    );
    current = next;
    let final_bytes = std::fs::read(&current).unwrap();
    m7_verify_tree(
        &final_bytes,
        M7_TRACKS,
        M7_PLAYLISTS,
        M7_MEMBERSHIPS + 2 - 45 - 1,
    );

    // Independent PDB decode → complete XML expectation after all six edits.
    stdout(&m7_python(&pdb, &current, 6));

    // Same-count, valid-ID corruption is refused for the precise reason.
    let final_text = String::from_utf8(final_bytes.clone()).unwrap();
    for (from, to, reason) in [
        ("Name=\"Track 1330\"", "Name=\"CORRUPTED\"", "tracks:"),
        ("Start=\"1.500\"", "Start=\"9.500\"", "children:"),
        (" Bpm=\"120.00\"", " Bpm=\"121.00\"", "children:"),
        ("Key=\"226\"", "Key=\"1330\"", "tree:"),
    ] {
        assert!(final_text.contains(from), "mutation target absent: {from}");
        let corrupt = dir.join("corrupt.xml");
        std::fs::write(&corrupt, final_text.replacen(from, to, 1)).unwrap();
        let rejected = m7_python(&pdb, &corrupt, 6);
        assert_eq!(rejected.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains(reason),
            "{from}: {}",
            String::from_utf8_lossy(&rejected.stderr)
        );
        assert_eq!(std::fs::read(&current).unwrap(), final_bytes);
    }

    // Swap only two unrelated members: same IDs, membership multiset and count.
    assert_eq!(final_text.matches("Key=\"226\"").count(), 1);
    assert_eq!(final_text.matches("Key=\"227\"").count(), 1);
    let reordered = final_text
        .replace("Key=\"226\"", "Key=\"SWAP\"")
        .replace("Key=\"227\"", "Key=\"226\"")
        .replace("Key=\"SWAP\"", "Key=\"227\"");
    let corrupt = dir.join("reordered.xml");
    std::fs::write(&corrupt, reordered).unwrap();
    let rejected = m7_python(&pdb, &corrupt, 6);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("tree:"));
    assert_eq!(std::fs::read(&current).unwrap(), final_bytes);

    // Sources untouched throughout.
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in &analysis {
        assert_eq!(&std::fs::read(path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
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

/// 30 s mono 16-bit WAV with a click train at `bpm` (first click at 0.3 s).
fn click_wav(bpm: f64) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let n = 30 * RATE as usize;
    let mut out = vec![0.0f64; n];
    let len = (0.25 * f64::from(RATE)) as usize;
    let mut hit = 0.3;
    while hit < 30.0 {
        let start = (hit * f64::from(RATE)).round() as usize;
        for (i, slot) in out.iter_mut().skip(start).take(len).enumerate() {
            let t = i as f64 / f64::from(RATE);
            *slot += 0.6 * (std::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp()
                + 0.3 * (std::f64::consts::TAU * 1_800.0 * t).sin() * (-t / 0.004).exp();
        }
        hit += 60.0 / bpm;
    }
    let data: Vec<u8> = out
        .iter()
        .flat_map(|s| ((s * 32_000.0) as i16).to_le_bytes())
        .collect();
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(data);
    wav
}

/// M7 slice 5 end to end: a click-track WAV under a local `Contents/` tree is
/// imported and analysed; `--computed-grids` writes exactly one constant TEMPO
/// (first computed beat, computed BPM, 4/4, beat 1) for the library track that
/// has no Rekordbox grid, fills its missing AverageBpm, keeps the vendor grid, and reports counts only. Without the flag the export is unchanged.
#[test]
fn computed_grids_option_writes_one_tempo_for_the_grid_less_track_only() {
    let dir = scratch();
    let device = dir.join("device");
    std::fs::create_dir_all(device.join("PIONEER/rekordbox")).unwrap();
    let pdb = device.join("PIONEER/rekordbox/export.pdb");
    std::fs::write(&pdb, synthetic_pdb_with(None, "/Contents/C.wav")).unwrap();
    write_analysis(&device);
    let local = dir.join("local");
    std::fs::create_dir_all(local.join("Contents")).unwrap();
    let wav = click_wav(128.0);
    std::fs::write(local.join("Contents/C.wav"), &wav).unwrap();
    let catalog = dir.join("catalog.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]));
    stdout(&cli(&[
        s("catalog"),
        s("import"),
        s("--dir"),
        local.as_os_str(),
        s("--db"),
        catalog.as_os_str(),
    ]));
    let analysed = stdout(&cli(&[
        s("catalog"),
        s("analyze-tempo"),
        s("--db"),
        catalog.as_os_str(),
    ]));
    assert!(analysed.contains("Analyzed: 1"), "{analysed}");

    // Oracle values straight from the stored computed grid.
    let stored = oxidj_library::Catalog::open(&catalog)
        .unwrap()
        .computed_beat_grid(oxidj_core::TrackId::new(1).unwrap())
        .unwrap()
        .expect("click track analysed");
    assert!(
        (stored.grid.bpm() - 128.0).abs() <= 0.5,
        "{}",
        stored.grid.bpm()
    );
    assert!(
        stored.grid.confidence() >= 0.5,
        "{}",
        stored.grid.confidence()
    );
    let first = stored.grid.beats_ms()[0];
    let start = format!("{}.{:03}", first / 1000, first % 1000);
    let bpm = format!("{:.2}", stored.grid.bpm());

    let mut oracle = expected();
    oracle.tracks[2]
        .iter_mut()
        .find(|(k, _)| k == "Location")
        .unwrap()
        .1 = "file://localhost/Contents/C.wav".into();
    let plain = dir.join("plain.xml");
    let plain_report = stdout(&export_with(&catalog, &plain, &[]));
    assert!(!plain_report.contains("ComputedGrid"), "{plain_report}");
    let plain_bytes = std::fs::read(&plain).unwrap();
    assert_eq!(decode(&plain_bytes), oracle, "no flag: unchanged export");

    let xml = dir.join("computed.xml");
    let report = stdout(&export_with(&catalog, &xml, &["--computed-grids"]));
    assert_aggregate_only(&report);
    for line in [
        "ComputedGridWritten: 1",
        // Song B's grid is unresolved in the source (empty) and its absolute
        // location has no local file: refused as no match.
        "VendorGridKept: 1",
        "SkippedLowConfidence: 0",
        "SkippedNotAnalyzed: 0",
        "SkippedNoMatch: 1",
        "SkippedAmbiguous: 0",
        "PreviewVerified: true",
    ] {
        assert!(report.lines().any(|l| l == line), "{line}: {report}");
    }
    oracle.tracks[2]
        .iter_mut()
        .find(|(k, _)| k == "AverageBpm")
        .unwrap()
        .1 = bpm.clone();
    oracle.children[2] = vec![tempo(&start, &bpm, "1")];
    assert_eq!(decode(&std::fs::read(&xml).unwrap()), oracle);

    // Combined with --drive (either order): only Location prefixes differ.
    let drive = dir.join("drive.xml");
    stdout(&export_with(
        &catalog,
        &drive,
        &["--computed-grids", "--drive", "D"],
    ));
    let want = String::from_utf8(std::fs::read(&xml).unwrap())
        .unwrap()
        .replace(
            "file://localhost/Contents/",
            "file://localhost/D:/Contents/",
        );
    assert_eq!(
        String::from_utf8(std::fs::read(&drive).unwrap()).unwrap(),
        want
    );
    // Repeating the flag is a usage error and writes nothing.
    let twice = dir.join("twice.xml");
    let out = export_with(&catalog, &twice, &["--computed-grids", "--computed-grids"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!twice.exists());
    // Sources are only read.
    assert_eq!(std::fs::read(local.join("Contents/C.wav")).unwrap(), wav);
    std::fs::remove_dir_all(dir).unwrap();
}
