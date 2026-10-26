//! Independent checks for synthetic Rekordbox XML A1 fixtures.
//!
//! This is test tooling only. It does not implement a product adapter or prove
//! acceptance by Rekordbox or OneLibrary.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq)]
struct Element {
    name: String,
    attributes: BTreeMap<String, String>,
    children: Vec<Element>,
}

impl Element {
    fn from_start(start: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Self, String> {
        let name = std::str::from_utf8(start.local_name().as_ref())
            .map_err(|error| error.to_string())?
            .to_owned();
        let mut attributes = BTreeMap::new();
        for attribute in start.attributes().with_checks(true) {
            let attribute = attribute.map_err(|error| error.to_string())?;
            let key = std::str::from_utf8(attribute.key.local_name().as_ref())
                .map_err(|error| error.to_string())?
                .to_owned();
            let value = attribute
                .decoded_and_normalized_value(XmlVersion::Explicit1_0, reader.decoder())
                .map_err(|error| error.to_string())?
                .into_owned();
            if attributes.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate XML attribute {key}"));
            }
        }
        Ok(Self {
            name,
            attributes,
            children: Vec::new(),
        })
    }

    fn attr(&self, name: &str) -> Result<&str, String> {
        self.attributes
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("{} missing attribute {name}", self.name))
    }

    fn children_named(&self, name: &str) -> Vec<&Element> {
        self.children
            .iter()
            .filter(|child| child.name == name)
            .collect()
    }
}

fn parse(source: &[u8]) -> Result<Element, String> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;

    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(start) => stack.push(Element::from_start(&start, &reader)?),
            Event::Empty(start) => {
                let child = Element::from_start(&start, &reader)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(child);
                } else if root.replace(child).is_some() {
                    return Err("multiple XML root elements".to_owned());
                }
            }
            Event::End(end) => {
                let element = stack.pop().ok_or("unexpected closing element")?;
                let open_name = element.name.as_bytes();
                let close_name = end.name();
                if close_name.as_ref() != open_name {
                    return Err(format!(
                        "closing element {} does not match open element {}",
                        String::from_utf8_lossy(end.name().as_ref()),
                        element.name
                    ));
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(element);
                } else if root.replace(element).is_some() {
                    return Err("multiple XML root elements".to_owned());
                }
            }
            Event::Text(text) if !text.is_empty() => {
                return Err("text content is outside this fixture subset".to_owned());
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::DocType(_) | Event::PI(_) => {}
            Event::GeneralRef(_) => {
                return Err("entity references are outside this fixture subset".to_owned());
            }
            Event::Text(_) | Event::CData(_) => {}
        }
    }

    if !stack.is_empty() {
        return Err("unclosed XML element".to_owned());
    }
    root.ok_or_else(|| "missing XML root element".to_owned())
}

fn root(source: &[u8]) -> Result<Element, String> {
    let root = parse(source)?;
    if root.name != "DJ_PLAYLISTS" || root.attr("Version")? != "1.0.0" {
        return Err("expected DJ_PLAYLISTS Version 1.0.0".to_owned());
    }
    Ok(root)
}

fn one_child<'a>(element: &'a Element, name: &str) -> Result<&'a Element, String> {
    let children = element.children_named(name);
    if children.len() != 1 {
        return Err(format!("expected one {name}, found {}", children.len()));
    }
    Ok(children[0])
}

fn validate_envelope(root: &Element, expected_tracks: usize) -> Result<Vec<&Element>, String> {
    let product = one_child(root, "PRODUCT")?;
    for field in ["Name", "Version", "Company"] {
        product.attr(field)?;
    }
    let collection = one_child(root, "COLLECTION")?;
    let tracks = collection.children_named("TRACK");
    if collection.attr("Entries")? != tracks.len().to_string() || tracks.len() != expected_tracks {
        return Err("collection Entries mismatch".to_owned());
    }
    let mut ids = HashSet::new();
    for track in &tracks {
        let id = track
            .attr("TrackID")?
            .parse::<i64>()
            .map_err(|error| error.to_string())?;
        if !ids.insert(id) {
            return Err(format!("duplicate TrackID {id}"));
        }
        if !track.attr("Location")?.starts_with("file://localhost/") {
            return Err(format!("TrackID {id} has unsupported Location URI"));
        }
    }
    let playlist_root = one_child(one_child(root, "PLAYLISTS")?, "NODE")?;
    let folder_count = playlist_root.children_named("NODE").len();
    if playlist_root.attr("Type")? != "0"
        || playlist_root.attr("Name")? != "ROOT"
        || playlist_root.attr("Count")? != folder_count.to_string()
    {
        return Err("playlist root folder envelope mismatch".to_owned());
    }
    Ok(tracks)
}

fn check_f02(source: &[u8]) -> Result<(Value, Value), String> {
    let root = root(source)?;
    let tracks = validate_envelope(&root, 4)?;
    let track_ids: Vec<i64> = tracks
        .iter()
        .map(|track| track.attr("TrackID")?.parse().map_err(|e| format!("{e}")))
        .collect::<Result<_, _>>()?;
    let locations: Vec<&str> = tracks
        .iter()
        .map(|track| track.attr("Location"))
        .collect::<Result<_, _>>()?;
    let track_names: Vec<&str> = tracks
        .iter()
        .map(|track| track.attr("Name"))
        .collect::<Result<_, _>>()?;
    if track_ids != [100, 200, 300, 400] {
        return Err("F-02 collection TrackID order mismatch".to_owned());
    }
    let playlist_root = one_child(one_child(&root, "PLAYLISTS")?, "NODE")?;
    if playlist_root.attr("Type")? != "0"
        || playlist_root.attr("Name")? != "ROOT"
        || playlist_root.attr("Count")? != "1"
    {
        return Err("F-02 root folder envelope mismatch".to_owned());
    }
    let folder = one_child(playlist_root, "NODE")?;
    if folder.attr("Type")? != "0"
        || folder.attr("Name")? != "Sets"
        || folder.attr("Count")? != "2"
        || folder.children.len() != 2
    {
        return Err("F-02 Sets folder order/count mismatch".to_owned());
    }
    let mut playlist_values = Vec::new();
    let mut membership_count = 0;
    for (playlist, expected_name, expected_keys) in [
        (&folder.children[0], "House", vec![100, 200, 300]),
        (&folder.children[1], "Techno", vec![200, 400]),
    ] {
        if playlist.attr("Type")? != "1"
            || playlist.attr("Name")? != expected_name
            || playlist.attr("Entries")? != expected_keys.len().to_string()
            || playlist.attr("KeyType")? != "0"
        {
            return Err(format!("F-02 {expected_name} playlist binding mismatch"));
        }
        let keys: Vec<i64> = playlist
            .children_named("TRACK")
            .iter()
            .map(|track| track.attr("Key")?.parse().map_err(|e| format!("{e}")))
            .collect::<Result<_, _>>()?;
        if keys != expected_keys {
            return Err(format!("F-02 {expected_name} membership order mismatch"));
        }
        if keys.iter().any(|key| !track_ids.contains(key)) {
            return Err(format!("F-02 {expected_name} has dangling track key"));
        }
        membership_count += keys.len();
        playlist_values.push(json!({"name": expected_name, "track_ids": keys}));
    }
    if membership_count != 5
        || !playlist_values.iter().any(|playlist| {
            playlist["track_ids"]
                .as_array()
                .is_some_and(|ids| ids.contains(&json!(200)))
        })
    {
        return Err("F-02 shared endpoint or membership count mismatch".to_owned());
    }
    let tree = json!([{"name":"Sets","children":playlist_values}]);
    let archive = json!({"track_ids":track_ids,"track_names":track_names,"locations":locations,"playlist_tree":tree,"membership_count":membership_count,"revision":0,"history":[],"report":[]});
    let visible = json!({"playlist_tree":tree,"report":[]});
    Ok((archive, visible))
}

fn tempo_values(track: &Element) -> Result<Vec<Value>, String> {
    track
        .children_named("TEMPO")
        .iter()
        .map(|tempo| {
            Ok(json!({
                "inizio": tempo.attr("Inizio")?,
                "bpm": tempo.attr("Bpm")?,
                "metro": tempo.attr("Metro")?,
                "battito": tempo.attr("Battito")?.parse::<i64>().map_err(|e| format!("{e}"))?
            }))
        })
        .collect()
}

fn check_f07(source: &[u8]) -> Result<(Value, Value), String> {
    let root = root(source)?;
    let track = validate_envelope(&root, 1)?[0];
    let source_tempo = tempo_values(track)?;
    if Value::Array(source_tempo.clone())
        != json!([
            {"inizio":"0","bpm":"120","metro":"4/4","battito":1},
            {"inizio":"10","bpm":"128","metro":"4/4","battito":1}
        ])
    {
        return Err("F-07 source tempo values/order mismatch".to_owned());
    }
    let marks = track.children_named("POSITION_MARK");
    if marks.len() != 1 {
        return Err("F-07 expected exactly one absolute mark".to_owned());
    }
    let mark = marks[0];
    if mark.attr("Name")? != "Intro"
        || mark.attr("Type")? != "0"
        || mark.attr("Start")? != "5"
        || mark.attr("Num")? != "0"
        || mark.attributes.contains_key("End")
    {
        return Err("F-07 absolute mark or absent End mismatch".to_owned());
    }
    let source_mark = json!({"name":"Intro","type":0,"start_seconds":"5","num":0,"end":"absent"});
    let mut effective_tempo = source_tempo.clone();
    effective_tempo[0]["bpm"] = json!("125");
    let visible = json!({"tempo":effective_tempo,"position_mark":source_mark,"report":[]});
    let archive = json!({"source_tempo_bpm":["120","128"],"source_mark":source_mark,"operation":{"operation_id":"e1","target":"tempo[0].Bpm","old":"120","new":"125","expected_result":"accepted"},"effective_tempo_bpm":["125","128"],"effective_mark":source_mark,"revision":1});
    Ok((archive, visible))
}

fn check_f10(source: &[u8]) -> Result<(Value, Value, Value), String> {
    let root = root(source)?;
    let track = validate_envelope(&root, 1)?[0];
    let tempo = tempo_values(track)?;
    if Value::Array(tempo.clone())
        != json!([
            {"inizio":"0","bpm":"120","metro":"4/4","battito":1},
            {"inizio":"8","bpm":"128","metro":"4/4","battito":1},
            {"inizio":"16","bpm":"132","metro":"7/8","battito":3}
        ])
    {
        return Err("F-10 ordered tempo/meter points mismatch".to_owned());
    }
    let result = json!({"tempo":tempo,"revision":0,"report":[]});
    let visible = json!({"tempo":result["tempo"],"report":[]});
    let plan = json!({
        "contract_version":"logical-v1",
        "case_id":"F-10",
        "target_format":"rekordbox-xml",
        "export_policy":"safe-v1",
        "decision_phase":"pre-write",
        "outcome":"ALLOW_EXPORT",
        "selected_units": result["tempo"].as_array().unwrap().iter().enumerate().map(|(index, tempo)| {
            json!({
                "unit_id":format!("tempo[{index}]"),
                "inizio_seconds":tempo["inizio"],
                "bpm":tempo["bpm"],
                "metro":tempo["metro"],
                "battito":tempo["battito"]
            })
        }).collect::<Vec<_>>(),
        "withheld_units":[],
        "report":[],
        "target_write":"permitted-but-not-performed-by-fixture-harness",
        "no_write_expected":false
    });
    Ok((result, visible, plan))
}

fn fixture(case_id: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rekordbox-a1")
        .join(case_id)
        .join(name)
}

fn read_json(case_id: &str, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(case_id, name)).unwrap()).unwrap()
}

#[test]
fn a1_fixture_bytes_match_frozen_provenance_and_goldens() {
    for (case_id, expected_hash) in [
        (
            "F-02",
            "4304e6df3de22a0f718ca3e23e3a577ae361d45b946c161f7e3557767587ee9f",
        ),
        (
            "F-07",
            "13becb20e52ed3aa4316fa69e95155624d4b53457dd4a1aa3e56beaf10baf5e7",
        ),
        (
            "F-10",
            "a0f7c6662eba04bae044033cfff867d04289b44b3bf637d2ee28dcdd223af459",
        ),
    ] {
        let source = std::fs::read(fixture(case_id, "source.bin")).unwrap();
        let digest = format!("{:x}", Sha256::digest(&source));
        assert_eq!(digest, expected_hash, "{case_id} source bytes changed");
        let provenance = read_json(case_id, "provenance.json");
        assert_eq!(provenance["source_sha256"], json!(digest));
        assert_eq!(provenance["source_length_bytes"], json!(source.len()));
        assert_eq!(
            provenance["specification_sha256"],
            "2acbe04491bfed8b3c53a977a63e372c1605dfc4d8f8047fe89f28f36151aeeb"
        );
        let case = read_json(case_id, "case.json");
        assert_eq!(case["case_id"], case_id);
        assert_eq!(case["source_kind"], "rekordbox-xml");
        assert_eq!(case["contract_version"], "logical-v1");
        assert_eq!(case["source_version"], "1.0.0");
        assert_eq!(case["target_format"], "rekordbox-xml");
        assert_eq!(case["export_policy"], "safe-v1");
    }

    let source = std::fs::read(fixture("F-02", "source.bin")).unwrap();
    let exported = oxidj_rekordbox::export_xml(&source).unwrap();
    let (archive, visible) = check_f02(&exported).unwrap();
    assert_eq!(archive, read_json("F-02", "expected-archive.json"));
    assert_eq!(visible, read_json("F-02", "expected-visible.json"));
    assert_eq!(read_json("F-02", "operations.json"), json!([]));
    assert_eq!(read_json("F-02", "expected-report.json"), json!([]));

    let source = std::fs::read(fixture("F-07", "source.bin")).unwrap();
    let exported = oxidj_rekordbox::export_xml(&source).unwrap();
    assert_eq!(exported, source);
    let exported_root = root(&exported).unwrap();
    let exported_track = validate_envelope(&exported_root, 1).unwrap()[0];
    assert_eq!(
        tempo_values(exported_track).unwrap(),
        tempo_values(
            root(&source).unwrap().children_named("COLLECTION")[0].children_named("TRACK")[0]
        )
        .unwrap()
    );
    let exported_mark = exported_track.children_named("POSITION_MARK")[0];
    assert_eq!(exported_mark.attr("Num").unwrap(), "0");
    assert!(!exported_mark.attributes.contains_key("End"));
    let (archive, visible) = check_f07(&exported).unwrap();
    assert_eq!(archive, read_json("F-07", "expected-archive.json"));
    assert_eq!(visible, read_json("F-07", "expected-visible.json"));
    assert_eq!(
        read_json("F-07", "operations.json"),
        json!([{"operation_id":"e1","target":"tempo[0].Bpm","expected_revision":0,"old":"120","new":"125","expected_result":"accepted"}])
    );
    assert_eq!(read_json("F-07", "expected-report.json"), json!([]));

    let source = std::fs::read(fixture("F-10", "source.bin")).unwrap();
    let exported = oxidj_rekordbox::export_xml(&source).unwrap();
    let (archive, visible, plan) = check_f10(&exported).unwrap();
    assert_eq!(archive, read_json("F-10", "expected-archive.json"));
    assert_eq!(visible, read_json("F-10", "expected-visible.json"));
    assert_eq!(plan, read_json("F-10", "expected-plan.json"));
    let provenance = read_json("F-10", "provenance.json");
    let plan_bytes = std::fs::read(fixture("F-10", "expected-plan.json")).unwrap();
    assert_eq!(provenance["expected_plan_path"], "expected-plan.json");
    assert_eq!(provenance["expected_plan_length_bytes"], plan_bytes.len());
    assert_eq!(
        provenance["expected_plan_sha256"],
        format!("{:x}", Sha256::digest(&plan_bytes))
    );
    assert_eq!(read_json("F-10", "operations.json"), json!([]));
    assert_eq!(read_json("F-10", "expected-report.json"), json!([]));
}

#[test]
fn parser_rejects_mismatched_closing_tag() {
    let source = std::fs::read(fixture("F-02", "source.bin")).unwrap();
    let malformed = String::from_utf8(source)
        .unwrap()
        .replacen("</COLLECTION>", "</WRONG>", 1);
    assert!(parse(malformed.as_bytes()).is_err());
}

#[test]
fn f02_checker_rejects_reordered_membership_and_dangling_endpoint() {
    let source = std::fs::read(fixture("F-02", "source.bin")).unwrap();
    let reordered = String::from_utf8(source.clone()).unwrap().replacen(
        "<TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>",
        "<TRACK Key=\"300\"/>\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>",
        1,
    );
    assert!(check_f02(reordered.as_bytes()).is_err());

    let dangling = String::from_utf8(source).unwrap().replace(
        "TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\"",
        "TrackID=\"201\" Name=\"B\" Location=\"file://localhost/music/b.flac\"",
    );
    assert!(check_f02(dangling.as_bytes()).is_err());
}

#[test]
fn f07_checker_rejects_changed_absolute_mark_and_optionality_or_units() {
    let source = std::fs::read(fixture("F-07", "source.bin")).unwrap();
    let text = String::from_utf8(source).unwrap();
    for changed in [
        text.replace("Start=\"5\"", "Start=\"5.5\""),
        text.replace("Start=\"5\"", "Start=\"5000\""),
        text.replace("Num=\"0\"", ""),
        text.replace("Num=\"0\"", "Num=\"0\" End=\"5\""),
    ] {
        assert!(check_f07(changed.as_bytes()).is_err());
    }
}

#[test]
fn f10_checker_rejects_changed_missing_or_reordered_tempo_points() {
    let source = std::fs::read(fixture("F-10", "source.bin")).unwrap();
    let text = String::from_utf8(source.clone()).unwrap();
    let changed = text.replace("Bpm=\"128\"", "Bpm=\"129\"");
    assert!(check_f10(changed.as_bytes()).is_err());

    let missing = text.replace(
        "      <TEMPO Inizio=\"8\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n",
        "",
    );
    assert!(check_f10(missing.as_bytes()).is_err());

    let reordered = text.replace(
        "      <TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"8\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>",
        "      <TEMPO Inizio=\"8\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"1\"/>",
    );
    assert!(check_f10(reordered.as_bytes()).is_err());
}
