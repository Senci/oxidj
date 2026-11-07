//! OxiDJ CLI entry point.
//!
//! The CLI calls shared core and library handlers; persistence stays behind
//! `oxidj-library`, and audio, UI, and vendor adapters stay out of this crate.
//!
//! This binary is deliberately thin. Each user-facing verb maps to a domain
//! operation in `oxidj-core` or `oxidj-library`; orchestration lives there.

use oxidj_application::TrackAttribute;
use oxidj_core::TrackId;
use oxidj_library::Catalog;
#[cfg(test)]
use quick_xml as _;

/// Exit code for usage / argument-shape errors.
const USAGE: i32 = 2;
/// Exit code for input that is well-formed but violates a domain rule.
const DOMAIN: i32 = 1;
/// Success.
const OK: i32 = 0;

/// CLI dispatch, factored out of `main` so it is unit-testable without spawning
/// a process. `args[0]` is the program name. Returns a process exit code.
fn run(args: &[String]) -> i32 {
    let Some(command) = args.get(1).map(String::as_str) else {
        print_usage(false);
        return USAGE;
    };
    match command {
        "--help" | "-h" => {
            print_usage(true);
            OK
        }
        "--version" | "-V" => {
            println!("oxidj-cli {}", env!("CARGO_PKG_VERSION"));
            OK
        }
        "track-id" => match args.get(2) {
            Some(raw) => match raw.parse::<u64>() {
                Ok(n) => match TrackId::new(n) {
                    Ok(id) => {
                        println!("{id}");
                        OK
                    }
                    Err(oxidj_core::TrackIdError::Zero) => {
                        eprintln!("error: track id must be >= 1 (0 is reserved)");
                        DOMAIN
                    }
                    Err(oxidj_core::TrackIdError::Parse) => {
                        eprintln!("error: argument must be an unsigned integer");
                        USAGE
                    }
                },
                Err(_) => {
                    eprintln!("error: argument must be an unsigned integer");
                    USAGE
                }
            },
            None => {
                eprintln!("error: track-id requires a numeric argument");
                USAGE
            }
        },
        "catalog" => run_catalog(&args[2..]),
        "onelibrary" => run_onelibrary(&args[2..]),
        "rekordbox" => run_rekordbox(&args[2..]),
        unknown => {
            eprintln!("error: unknown command: {unknown}");
            USAGE
        }
    }
}

fn run_onelibrary(args: &[String]) -> i32 {
    if args.len() != 4 || args[0] != "preview" || args[1] != "--db" || args[3] != "--key-stdin" {
        eprintln!(
            "error: expected onelibrary preview --db PATH --key-stdin (provide key bytes on non-terminal stdin)"
        );
        return USAGE;
    }
    match oxidj_application::preview_onelibrary_stdin(&args[2]) {
        Ok(preview) => {
            println!(
                "Profile: {}",
                if preview.schema_profile == oxidj_application::ONELIBRARY_SCHEMA_PROFILE_NATIVE {
                    "native OneLibrary (provisional structural match; no application-release claim)"
                } else {
                    "synthetic v1"
                }
            );
            println!(
                "Supported subset: title, artist name, album name, playlist hierarchy/order and ordered memberships. Excluded: BPM, length, path, cues/ANLZ, history, tags, color, artwork, and analysis."
            );
            println!("Tracks:");
            for track in preview.tracks {
                println!(
                    "Track\t{}\tTitle={:?}\tArtist={:?}\tAlbum={:?}",
                    track.id.as_str(),
                    track.title,
                    track.artist,
                    track.album
                );
            }
            println!("Playlists:");
            for playlist in preview.playlists {
                println!(
                    "Playlist\t{}\tParent={:?}\tOrder={}\tName={:?}\tTracks={:?}",
                    playlist.id.as_str(),
                    playlist.parent_id.as_ref().map(|id| id.as_str()),
                    playlist.sibling_order,
                    playlist.name,
                    playlist
                        .track_ids
                        .iter()
                        .map(|id| id.as_str())
                        .collect::<Vec<_>>()
                );
            }
            OK
        }
        Err(_) => {
            eprintln!("error: OneLibrary preview rejected input");
            DOMAIN
        }
    }
}

/// `rekordbox edit-track-<action>` rows: action, value flag, attribute, success prefix,
/// rejection label.
const TRACK_ATTRIBUTE_EDITS: &[(&str, &str, TrackAttribute, &str, &str)] = &[
    ("name", "name", TrackAttribute::Name, "Renamed ", "name"),
    (
        "album",
        "album",
        TrackAttribute::Album,
        "Changed Album for ",
        "Album",
    ),
    (
        "comments",
        "comments",
        TrackAttribute::Comments,
        "Changed Comments for ",
        "Comments",
    ),
    (
        "genre",
        "genre",
        TrackAttribute::Genre,
        "Changed Genre for ",
        "Genre",
    ),
    (
        "remixer",
        "remixer",
        TrackAttribute::Remixer,
        "Changed Remixer for ",
        "Remixer",
    ),
    ("mix", "mix", TrackAttribute::Mix, "Changed Mix for ", "Mix"),
    (
        "grouping",
        "grouping",
        TrackAttribute::Grouping,
        "Changed Grouping for ",
        "Grouping",
    ),
    (
        "rating",
        "rating",
        TrackAttribute::Rating,
        "Changed raw Rating for ",
        "Rating",
    ),
    (
        "colour",
        "colour",
        TrackAttribute::Colour,
        "Changed raw Colour for ",
        "Colour",
    ),
    (
        "composer",
        "composer",
        TrackAttribute::Composer,
        "Changed Composer for ",
        "Composer",
    ),
    (
        "tonality",
        "tonality",
        TrackAttribute::Tonality,
        "Changed Tonality for ",
        "Tonality",
    ),
    (
        "label",
        "label",
        TrackAttribute::Label,
        "Changed Label for ",
        "Label",
    ),
    (
        "artist",
        "artist",
        TrackAttribute::Artist,
        "Changed Artist for ",
        "Artist",
    ),
    (
        "date-added",
        "date-added",
        TrackAttribute::DateAdded,
        "Changed raw DateAdded for ",
        "DateAdded",
    ),
    (
        "date-modified",
        "date-modified",
        TrackAttribute::DateModified,
        "Changed raw DateModified for ",
        "DateModified",
    ),
    (
        "play-count",
        "play-count",
        TrackAttribute::PlayCount,
        "Changed raw PlayCount for ",
        "PlayCount",
    ),
    (
        "average-bpm",
        "average-bpm",
        TrackAttribute::AverageBpm,
        "Changed raw AverageBpm for ",
        "AverageBpm",
    ),
    (
        "disc-number",
        "disc-number",
        TrackAttribute::DiscNumber,
        "Changed raw DiscNumber for ",
        "DiscNumber",
    ),
    (
        "number",
        "track-number",
        TrackAttribute::TrackNumber,
        "Changed raw TrackNumber for ",
        "TrackNumber",
    ),
    (
        "size",
        "size",
        TrackAttribute::Size,
        "Changed raw Size for ",
        "Size",
    ),
    (
        "total-time",
        "total-time",
        TrackAttribute::TotalTime,
        "Changed raw TotalTime for ",
        "TotalTime",
    ),
    (
        "bit-rate",
        "bit-rate",
        TrackAttribute::BitRate,
        "Changed raw BitRate for ",
        "BitRate",
    ),
    (
        "sample-rate",
        "sample-rate",
        TrackAttribute::SampleRate,
        "Changed raw SampleRate for ",
        "SampleRate",
    ),
    (
        "last-played",
        "last-played",
        TrackAttribute::LastPlayed,
        "Changed raw LastPlayed for ",
        "LastPlayed",
    ),
    (
        "year",
        "year",
        TrackAttribute::Year,
        "Changed Year for ",
        "Year",
    ),
    (
        "kind",
        "kind",
        TrackAttribute::Kind,
        "Changed Kind for ",
        "Kind",
    ),
];

fn run_rekordbox(args: &[String]) -> i32 {
    if args.first().is_some_and(|action| action == "pdb-preview") {
        if args.len() != 3 || args[1] != "--pdb" {
            eprintln!("error: expected rekordbox pdb-preview --pdb PATH");
            return USAGE;
        }
        return match oxidj_application::preview_rekordbox_pdb(&args[2]) {
            Ok(preview) => {
                println!("Tracks:");
                for track in preview.tracks {
                    println!(
                        "Track\t{}\t{}\t{:?}\t{:?}\t{}\t{}\t{:?}",
                        track.source_id.0,
                        track.title,
                        track.artist,
                        track.album,
                        track.tempo_hundredths_bpm,
                        track.duration_seconds,
                        track.file_path
                    );
                }
                println!("Playlists:");
                for playlist in preview.playlists {
                    println!(
                        "Playlist\t{}\tParent={}\tPath={:?}\tTracks={:?}",
                        playlist.source_id.0,
                        playlist.parent_id.0,
                        playlist.hierarchy,
                        playlist
                            .ordered_track_ids
                            .iter()
                            .map(|id| id.0)
                            .collect::<Vec<_>>()
                    );
                }
                OK
            }
            Err(error) => {
                eprintln!("error: PDB preview rejected input: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "create-folder") {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--parent-index"
            || args[7] != "--expected-name"
            || args[9] != "--name"
        {
            eprintln!(
                "error: expected rekordbox create-folder --xml IN --output NEW --parent-index N --expected-name NAME --name NAME"
            );
            return USAGE;
        }
        let Ok(parent_index) = args[6].parse::<usize>() else {
            eprintln!("error: parent index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::FolderCreateRequest {
            parent_index,
            expected_parent_name: args[8].clone(),
            name: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_folder_create(&args[2], &args[4], &request)
        {
            Ok(()) => {
                println!(
                    "Created folder under folder[{}]: {:?}",
                    parent_index, args[10]
                );
                OK
            }
            Err(error) => {
                eprintln!("error: folder creation rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "create-playlist")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--parent-index"
            || args[7] != "--expected-name"
            || args[9] != "--name"
        {
            eprintln!(
                "error: expected rekordbox create-playlist --xml IN --output NEW --parent-index N --expected-name NAME --name NAME"
            );
            return USAGE;
        }
        let Ok(parent_index) = args[6].parse::<usize>() else {
            eprintln!("error: parent index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PlaylistCreateRequest {
            parent_index,
            expected_parent_name: args[8].clone(),
            name: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_playlist_create(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!(
                    "Created playlist under folder[{}]: {:?}",
                    parent_index, args[10]
                );
                OK
            }
            Err(error) => {
                eprintln!("error: playlist creation rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "delete-empty-folder")
    {
        if args.len() != 9
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--folder-index"
            || args[7] != "--expected-name"
        {
            eprintln!(
                "error: expected rekordbox delete-empty-folder --xml IN --output NEW --folder-index N --expected-name NAME"
            );
            return USAGE;
        }
        let Ok(folder_index) = args[6].parse::<usize>() else {
            eprintln!("error: folder index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::FolderDeleteRequest {
            folder_index,
            expected_name: args[8].clone(),
        };
        return match oxidj_application::export_rekordbox_folder_delete(&args[2], &args[4], &request)
        {
            Ok(()) => {
                println!("Deleted empty folder[{}]: {:?}", folder_index, args[8]);
                OK
            }
            Err(error) => {
                eprintln!("error: empty folder deletion rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "delete-playlist")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-name"
            || args[9] != "--expected-order"
        {
            eprintln!(
                "error: expected rekordbox delete-playlist --xml IN --output NEW --playlist-index N --expected-name NAME --expected-order ID,ID"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: malformed playlist index");
            return USAGE;
        };
        let request = oxidj_application::PlaylistDeleteRequest {
            playlist_index,
            expected_name: args[8].clone(),
            expected_order: if args[10].is_empty() {
                vec![]
            } else {
                args[10].split(',').map(str::to_owned).collect()
            },
        };
        return match oxidj_application::export_rekordbox_playlist_delete(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Deleted playlist {}", args[8]);
                OK
            }
            Err(error) => {
                eprintln!("error: playlist deletion rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "insert-playlist-member")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-order"
            || args[9] != "--track-id"
            || args[11] != "--position"
        {
            eprintln!(
                "error: expected rekordbox insert-playlist-member --xml IN --output NEW --playlist-index N --expected-order ID,ID --track-id ID --position N"
            );
            return USAGE;
        }
        let (Ok(playlist_index), Ok(position)) =
            (args[6].parse::<usize>(), args[12].parse::<usize>())
        else {
            eprintln!("error: malformed playlist index or insertion position");
            return USAGE;
        };
        let request = oxidj_application::PlaylistMemberInsertRequest {
            playlist_index,
            expected_order: args[8].split(',').map(str::to_owned).collect(),
            track_id: args[10].clone(),
            position,
        };
        return match oxidj_application::export_rekordbox_playlist_member_insert(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Inserted playlist reference {} at {}", args[10], position);
                OK
            }
            Err(error) => {
                eprintln!("error: playlist member insertion rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "clear-playlist")
    {
        if args.len() != 9
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-order"
        {
            eprintln!(
                "error: expected rekordbox clear-playlist --xml IN --output NEW --playlist-index N --expected-order ID,ID"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: malformed playlist index");
            return USAGE;
        };
        let request = oxidj_application::PlaylistClearRequest {
            playlist_index,
            expected_order: if args[8].is_empty() {
                vec![]
            } else {
                args[8].split(',').map(str::to_owned).collect()
            },
        };
        return match oxidj_application::export_rekordbox_playlist_clear(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Cleared playlist memberships");
                OK
            }
            Err(error) => {
                eprintln!("error: playlist clear rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "append-playlist-member")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-order"
            || args[9] != "--track-id"
        {
            eprintln!(
                "error: expected rekordbox append-playlist-member --xml IN --output NEW --playlist-index N --expected-order ID,ID --track-id ID"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: malformed playlist index");
            return USAGE;
        };
        let request = oxidj_application::PlaylistMemberAppendRequest {
            playlist_index,
            expected_order: if args[8].is_empty() {
                vec![]
            } else {
                args[8].split(',').map(str::to_owned).collect()
            },
            track_id: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_playlist_member_append(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Appended playlist reference {}", args[10]);
                OK
            }
            Err(error) => {
                eprintln!("error: playlist member append rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "remove-playlist-member")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-order"
            || args[9] != "--track-id"
        {
            eprintln!(
                "error: expected rekordbox remove-playlist-member --xml IN --output NEW --playlist-index N --expected-order ID,ID --track-id ID"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: malformed playlist index");
            return USAGE;
        };
        let request = oxidj_application::PlaylistMemberDeleteRequest {
            playlist_index,
            expected_order: args[8].split(',').map(str::to_owned).collect(),
            track_id: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_playlist_member_delete(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Removed playlist reference {}", args[10]);
                OK
            }
            Err(error) => {
                eprintln!("error: playlist member removal rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "delete-track") {
        if args.len() != 9
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--expected-memberships"
        {
            eprintln!(
                "error: expected rekordbox delete-track --xml IN --output NEW --track-id ID --expected-memberships INDEX:ID,ID;INDEX:ID,ID"
            );
            return USAGE;
        }
        let mut expected_memberships = Vec::new();
        for part in args[8].split(';') {
            let Some((index, ids)) = part.split_once(':') else {
                eprintln!("error: malformed membership guard");
                return USAGE;
            };
            let Ok(index) = index.parse::<usize>() else {
                eprintln!("error: malformed playlist index in membership guard");
                return USAGE;
            };
            if ids.is_empty() {
                eprintln!("error: empty playlist membership guard");
                return USAGE;
            }
            expected_memberships.push((index, ids.split(',').map(str::to_owned).collect()));
        }
        let request = oxidj_application::TrackDeleteRequest {
            track_id: args[6].clone(),
            expected_memberships,
        };
        return match oxidj_application::export_rekordbox_track_delete(&args[2], &args[4], &request)
        {
            Ok(()) => {
                println!(
                    "Deleted collection track {} and guarded playlist memberships",
                    args[6]
                );
                OK
            }
            Err(error) => {
                eprintln!("error: track deletion rejected: {error}");
                DOMAIN
            }
        };
    }
    if let Some((_, flag, attribute, changed, label)) = args
        .first()
        .and_then(|action| action.strip_prefix("edit-track-"))
        .and_then(|flag| TRACK_ATTRIBUTE_EDITS.iter().find(|row| row.0 == flag))
    {
        let expected_flag = format!("--expected-{flag}");
        let new_flag = format!("--{flag}");
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != expected_flag
            || args[9] != new_flag
        {
            eprintln!(
                "error: expected rekordbox {} --xml IN --output NEW --track-id ID {expected_flag} OLD {new_flag} NEW",
                args[0]
            );
            return USAGE;
        }
        let request = oxidj_application::TrackAttributeEditRequest {
            track_id: args[6].clone(),
            attribute: *attribute,
            expected_old: args[8].clone(),
            new: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_track_attribute_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "{changed}track[{}]: {:?} to {:?}",
                    change.track_id, change.old, change.new
                );
                OK
            }
            Err(error) => {
                eprintln!("error: track {label} edit rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "rename-folder") {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--folder-index"
            || args[7] != "--expected-name"
            || args[9] != "--name"
        {
            eprintln!(
                "error: expected rekordbox rename-folder --xml IN --output NEW --folder-index N --expected-name OLD --name NEW"
            );
            return USAGE;
        }
        let Ok(folder_index) = args[6].parse::<usize>() else {
            eprintln!("error: folder index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::FolderRenameRequest {
            folder_index,
            expected_old_name: args[8].clone(),
            new_name: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_folder_rename(&args[2], &args[4], &request)
        {
            Ok(change) => {
                println!(
                    "Renamed folder[{}]: {:?} to {:?}",
                    change.folder_index, change.old_name, change.new_name
                );
                OK
            }
            Err(error) => {
                eprintln!("error: folder rename rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "reorder-playlist")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-order"
            || args[9] != "--order"
        {
            eprintln!(
                "error: expected rekordbox reorder-playlist --xml IN --output NEW --playlist-index N --expected-order ID,ID --order ID,ID"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: playlist index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PlaylistReorderRequest {
            playlist_index,
            expected_order: args[8].split(',').map(str::to_owned).collect(),
            new_order: args[10].split(',').map(str::to_owned).collect(),
        };
        return match oxidj_application::export_rekordbox_playlist_reorder(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Reordered playlist[{}]", playlist_index);
                OK
            }
            Err(error) => {
                eprintln!("error: playlist reorder rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "reorder-sibling-playlists")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--parent-index"
            || args[7] != "--expected-name"
            || args[9] != "--expected-order"
            || args[11] != "--order"
        {
            eprintln!(
                "error: expected rekordbox reorder-sibling-playlists --xml IN --output NEW --parent-index N --expected-name NAME --expected-order NAME,NAME --order NAME,NAME"
            );
            return USAGE;
        }
        let Ok(parent_index) = args[6].parse::<usize>() else {
            eprintln!("error: parent index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::SiblingPlaylistReorderRequest {
            parent_index,
            expected_parent_name: args[8].clone(),
            expected_order: args[10].split(',').map(str::to_owned).collect(),
            new_order: args[12].split(',').map(str::to_owned).collect(),
        };
        return match oxidj_application::export_rekordbox_sibling_playlist_reorder(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Reordered direct playlists under folder[{}]", parent_index);
                OK
            }
            Err(error) => {
                eprintln!("error: sibling playlist reorder rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "reorder-sibling-folders")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--parent-index"
            || args[7] != "--expected-name"
            || args[9] != "--expected-order"
            || args[11] != "--order"
        {
            eprintln!(
                "error: expected rekordbox reorder-sibling-folders --xml IN --output NEW --parent-index N --expected-name NAME --expected-order NAME,NAME --order NAME,NAME"
            );
            return USAGE;
        }
        let Ok(parent_index) = args[6].parse::<usize>() else {
            eprintln!("error: parent index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::SiblingFolderReorderRequest {
            parent_index,
            expected_parent_name: args[8].clone(),
            expected_order: args[10].split(',').map(str::to_owned).collect(),
            new_order: args[12].split(',').map(str::to_owned).collect(),
        };
        return match oxidj_application::export_rekordbox_sibling_folder_reorder(
            &args[2], &args[4], &request,
        ) {
            Ok(()) => {
                println!("Reordered direct folders under folder[{}]", parent_index);
                OK
            }
            Err(error) => {
                eprintln!("error: sibling folder reorder rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "move-playlist") {
        if args.len() != 21
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-name"
            || args[9] != "--expected-members"
            || args[11] != "--source-parent-index"
            || args[13] != "--source-parent-name"
            || args[15] != "--target-parent-index"
            || args[17] != "--target-parent-name"
            || args[19] != "--expected-target-order"
        {
            eprintln!(
                "error: expected rekordbox move-playlist --xml IN --output NEW --playlist-index N --expected-name NAME --expected-members IDs --source-parent-index N --source-parent-name NAME --target-parent-index N --target-parent-name NAME --expected-target-order NAMES"
            );
            return USAGE;
        }
        let parse_index = |index: usize| args[index].parse::<usize>();
        let (Ok(playlist_index), Ok(source_parent_index), Ok(target_parent_index)) =
            (parse_index(6), parse_index(12), parse_index(16))
        else {
            eprintln!("error: playlist and folder indices must be non-negative integers");
            return USAGE;
        };
        let parse_list = |value: &str| {
            if value.is_empty() {
                Vec::new()
            } else {
                value.split(',').map(str::to_owned).collect()
            }
        };
        let request = oxidj_application::PlaylistMoveRequest {
            playlist_index,
            expected_playlist_name: args[8].clone(),
            expected_members: parse_list(&args[10]),
            source_parent_index,
            expected_source_parent_name: args[14].clone(),
            target_parent_index,
            expected_target_parent_name: args[18].clone(),
            expected_target_order: parse_list(&args[20]),
        };
        return match oxidj_application::export_rekordbox_playlist_move(&args[2], &args[4], &request)
        {
            Ok(()) => {
                println!(
                    "Moved playlist[{}] to folder[{}]",
                    playlist_index, target_parent_index
                );
                OK
            }
            Err(error) => {
                eprintln!("error: playlist move rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "move-folder") {
        if args.len() != 21
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--folder-index"
            || args[7] != "--expected-name"
            || args[9] != "--source-parent-index"
            || args[11] != "--source-parent-name"
            || args[13] != "--expected-source-order"
            || args[15] != "--target-parent-index"
            || args[17] != "--target-parent-name"
            || args[19] != "--expected-target-order"
        {
            eprintln!(
                "error: expected rekordbox move-folder --xml IN --output NEW --folder-index N --expected-name NAME --source-parent-index N --source-parent-name NAME --expected-source-order TYPE:NAME,... --target-parent-index N --target-parent-name NAME --expected-target-order TYPE:NAME,..."
            );
            return USAGE;
        }
        let parse_index = |index: usize| args[index].parse::<usize>();
        let (Ok(folder_index), Ok(source_parent_index), Ok(target_parent_index)) =
            (parse_index(6), parse_index(10), parse_index(16))
        else {
            eprintln!("error: folder indices must be non-negative integers");
            return USAGE;
        };
        let parse_order = |value: &str| {
            if value.is_empty() {
                Vec::new()
            } else {
                value.split(',').map(str::to_owned).collect()
            }
        };
        let request = oxidj_application::FolderMoveRequest {
            folder_index,
            expected_folder_name: args[8].clone(),
            source_parent_index,
            expected_source_parent_name: args[12].clone(),
            expected_source_order: parse_order(&args[14]),
            target_parent_index,
            expected_target_parent_name: args[18].clone(),
            expected_target_order: parse_order(&args[20]),
        };
        return match oxidj_application::export_rekordbox_folder_move(&args[2], &args[4], &request) {
            Ok(()) => {
                println!(
                    "Moved folder[{}] to folder[{}]",
                    folder_index, target_parent_index
                );
                OK
            }
            Err(error) => {
                eprintln!("error: folder move rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "rename-playlist")
    {
        if args.len() != 11
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--playlist-index"
            || args[7] != "--expected-name"
            || args[9] != "--name"
        {
            eprintln!(
                "error: expected rekordbox rename-playlist --xml IN --output NEW --playlist-index N --expected-name OLD --name NEW"
            );
            return USAGE;
        }
        let Ok(playlist_index) = args[6].parse::<usize>() else {
            eprintln!("error: playlist index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PlaylistRenameRequest {
            playlist_index,
            expected_old_name: args[8].clone(),
            new_name: args[10].clone(),
        };
        return match oxidj_application::export_rekordbox_playlist_rename(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Renamed playlist[{}]: {:?} to {:?}",
                    change.playlist_index, change.old_name, change.new_name
                );
                OK
            }
            Err(error) => {
                eprintln!("error: playlist rename rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "delete-mark") {
        if !(args.len() == 16 || args.len() == 17)
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--mark-index"
            || args[9] != "--type"
            || args[11] != "--start"
            || args[13] != "--name"
        {
            eprintln!(
                "error: expected rekordbox delete-mark --xml IN --output NEW --track-id ID --mark-index N --type RAW --start RAW --name RAW --end VALUE|--no-end"
            );
            return USAGE;
        }
        let Ok(mark_index) = args[8].parse::<usize>() else {
            eprintln!("error: mark index must be a non-negative integer");
            return USAGE;
        };
        let end = match args.get(15).map(String::as_str) {
            Some("--end") if args.len() == 17 => Some(args[16].clone()),
            Some("--no-end") if args.len() == 16 => None,
            _ => {
                eprintln!("error: specify --end VALUE or --no-end");
                return USAGE;
            }
        };
        let request = oxidj_application::PositionMarkDeleteRequest {
            track_id: args[6].clone(),
            mark_index,
            expected_type: args[10].clone(),
            expected_start: args[12].clone(),
            expected_name: args[14].clone(),
            expected_end: end,
        };
        return match oxidj_application::export_rekordbox_position_mark_delete(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Deleted TrackID={} POSITION_MARK[{}] Type={} Start={} Name={:?} End={:?}",
                    change.track_id,
                    change.mark_index,
                    change.deleted_type,
                    change.deleted_start,
                    change.deleted_name,
                    change.deleted_end
                );
                OK
            }
            Err(error) => {
                eprintln!("error: position mark delete rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "edit-mark-end") {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--mark-index"
            || args[9] != "--expected-end"
            || args[11] != "--end"
        {
            eprintln!(
                "error: expected rekordbox edit-mark-end --xml IN --output NEW --track-id ID --mark-index N --expected-end OLD --end NEW"
            );
            return USAGE;
        }
        let Ok(mark_index) = args[8].parse::<usize>() else {
            eprintln!("error: mark index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PositionMarkEndEditRequest {
            track_id: args[6].clone(),
            mark_index,
            expected_old_end_seconds: args[10].clone(),
            new_end_seconds: args[12].clone(),
        };
        return match oxidj_application::export_rekordbox_position_mark_end_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Edited TrackID={} POSITION_MARK[{}].End {} to {}",
                    change.track_id,
                    change.mark_index,
                    change.old_end_seconds,
                    change.new_end_seconds
                );
                OK
            }
            Err(error) => {
                eprintln!("error: position mark End edit rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "edit-mark-name")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--mark-index"
            || args[9] != "--expected-name"
            || args[11] != "--name"
        {
            eprintln!(
                "error: expected rekordbox edit-mark-name --xml IN --output NEW --track-id ID --mark-index N --expected-name OLD --name NEW"
            );
            return USAGE;
        }
        let Ok(mark_index) = args[8].parse::<usize>() else {
            eprintln!("error: mark index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PositionMarkNameEditRequest {
            track_id: args[6].clone(),
            mark_index,
            expected_old_name: args[10].clone(),
            new_name: args[12].clone(),
        };
        return match oxidj_application::export_rekordbox_position_mark_name_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Renamed TrackID={} POSITION_MARK[{}]: {:?} to {:?}",
                    change.track_id, change.mark_index, change.old_name, change.new_name
                );
                OK
            }
            Err(error) => {
                eprintln!("error: position mark name edit rejected: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "edit-position") {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--mark-index"
            || args[9] != "--expected-start"
            || args[11] != "--start"
        {
            eprintln!(
                "error: expected rekordbox edit-position --xml IN --output NEW --track-id ID --mark-index N --expected-start OLD --start NEW"
            );
            return USAGE;
        }
        let Ok(mark_index) = args[8].parse::<usize>() else {
            eprintln!("error: mark index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::PositionMarkEditRequest {
            track_id: args[6].clone(),
            mark_index,
            expected_old_start_seconds: args[10].clone(),
            new_start_seconds: args[12].clone(),
        };
        let prepared =
            match oxidj_application::prepare_rekordbox_position_mark_edit(&args[2], &request) {
                Ok(prepared) => prepared,
                Err(error) => {
                    eprintln!("error: position mark edit rejected: {error}");
                    return DOMAIN;
                }
            };
        println!(
            "Planned: TrackID={} POSITION_MARK[{}].Start {} to {}",
            prepared.change.track_id,
            prepared.change.mark_index,
            prepared.change.old_start_seconds,
            prepared.change.new_start_seconds
        );
        return match oxidj_application::export_rekordbox_position_mark_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(_) => {
                println!("Wrote new XML: {}", args[4]);
                OK
            }
            Err(error) => {
                eprintln!("error: position mark edit export failed: {error}");
                DOMAIN
            }
        };
    }
    if args.first().is_some_and(|action| action == "edit-tempo") {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--tempo-index"
            || args[9] != "--expected-bpm"
            || args[11] != "--bpm"
        {
            eprintln!(
                "error: expected rekordbox edit-tempo --xml IN --output NEW --track-id ID --tempo-index N --expected-bpm OLD --bpm NEW"
            );
            return USAGE;
        }
        let Ok(tempo_index) = args[8].parse::<usize>() else {
            eprintln!("error: tempo index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::TempoEditRequest {
            track_id: args[6].clone(),
            tempo_index,
            expected_old_bpm: args[10].clone(),
            new_bpm: args[12].clone(),
        };
        let prepared = match oxidj_application::prepare_rekordbox_tempo_edit(&args[2], &request) {
            Ok(prepared) => prepared,
            Err(error) => {
                eprintln!("error: tempo edit rejected: {error}");
                return DOMAIN;
            }
        };
        println!(
            "Planned: TrackID={} TEMPO[{}].Bpm {} to {}",
            prepared.change.track_id,
            prepared.change.tempo_index,
            prepared.change.old_bpm,
            prepared.change.new_bpm
        );
        return match oxidj_application::export_rekordbox_tempo_edit(&args[2], &args[4], &request) {
            Ok(_) => {
                println!("Wrote new XML: {}", args[4]);
                OK
            }
            Err(error) => {
                eprintln!("error: tempo edit export failed: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "edit-tempo-beat")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--tempo-index"
            || args[9] != "--expected-beat"
            || args[11] != "--beat"
        {
            eprintln!(
                "error: expected rekordbox edit-tempo-beat --xml IN --output NEW --track-id ID --tempo-index N --expected-beat OLD --beat NEW"
            );
            return USAGE;
        }
        let Ok(tempo_index) = args[8].parse::<usize>() else {
            eprintln!("error: tempo index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::TempoBeatEditRequest {
            track_id: args[6].clone(),
            tempo_index,
            expected_old_beat: args[10].clone(),
            new_beat: args[12].clone(),
        };
        return match oxidj_application::export_rekordbox_tempo_beat_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Edited TrackID={} TEMPO[{}].Battito {} to {}",
                    change.track_id, change.tempo_index, change.old_beat, change.new_beat
                );
                OK
            }
            Err(error) => {
                eprintln!("error: tempo Battito edit rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "edit-tempo-meter")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--tempo-index"
            || args[9] != "--expected-meter"
            || args[11] != "--meter"
        {
            eprintln!(
                "error: expected rekordbox edit-tempo-meter --xml IN --output NEW --track-id ID --tempo-index N --expected-meter OLD --meter NEW"
            );
            return USAGE;
        }
        let Ok(tempo_index) = args[8].parse::<usize>() else {
            eprintln!("error: tempo index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::TempoMeterEditRequest {
            track_id: args[6].clone(),
            tempo_index,
            expected_old_meter: args[10].clone(),
            new_meter: args[12].clone(),
        };
        return match oxidj_application::export_rekordbox_tempo_meter_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(change) => {
                println!(
                    "Edited TrackID={} TEMPO[{}].Metro {:?} to {:?}",
                    change.track_id, change.tempo_index, change.old_meter, change.new_meter
                );
                OK
            }
            Err(error) => {
                eprintln!("error: tempo Metro edit rejected: {error}");
                DOMAIN
            }
        };
    }
    if args
        .first()
        .is_some_and(|action| action == "edit-tempo-start")
    {
        if args.len() != 13
            || args[1] != "--xml"
            || args[3] != "--output"
            || args[5] != "--track-id"
            || args[7] != "--tempo-index"
            || args[9] != "--expected-start"
            || args[11] != "--start"
        {
            eprintln!(
                "error: expected rekordbox edit-tempo-start --xml IN --output NEW --track-id ID --tempo-index N --expected-start OLD --start NEW"
            );
            return USAGE;
        }
        let Ok(tempo_index) = args[8].parse::<usize>() else {
            eprintln!("error: tempo index must be a non-negative integer");
            return USAGE;
        };
        let request = oxidj_application::TempoStartEditRequest {
            track_id: args[6].clone(),
            tempo_index,
            expected_old_start_seconds: args[10].clone(),
            new_start_seconds: args[12].clone(),
        };
        let prepared =
            match oxidj_application::prepare_rekordbox_tempo_start_edit(&args[2], &request) {
                Ok(prepared) => prepared,
                Err(error) => {
                    eprintln!("error: tempo start edit rejected: {error}");
                    return DOMAIN;
                }
            };
        println!(
            "Planned: TrackID={} TEMPO[{}].Inizio {} to {}",
            prepared.change.track_id,
            prepared.change.tempo_index,
            prepared.change.old_start_seconds,
            prepared.change.new_start_seconds
        );
        return match oxidj_application::export_rekordbox_tempo_start_edit(
            &args[2], &args[4], &request,
        ) {
            Ok(_) => {
                println!("Wrote new XML: {}", args[4]);
                OK
            }
            Err(error) => {
                eprintln!("error: tempo start edit export failed: {error}");
                DOMAIN
            }
        };
    }
    if args.len() == 3 && args[0] == "export-plan" && args[1] == "--xml" {
        return match oxidj_application::preflight_rekordbox_xml(&args[2]) {
            Ok(plan) => {
                let decision = match plan.decision {
                    oxidj_application::ExportDecision::AllowExport => "ALLOW_EXPORT",
                };
                println!("Decision: {decision}");
                println!("Selected units:");
                for track in plan.selected_tracks {
                    println!(
                        "Track\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                        track.id,
                        track.name,
                        track.artist,
                        track.comments,
                        track.album,
                        track.genre,
                        track.label,
                        track.composer,
                        track.grouping,
                        track.remixer,
                        track.mix,
                        track.tonality,
                        track.location,
                        track.kind,
                        track.year,
                        track.size,
                        track.disc_number,
                        track.track_number,
                        track.total_time,
                        track.bit_rate,
                        track.sample_rate,
                        track.date_added,
                        track.date_modified,
                        track.last_played,
                        track.play_count,
                        track.rating,
                        track.colour,
                        track.average_bpm
                    );
                    for tempo in track.tempos {
                        println!(
                            "TEMPO\t{}\t{}\t{}\t{}\t{}",
                            track.id, tempo.start_seconds, tempo.bpm, tempo.meter, tempo.beat
                        );
                    }
                    for mark in track.marks {
                        println!(
                            "POSITION_MARK\t{}\t{}\t{}\t{}\t{}\t{:?}",
                            track.id,
                            mark.name,
                            mark.kind,
                            mark.start_seconds,
                            mark.number,
                            mark.end_seconds
                        );
                    }
                }
                for playlist in plan.selected_playlists {
                    println!(
                        "Playlist\t{}\t{}",
                        playlist.path.join("/"),
                        playlist.tracks.join(",")
                    );
                }
                println!("Withheld units: {}", plan.withheld_units.len());
                println!("Loss report: {:?}", plan.loss_report);
                OK
            }
            Err(error) => {
                eprintln!("error: export preflight rejected input: {error}");
                DOMAIN
            }
        };
    }
    if args.len() == 5 && args[0] == "export" && args[1] == "--xml" && args[3] == "--output" {
        return match oxidj_application::export_rekordbox_xml(&args[2], &args[4]) {
            Ok(()) => OK,
            Err(e) => {
                eprintln!("error: {e}");
                DOMAIN
            }
        };
    }
    if args.len() != 3 || args[0] != "preview" || args[1] != "--xml" {
        eprintln!(
            "error: expected rekordbox preview --xml PATH, export-plan --xml PATH, edit-position --xml IN --output NEW --track-id ID --mark-index N --expected-start OLD --start NEW, edit-tempo --xml IN --output NEW --track-id ID --tempo-index N --expected-bpm OLD --bpm NEW, edit-tempo-beat --xml IN --output NEW --track-id ID --tempo-index N --expected-beat OLD --beat NEW, edit-tempo-meter --xml IN --output NEW --track-id ID --tempo-index N --expected-meter OLD --meter NEW, or export --xml IN --output NEW"
        );
        return USAGE;
    }
    let Some(path) = args.get(2) else {
        eprintln!("error: rekordbox preview requires --xml PATH");
        return USAGE;
    };
    match oxidj_application::preview_rekordbox_xml(path) {
        Ok(p) => {
            println!("Tracks:");
            for t in p.tracks {
                println!(
                    "{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                    t.id,
                    t.name,
                    t.artist,
                    t.comments,
                    t.album,
                    t.genre,
                    t.label,
                    t.tonality,
                    t.composer,
                    t.grouping,
                    t.remixer,
                    t.mix,
                    t.location,
                    t.kind,
                    t.year,
                    t.size,
                    t.disc_number,
                    t.track_number,
                    t.total_time,
                    t.bit_rate,
                    t.sample_rate,
                    t.date_added,
                    t.date_modified,
                    t.last_played,
                    t.play_count,
                    t.rating,
                    t.colour,
                    t.average_bpm
                );
                for x in t.tempos {
                    println!(
                        "  TEMPO\t{}\t{}\t{}\t{}",
                        x.start_seconds, x.bpm, x.meter, x.beat
                    );
                }
                for (ordinal, x) in t.marks.into_iter().enumerate() {
                    println!(
                        "  POSITION_MARK\t{}\t{}\tType={}\tStart={}\tNum={}\tName={:?}\tEnd={:?}",
                        t.id, ordinal, x.kind, x.start_seconds, x.number, x.name, x.end_seconds
                    );
                }
            }
            println!("Folders:");
            for folder in p.folders {
                println!(
                    "Folder\t{}\tPath={:?}\tName={:?}\tCount={:?}",
                    folder.ordinal, folder.path, folder.name, folder.count
                );
            }
            println!("Playlists:");
            for (playlist, node) in p.playlists.into_iter().zip(p.playlist_nodes) {
                println!(
                    "{}\t{}\tEntries={:?}\tKeyType={:?}",
                    playlist.path.join("/"),
                    playlist.tracks.join(","),
                    node.entries,
                    node.key_type
                );
            }
            for x in p.unsupported {
                eprintln!("warning: unsupported content: {x}");
            }
            OK
        }
        Err(e) => {
            eprintln!("error: {e}");
            DOMAIN
        }
    }
}

fn run_catalog(args: &[String]) -> i32 {
    let Some(action) = args.first().map(String::as_str) else {
        eprintln!("error: catalog requires init, list, backup, or restore");
        return USAGE;
    };
    if action == "import-library" {
        return run_import_library(&args[1..]);
    }
    if action == "create-library-playlist" {
        return run_create_library_playlist(&args[1..]);
    }
    if action == "append-library-playlist-members" {
        return run_append_library_playlist_members(&args[1..]);
    }
    if action == "list-library-tracks" {
        return run_list_library_tracks(&args[1..]);
    }
    if action == "export-library-xml" {
        return run_export_library_xml(&args[1..]);
    }
    if action == "analyze-waveform" {
        return run_analyze_waveform(&args[1..]);
    }
    if action == "analyze-tempo" {
        return run_analyze_tempo(&args[1..]);
    }
    if action == "import" {
        let Some(dir_index) = args.iter().position(|arg| arg == "--dir") else {
            eprintln!("error: catalog import requires --dir PATH");
            return USAGE;
        };
        let Some(db_index) = args.iter().position(|arg| arg == "--db") else {
            eprintln!("error: catalog import requires --db PATH");
            return USAGE;
        };
        if args.len() != 5 || dir_index + 1 >= args.len() || db_index + 1 >= args.len() {
            eprintln!("error: invalid catalog import arguments");
            return USAGE;
        }
        let directory = &args[dir_index + 1];
        let db = &args[db_index + 1];
        return match Catalog::open(db)
            .and_then(|mut catalog| catalog.import_audio_directory(directory))
        {
            Ok(report) => {
                for outcome in report.outcomes {
                    println!("{:?}\t{}", outcome.status, outcome.path);
                }
                if report.complete {
                    OK
                } else {
                    eprintln!("warning: scan incomplete; missing status unchanged");
                    DOMAIN
                }
            }
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        };
    }
    if action == "backup" || action == "restore" {
        let (source_flag, destination_flag) = if action == "backup" {
            ("--db", "--to")
        } else {
            ("--from", "--db")
        };
        let Some(source_index) = args.iter().position(|arg| arg == source_flag) else {
            eprintln!("error: catalog {action} requires {source_flag} PATH");
            return USAGE;
        };
        let Some(destination_index) = args.iter().position(|arg| arg == destination_flag) else {
            eprintln!("error: catalog {action} requires {destination_flag} PATH");
            return USAGE;
        };
        if args.len() != 5 || source_index + 1 >= args.len() || destination_index + 1 >= args.len()
        {
            eprintln!("error: invalid catalog {action} arguments");
            return USAGE;
        }
        let source = &args[source_index + 1];
        let destination = &args[destination_index + 1];
        let result = if action == "backup" {
            Catalog::backup_from(source, destination)
        } else {
            Catalog::restore_from(source, destination)
        };
        return match result {
            Ok(()) => OK,
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        };
    }
    let Some(db_index) = args.iter().position(|arg| arg == "--db") else {
        eprintln!("error: catalog {action} requires --db PATH");
        return USAGE;
    };
    let Some(path) = args.get(db_index + 1) else {
        eprintln!("error: --db requires a path");
        return USAGE;
    };
    if args.len() != db_index + 2 {
        eprintln!("error: unexpected catalog argument");
        return USAGE;
    }

    match action {
        "init" => match Catalog::open(path) {
            Ok(_) => OK,
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        },
        "list" => match Catalog::open(path).and_then(|catalog| catalog.list_tracks()) {
            Ok(tracks) => {
                println!("TrackId\tPath\tSHA-256\tBytes");
                for track in tracks {
                    let digest = track
                        .sha256
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    println!(
                        "{}\t{}\t{}\t{}",
                        track.id, track.path, digest, track.byte_length
                    );
                }
                OK
            }
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        },
        _ => {
            eprintln!("error: unknown catalog action: {action}");
            USAGE
        }
    }
}

/// `catalog analyze-waveform --db DB [--track-id ID]`: compute and store waveform
/// peaks. Prints one line per track (ID and outcome, never paths) and a summary;
/// exits non-zero when any track was refused.
fn run_analyze_waveform(args: &[String]) -> i32 {
    let (db, track) = match args {
        [a, db] if a == "--db" => (db, None),
        [a, db, b, id] | [b, id, a, db] if a == "--db" && b == "--track-id" => {
            match id
                .parse::<u64>()
                .ok()
                .and_then(|n| oxidj_core::TrackId::new(n).ok())
            {
                Some(id) => (db, Some(id)),
                None => {
                    eprintln!("error: --track-id requires a positive integer");
                    return USAGE;
                }
            }
        }
        _ => {
            eprintln!("error: expected catalog analyze-waveform --db PATH [--track-id ID]");
            return USAGE;
        }
    };
    match oxidj_application::analyze_catalog_waveforms(db, track) {
        Ok(report) => {
            for (id, outcome) in &report.outcomes {
                match outcome {
                    Ok(outcome) => println!("{outcome:?}\t{id}"),
                    Err(error) => println!("Failed\t{id}\t{error}"),
                }
            }
            println!("Analyzed: {}", report.analyzed());
            println!("Unchanged: {}", report.unchanged());
            println!("Failed: {}", report.failed());
            if report.failed() == 0 { OK } else { DOMAIN }
        }
        Err(error) => {
            eprintln!("error: {error}");
            DOMAIN
        }
    }
}

/// `catalog analyze-tempo --db DB [--track-id ID] [--bpm-range LO-HI]`: compute
/// and store tempo/beat grids (default range 70-180). Prints one line per track
/// (ID and outcome, never paths) and a summary; `NoGrid` is not a failure; exits
/// non-zero when any track failed.
fn run_analyze_tempo(args: &[String]) -> i32 {
    const USAGE_TEXT: &str =
        "error: expected catalog analyze-tempo --db PATH [--track-id ID] [--bpm-range LO-HI]";
    let (mut db, mut track, mut range) = (None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            eprintln!("{USAGE_TEXT}");
            return USAGE;
        };
        match flag.as_str() {
            "--db" if db.is_none() => db = Some(value),
            "--track-id" if track.is_none() => {
                match value
                    .parse::<u64>()
                    .ok()
                    .and_then(|n| oxidj_core::TrackId::new(n).ok())
                {
                    Some(id) => track = Some(id),
                    None => {
                        eprintln!("error: --track-id requires a positive integer");
                        return USAGE;
                    }
                }
            }
            "--bpm-range" if range.is_none() => {
                let parsed = value.split_once('-').and_then(|(lo, hi)| {
                    let (lo, hi) = (lo.parse::<u32>().ok()?, hi.parse::<u32>().ok()?);
                    (lo > 0 && u64::from(hi) >= 2 * u64::from(lo)).then_some((lo, hi))
                });
                match parsed {
                    Some(parsed) => range = Some(parsed),
                    None => {
                        eprintln!(
                            "error: --bpm-range requires LO-HI whole BPM with HI >= 2 * LO (e.g. 70-180)"
                        );
                        return USAGE;
                    }
                }
            }
            _ => {
                eprintln!("{USAGE_TEXT}");
                return USAGE;
            }
        }
    }
    let Some(db) = db else {
        eprintln!("{USAGE_TEXT}");
        return USAGE;
    };
    let range = range.unwrap_or(oxidj_application::DEFAULT_TEMPO_RANGE);
    match oxidj_application::analyze_catalog_tempo(db, track, range) {
        Ok(report) => {
            for (id, outcome) in &report.outcomes {
                match outcome {
                    Ok(outcome) => println!("{outcome:?}\t{id}"),
                    Err(error) => println!("Failed\t{id}\t{error}"),
                }
            }
            for (id, key) in &report.keys {
                match key {
                    Ok(Some(key)) => println!("Key\t{id}\t{key:?}"),
                    Ok(None) => println!("NoKey\t{id}"),
                    Err(error) => println!("KeyFailed\t{id}\t{error}"),
                }
            }
            println!("Analyzed: {}", report.analyzed());
            println!("Unchanged: {}", report.unchanged());
            println!("NoGrid: {}", report.no_grid());
            println!("Failed: {}", report.failed());
            if report.unavailable_library_audio > 0 {
                println!(
                    "UnavailableLibraryAudio: {}",
                    report.unavailable_library_audio
                );
            }
            if report.failed() == 0 { OK } else { DOMAIN }
        }
        Err(error) => {
            eprintln!("error: {error}");
            DOMAIN
        }
    }
}

fn print_counts(counts: &oxidj_application::LibraryCounts) {
    println!("Tracks: {}", counts.tracks);
    println!("Folders: {}", counts.folders);
    println!("Playlists: {}", counts.playlists);
    println!("Memberships: {}", counts.memberships);
}

fn print_not_carried(not_carried: &oxidj_application::NotCarried, tracks: usize) {
    for (name, count) in &not_carried.absent {
        println!(
            "Carried\tTRACK@{name}\t{}",
            not_carried.carried(name, tracks)
        );
        println!("AbsentInSource\t{name}\t{count}");
    }
    for (name, count) in &not_carried.elements_absent {
        println!(
            "Carried\tTRACK/{name}\t{}",
            not_carried.carried(name, tracks)
        );
        println!("AbsentInSource\t{name}\t{count}");
    }
    for (name, count) in &not_carried.track_attributes {
        println!("NotCarried\tTRACK@{name}\t{count}");
    }
    for (name, count) in &not_carried.track_elements {
        println!("NotCarried\tTRACK/{name}\t{count}");
    }
}

/// `catalog import-library --pdb SRC --catalog NEW [--onelibrary-db DB --key-stdin]`.
/// Aggregate-only output: counts and booleans, never names, paths or IDs.
fn run_import_library(args: &[String]) -> i32 {
    let (pdb, catalog, onelibrary) = match args {
        [a, pdb, b, catalog] if a == "--pdb" && b == "--catalog" => (pdb, catalog, None),
        [a, pdb, b, catalog, c, db, d]
            if a == "--pdb" && b == "--catalog" && c == "--onelibrary-db" && d == "--key-stdin" =>
        {
            (pdb, catalog, Some(std::path::Path::new(db.as_str())))
        }
        _ => {
            eprintln!(
                "error: expected catalog import-library --pdb SRC --catalog NEW [--onelibrary-db DB --key-stdin]"
            );
            return USAGE;
        }
    };
    match oxidj_application::import_library_from_pdb(pdb, catalog, onelibrary) {
        Ok(report) => {
            print_counts(&report.counts);
            println!("LocationsUriEncoded: {}", report.locations_uri_encoded);
            println!("SourceUnchanged: {}", report.source_unchanged);
            println!("CatalogMatchesSource: {}", report.catalog_matches_source);
            if let Some(check) = report.onelibrary {
                println!(
                    "OneLibraryCrossCheck\ttracks={}\tplaylists={}\tmemberships={}\ttracks_equal={}\tplaylists_equal={}\tmemberships_equal={}\tsource_unchanged={}",
                    check.tracks,
                    check.playlists,
                    check.memberships,
                    check.tracks_equal,
                    check.playlists_equal,
                    check.memberships_equal,
                    check.source_unchanged
                );
            }
            print_not_carried(&report.not_carried, report.counts.tracks);
            for (name, count) in &report.unresolved {
                println!("UnresolvedInSource\t{name}\t{count}");
            }
            OK
        }
        Err(error) => {
            eprintln!("error: import-library refused: {error}");
            DOMAIN
        }
    }
}

/// Create in the catalog, never in the original vendor database. No SQL here.
fn run_create_library_playlist(args: &[String]) -> i32 {
    const EXPECTED: &str = "error: expected catalog create-library-playlist --catalog DB --name NAME --track-ids 3,1,3 (exported XML TrackIDs)";
    let (mut catalog, mut name, mut ids) = (None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            eprintln!("{EXPECTED}");
            return USAGE;
        };
        let slot = match flag.as_str() {
            "--catalog" => &mut catalog,
            "--name" => &mut name,
            "--track-ids" => &mut ids,
            _ => {
                eprintln!("{EXPECTED}");
                return USAGE;
            }
        };
        if slot.replace(value).is_some() {
            eprintln!("{EXPECTED}");
            return USAGE;
        }
    }
    let (Some(catalog), Some(name), Some(ids)) = (catalog, name, ids) else {
        eprintln!("{EXPECTED}");
        return USAGE;
    };
    match oxidj_application::create_library_playlist(catalog, name, ids) {
        Ok(memberships) => {
            println!("CreatedPlaylists: 1");
            println!("Memberships: {memberships}");
            OK
        }
        Err(error) => {
            eprintln!("error: create-library-playlist refused: {error}");
            DOMAIN
        }
    }
}

/// Reversible TSV: escape backslash first; missing artist is the reserved `\N`.
fn escape_track_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn run_append_library_playlist_members(args: &[String]) -> i32 {
    const EXPECTED: &str = "error: expected catalog append-library-playlist-members --catalog DB --playlist-name NAME --track-ids 3,1 (exported XML TrackIDs)";
    let (mut catalog, mut name, mut ids) = (None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            eprintln!("{EXPECTED}");
            return USAGE;
        };
        let slot = match flag.as_str() {
            "--catalog" => &mut catalog,
            "--playlist-name" => &mut name,
            "--track-ids" => &mut ids,
            _ => {
                eprintln!("{EXPECTED}");
                return USAGE;
            }
        };
        if slot.replace(value).is_some() {
            eprintln!("{EXPECTED}");
            return USAGE;
        }
    }
    let (Some(catalog), Some(name), Some(ids)) = (catalog, name, ids) else {
        eprintln!("{EXPECTED}");
        return USAGE;
    };
    match oxidj_application::append_library_playlist_members(catalog, name, ids) {
        Ok(count) => {
            println!("AppendedMemberships: {count}");
            OK
        }
        Err(error) => {
            eprintln!("error: append-library-playlist-members refused: {error}");
            DOMAIN
        }
    }
}

fn run_list_library_tracks(args: &[String]) -> i32 {
    const EXPECTED: &str = "error: expected catalog list-library-tracks --catalog DB";
    if args.len() != 2 || args[0] != "--catalog" {
        eprintln!("{EXPECTED}");
        return USAGE;
    }
    match oxidj_application::list_library_tracks(&args[1]) {
        Ok(tracks) => {
            println!("ExportedTrackID\tTitle\tArtist");
            if tracks.is_empty() {
                println!("(no imported tracks)");
            }
            for (id, title, artist) in tracks {
                let artist = artist
                    .as_deref()
                    .map(escape_track_field)
                    .unwrap_or_else(|| "\\N".into());
                println!("{id}\t{}\t{artist}", escape_track_field(&title));
            }
            OK
        }
        Err(error) => {
            eprintln!("error: list-library-tracks refused: {error}");
            DOMAIN
        }
    }
}

/// `catalog export-library-xml --catalog DB --output NEW [--drive X] [--computed-grids]`:
/// no-edit, new-file-only. `--drive` qualifies device-relative locations for
/// Windows; `--computed-grids` writes OxiDJ's computed grid for tracks without a
/// vendor grid and prints per-decision counts only.
fn run_export_library_xml(args: &[String]) -> i32 {
    const EXPECTED: &str = "error: expected catalog export-library-xml --catalog DB --output NEW [--drive X] [--computed-grids]";
    let (mut catalog, mut output, mut drive, mut computed) = (None, None, None, false);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--computed-grids" if !computed => computed = true,
            "--catalog" | "--output" | "--drive" => {
                let Some(value) = rest.next() else {
                    eprintln!("{EXPECTED}");
                    return USAGE;
                };
                match flag.as_str() {
                    "--catalog" if catalog.is_none() => catalog = Some(value),
                    "--output" if output.is_none() => output = Some(value),
                    "--drive" if drive.is_none() => {
                        let Some(letter) = oxidj_application::DriveLetter::parse(value) else {
                            eprintln!(
                                "error: --drive expects a single drive letter A-Z (e.g. D), got {value:?}"
                            );
                            return USAGE;
                        };
                        drive = Some(letter);
                    }
                    _ => {
                        eprintln!("{EXPECTED}");
                        return USAGE;
                    }
                }
            }
            _ => {
                eprintln!("{EXPECTED}");
                return USAGE;
            }
        }
    }
    let (Some(catalog), Some(output)) = (catalog, output) else {
        eprintln!("{EXPECTED}");
        return USAGE;
    };
    match oxidj_application::export_library_xml(catalog, output, drive, computed) {
        Ok(report) => {
            print_counts(&report.counts);
            println!("PreviewVerified: {}", report.preview_verified);
            print_not_carried(&report.not_carried, report.counts.tracks);
            if computed {
                for decision in oxidj_application::ComputedGridDecision::ALL {
                    let count = report
                        .computed_grids
                        .iter()
                        .filter(|d| **d == decision)
                        .count();
                    println!("{}: {count}", decision.label());
                }
            }
            OK
        }
        Err(error) => {
            eprintln!("error: export-library-xml refused: {error}");
            DOMAIN
        }
    }
}

fn print_usage(to_stdout: bool) {
    let output = if to_stdout {
        &mut std::io::stdout().lock() as &mut dyn std::io::Write
    } else {
        &mut std::io::stderr().lock() as &mut dyn std::io::Write
    };
    let _ = writeln!(output, "usage: oxidj-cli <command>");
    let _ = writeln!(output, "commands:");
    let _ = writeln!(
        output,
        "  onelibrary preview --db <path> --key-stdin   Read-only provisional native-profile preview when the pinned 22-table/4-index signature matches; otherwise synthetic v1. Supports title, artist, album, playlist hierarchy/order and membership only. Excludes BPM, length, paths, cues/ANLZ, history, tags, color, artwork and analysis.
  rekordbox preview --xml <path>   Read-only Rekordbox XML preview
  rekordbox pdb-preview --pdb <path> Read-only legacy DeviceSQL PDB preview
  catalog import-library --pdb <src> --catalog <new> [--onelibrary-db <db> --key-stdin]   Import a PDB library into a new catalog (aggregate-only report)
  catalog list-library-tracks --catalog <db>   List exported XML TrackIDs, titles and artists as escaped TSV (PERSONAL METADATA: do not log or transmit)
  catalog create-library-playlist --catalog <db> --name <name> --track-ids <3,1,3>   Append an ordered root playlist in the catalog; IDs are exported XML TrackIDs, not audio/vendor IDs
  catalog append-library-playlist-members --catalog <db> --playlist-name <name> --track-ids <3,1>   Append ordered members to an exact root playlist
  catalog export-library-xml --catalog <db> --output <new> [--drive <X>] [--computed-grids]   Export the imported library as new Rekordbox XML (no edits); --drive prefixes device-relative locations with X: for Windows Rekordbox; --computed-grids adds one TEMPO from OxiDJ's computed grid (confidence >= 0.5) to tracks without a Rekordbox grid",
    );
    let _ = writeln!(
        output,
        "  rekordbox export-plan --xml <path>   Read-only bounded export preflight"
    );
    let _ = writeln!(
        output,
        "  rekordbox rename-folder --xml <in> --output <new> --folder-index <n> --expected-name <old> --name <new>   Export one guarded folder rename"
    );
    let _ = writeln!(
        output,
        "  rekordbox create-folder --xml <in> --output <new> --parent-index <n> --expected-name <name> --name <name>   Create one guarded empty child folder
  rekordbox create-playlist --xml <in> --output <new> --parent-index <n> --expected-name <name> --name <name>   Create one guarded empty child playlist"
    );
    let _ = writeln!(
        output,
        "  rekordbox delete-empty-folder --xml <in> --output <new> --folder-index <n> --expected-name <name>   Delete one guarded empty non-root folder"
    );
    let _ = writeln!(
        output,
        "  rekordbox rename-playlist --xml <in> --output <new> --playlist-index <n> --expected-name <old> --name <new>   Export one guarded playlist rename"
    );
    let _ = writeln!(
        output,
        "  rekordbox reorder-sibling-playlists --xml <in> --output <new> --parent-index <n> --expected-name <name> --expected-order <names> --order <names>   Reorder direct playlists in one folder"
    );
    let _ = writeln!(
        output,
        "  rekordbox reorder-sibling-folders --xml <in> --output <new> --parent-index <n> --expected-name <name> --expected-order <names> --order <names>   Reorder direct folders in one folder"
    );
    let _ = writeln!(
        output,
        "  rekordbox move-playlist --xml <in> --output <new> --playlist-index <n> --expected-name <name> --expected-members <IDs> --source-parent-index <n> --source-parent-name <name> --target-parent-index <n> --target-parent-name <name> --expected-target-order <names>   Move one playlist to another folder"
    );
    let _ = writeln!(
        output,
        "  rekordbox move-folder --xml <in> --output <new> --folder-index <n> --expected-name <name> --source-parent-index <n> --source-parent-name <name> --expected-source-order <typed-names> --target-parent-index <n> --target-parent-name <name> --expected-target-order <typed-names>   Move one folder subtree to another folder"
    );
    let _ = writeln!(
        output,
        "  rekordbox delete-playlist --xml <in> --output <new> --playlist-index <n> --expected-name <old> --expected-order <IDs>   Delete one guarded playlist NODE"
    );
    let _ = writeln!(
        output,
        "  rekordbox delete-track --xml <in> --output <new> --track-id <id> --expected-memberships <index:IDs;...>   Delete a track and all guarded playlist memberships"
    );
    let _ = writeln!(
        output,
        "  rekordbox append-playlist-member --xml <in> --output <new> --playlist-index <n> --expected-order <IDs> --track-id <id>   Append one existing track to one guarded playlist"
    );
    let _ = writeln!(
        output,
        "  rekordbox clear-playlist --xml <in> --output <new> --playlist-index <n> --expected-order <IDs>   Remove all guarded memberships from one playlist"
    );
    let _ = writeln!(
        output,
        "  rekordbox insert-playlist-member --xml <in> --output <new> --playlist-index <n> --expected-order <IDs> --track-id <id> --position <n>   Insert one existing track at a guarded zero-based position"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-name --xml <in> --output <new> --track-id <id> --expected-name <old> --name <new>   Export one guarded track Name edit\n  rekordbox edit-track-date-added --xml <in> --output <new> --track-id <id> --expected-date-added <old> --date-added <new>   Export one guarded raw DateAdded edit\n  rekordbox edit-track-date-modified --xml <in> --output <new> --track-id <id> --expected-date-modified <old> --date-modified <new>   Export one guarded raw DateModified edit\n  rekordbox edit-track-year --xml <in> --output <new> --track-id <id> --expected-year <old> --year <new>   Export one guarded raw track Year edit\n  rekordbox edit-track-kind --xml <in> --output <new> --track-id <id> --expected-kind <old> --kind <new>   Export one guarded raw track Kind edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-remixer --xml <in> --output <new> --track-id <id> --expected-remixer <old> --remixer <new>   Export one guarded track Remixer edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-mix --xml <in> --output <new> --track-id <id> --expected-mix <old> --mix <new>   Export one guarded track Mix edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-grouping --xml <in> --output <new> --track-id <id> --expected-grouping <old> --grouping <new>   Export one guarded track Grouping edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-rating --xml <in> --output <new> --track-id <id> --expected-rating <old> --rating <new>   Export one guarded raw Rating edit\n  rekordbox edit-track-colour --xml <in> --output <new> --track-id <id> --expected-colour <old> --colour <new>   Export one guarded raw Colour edit\n  rekordbox edit-track-average-bpm --xml <in> --output <new> --track-id <id> --expected-average-bpm <old> --average-bpm <new>   Export one guarded raw AverageBpm edit\n  rekordbox edit-track-disc-number --xml <in> --output <new> --track-id <id> --expected-disc-number <old> --disc-number <new>   Export one guarded raw DiscNumber edit\n  rekordbox edit-track-number --xml <in> --output <new> --track-id <id> --expected-track-number <old> --track-number <new>   Export one guarded raw TrackNumber edit\n  rekordbox edit-track-size --xml <in> --output <new> --track-id <id> --expected-size <old> --size <new>   Export one guarded raw Size edit\n  rekordbox edit-track-total-time --xml <in> --output <new> --track-id <id> --expected-total-time <old> --total-time <new>   Export one guarded raw TotalTime edit\n  rekordbox edit-track-bit-rate --xml <in> --output <new> --track-id <id> --expected-bit-rate <old> --bit-rate <new>   Export one guarded raw BitRate edit\n  rekordbox edit-track-sample-rate --xml <in> --output <new> --track-id <id> --expected-sample-rate <old> --sample-rate <new>   Export one guarded raw SampleRate edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-composer --xml <in> --output <new> --track-id <id> --expected-composer <old> --composer <new>   Export one guarded track Composer edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-tonality --xml <in> --output <new> --track-id <id> --expected-tonality <old> --tonality <new>   Export one guarded track Tonality edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-album --xml <in> --output <new> --track-id <id> --expected-album <old> --album <new>   Export one guarded track Album edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-genre --xml <in> --output <new> --track-id <id> --expected-genre <old> --genre <new>   Export one guarded track Genre edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-label --xml <in> --output <new> --track-id <id> --expected-label <old> --label <new>   Export one guarded track Label edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-comments --xml <in> --output <new> --track-id <id> --expected-comments <old> --comments <new>   Export one guarded track Comments edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-track-artist --xml <in> --output <new> --track-id <id> --expected-artist <old> --artist <new>   Export one guarded track Artist edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox delete-mark --xml <in> --output <new> --track-id <id> --mark-index <n> --type <raw> --start <raw> --name <raw> (--end <raw>|--no-end)   Delete one guarded position mark"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-mark-end --xml <in> --output <new> --track-id <id> --mark-index <n> --expected-end <old> --end <new>   Export one guarded End edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-position --xml <in> --output <new> --track-id <id> --mark-index <n> --expected-start <old> --start <new>   Export one position edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-tempo --xml <in> --output <new> --track-id <id> --tempo-index <n> --expected-bpm <old> --bpm <new>   Export one BPM edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-tempo-start --xml <in> --output <new> --track-id <id> --tempo-index <n> --expected-start <old> --start <new>   Export one tempo-point position edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-tempo-beat --xml <in> --output <new> --track-id <id> --tempo-index <n> --expected-beat <old> --beat <new>   Export one guarded Battito edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox edit-tempo-meter --xml <in> --output <new> --track-id <id> --tempo-index <n> --expected-meter <old> --meter <new>   Export one guarded raw Metro edit"
    );
    let _ = writeln!(
        output,
        "  rekordbox export --xml <input> --output <new-path>   Export bounded no-edit XML subset"
    );
    let _ = writeln!(
        output,
        "  track-id <n>   Construct and echo a domain TrackId (n >= 1)"
    );
    let _ = writeln!(
        output,
        "  catalog init --db <path>   Initialize local catalog"
    );
    let _ = writeln!(
        output,
        "  catalog list --db <path>   List local catalog tracks"
    );
    let _ = writeln!(
        output,
        "  catalog import --dir <directory> --db <path>   Import WAV/FLAC/MP3 references recursively"
    );
    let _ = writeln!(
        output,
        "  catalog analyze-waveform --db <path> [--track-id <id>]   Compute and store waveform peaks (idempotent; sources only read)"
    );
    let _ = writeln!(
        output,
        "  catalog analyze-tempo --db <path> [--track-id <id>] [--bpm-range <lo-hi>]   Compute and store tempo/beat grids (default 70-180; idempotent; sources only read)"
    );
    let _ = writeln!(
        output,
        "  catalog backup --db <source> --to <new-backup>   Create verified backup"
    );
    let _ = writeln!(
        output,
        "  catalog restore --from <backup> --db <new-destination>   Restore backup"
    );
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let code = u8::try_from(run(&args)).unwrap_or(1);
    std::process::ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(rest: &[&str]) -> Vec<String> {
        let mut v = vec!["oxidj-cli".to_string()];
        v.extend(rest.iter().map(|s| s.to_string()));
        v
    }

    #[test]
    fn no_command_is_a_usage_error() {
        assert_ne!(run(&argv(&[])), OK);
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        assert_ne!(run(&argv(&["bogus"])), OK);
    }

    #[test]
    fn zero_track_id_is_a_domain_error() {
        assert_eq!(run(&argv(&["track-id", "0"])), DOMAIN);
    }

    #[test]
    fn one_track_id_succeeds() {
        assert_eq!(run(&argv(&["track-id", "1"])), OK);
    }
}
