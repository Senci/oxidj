#!/usr/bin/env python3
"""Synthetic M7 oracle; invoked by the M7 Cargo integration test.

Compare an actual CLI XML export to independently decoded DeviceSQL fields and
ordered nodes/members, with exactly the first N documented M7 edits applied.
ANLZ expectations below describe the synthetic generator only, not private data.
Reproduce: cargo test -p oxidj-cli --test library_import_export m7_proof_import_manage_export_large_library -- --exact
"""
import argparse
import copy
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import quote

from decode_pdb import build_baseline, parse_pdb


def expected(baseline, step, remove_member=None, create_playlist=None, append_member=None, move_member=None, rename_playlist=None):
    tracks = []
    children = []
    for t in baseline["tracks"]:
        attrs = {
            "TrackID": str(t["id"]), "Name": t["title"],
            "Size": str(t["file_size"]), "TotalTime": str(t["duration_seconds"]),
            "DiscNumber": str(t["disc_number"]), "TrackNumber": str(t["track_number"]),
            "Year": str(t["year"]),
            "AverageBpm": f'{t["tempo_hundredths_bpm"] // 100}.{t["tempo_hundredths_bpm"] % 100:02}',
            "DateAdded": t["date_added"], "BitRate": str(t["bitrate"]),
            "SampleRate": str(t["sample_rate"]), "Comments": t["comment"],
            "Rating": str(t["rating"] * 51),
            "Location": "file://localhost" + quote(t["file_path"], safe="/"),
        }
        attrs.update({k: v for k, v in t["resolved"].items() if v is not None})
        tracks.append(attrs)
        # Fixture ANLZ contains two beats and one unnamed hot cue A.
        children.append([
            ("TEMPO", {"Inizio": "0.000", "Bpm": "120.00", "Metro": "4/4", "Battito": "1"}),
            ("TEMPO", {"Inizio": "0.500", "Bpm": "120.00", "Metro": "4/4", "Battito": "2"}),
            ("POSITION_MARK", {"Name": "", "Type": "0", "Start": "1.500", "Num": "0"}),
        ] if t["analyze_path"] else [])

    playlists = copy.deepcopy(baseline["playlists"])
    by_id = {p["id"]: p for p in playlists}
    if step >= 1:
        by_id[100]["member_track_ids"].insert(0, 1330)
    if step >= 2:
        by_id[101]["member_track_ids"].append(1330)
    if step >= 3:
        by_id[102]["member_track_ids"].clear()
    if step >= 4:
        by_id[103]["member_track_ids"].remove(136)
    if step >= 5:
        by_id[104]["name"] = "Renamed Playlist"
    if step >= 6:
        by_id[118]["parent_id"] = 201
        by_id[118]["sort_order"] = 1

    if remove_member is not None:
        assert step == 0, "selected removal must use the unedited PDB baseline"
        playlist_id, index, track_id = remove_member
        members = by_id[playlist_id]["member_track_ids"]
        assert index >= 0 and members[index] == track_id, "removal expectation differs from PDB"
        del members[index]

    if append_member is not None:
        assert step == 0 and remove_member is None and create_playlist is None
        playlist_id, track_id = append_member
        assert int(track_id) in {t["id"] for t in baseline["tracks"]}
        by_id[playlist_id]["member_track_ids"].append(int(track_id))

    if move_member is not None:
        assert step == 0 and remove_member is None and create_playlist is None and append_member is None
        playlist_id, index, destination, track_id = move_member
        members = by_id[playlist_id]["member_track_ids"]
        assert 0 <= index < len(members) and 0 <= destination < len(members) and abs(index - destination) == 1
        assert members[index] == track_id, "move expectation differs from PDB"
        members[index], members[destination] = members[destination], members[index]

    if rename_playlist is not None:
        assert step == 0 and all(x is None for x in (remove_member, create_playlist, append_member, move_member))
        playlist_id, old_name, new_name = rename_playlist
        playlist = by_id[int(playlist_id)]
        assert playlist["name"] == old_name, "rename expectation differs from PDB"
        playlist["name"] = new_name

    nodes = baseline["folders"] + playlists

    def folder(parent, name):
        result = []
        for n in sorted((n for n in nodes if n["parent_id"] == parent), key=lambda n: (n["sort_order"], n["id"])):
            if "member_track_ids" in n:
                members = [str(i) for i in n["member_track_ids"]]
                result.append(({"Name": n["name"], "Type": "1", "KeyType": "0", "Entries": str(len(members))}, members))
            else:
                result.append(folder(n["id"], n["name"]))
        return ({"Name": name, "Type": "0", "Count": str(len(result))}, result)

    root_attrs, root_children = folder(0, "ROOT")
    if create_playlist is not None:
        name, track_id = create_playlist
        assert step == 0 and remove_member is None
        assert int(track_id) in {t["id"] for t in baseline["tracks"]}
        root_children.append(({"Name": name, "Type": "1", "KeyType": "0", "Entries": "1"}, [track_id]))
        root_attrs["Count"] = str(len(root_children))
    return tracks, children, (root_attrs, root_children)


def verify(pdb, xml, step, remove_member=None, create_playlist=None, append_member=None, move_member=None, rename_playlist=None):
    baseline = build_baseline(parse_pdb(pdb.read_bytes()), with_titles=True)
    assert baseline["counts"]["tracks"] == 1330, "baseline tracks"
    assert baseline["counts"]["playlists"] == 20, "baseline playlists"
    assert baseline["counts"]["memberships"] == 901, "baseline memberships"
    assert baseline["unresolved_fields"] == 0, "baseline unresolved"
    want_tracks, want_children, want_tree = expected(baseline, step, remove_member, create_playlist, append_member, move_member, rename_playlist)
    # Only test-generated synthetic CLI exports are parsed; no untrusted XML intake.
    root = ET.parse(xml).getroot()
    collection = root.find("COLLECTION")
    assert collection is not None, "missing collection"
    assert all(t.tag == "TRACK" for t in collection), "collection shape"
    assert collection.attrib == {"Entries": "1330"}, "collection attributes"
    assert [t.attrib for t in collection] == want_tracks, "tracks: complete attributes differ"
    assert [[(c.tag, c.attrib) for c in t] for t in collection] == want_children, "children: cue/grid attributes differ"

    def tree(node):
        if node.attrib["Type"] == "1":
            assert all(c.tag == "TRACK" and set(c.attrib) == {"Key"} for c in node), "member shape"
            return node.attrib, [c.attrib["Key"] for c in node]
        assert all(c.tag == "NODE" for c in node), "folder shape"
        return node.attrib, [tree(c) for c in node]

    assert tree(root.find("PLAYLISTS/NODE")) == want_tree, "tree: complete ordered nodes/members differ"
    print(f"M7 Python baseline/export step={step}: 1330/20/901 baseline; unresolved=0; complete tree/metadata=ok")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdb", type=Path)
    parser.add_argument("xml", type=Path)
    parser.add_argument("--step", type=int, choices=range(7), default=0)
    parser.add_argument("--remove-member", type=int, nargs=3,
                        metavar=("PLAYLIST_ID", "INDEX", "TRACK_ID"),
                        help="one independently specified occurrence removed from the PDB baseline")
    parser.add_argument("--create-playlist", nargs=2, metavar=("NAME", "TRACK_ID"))
    parser.add_argument("--append-member", type=int, nargs=2, metavar=("PLAYLIST_ID", "TRACK_ID"))
    parser.add_argument("--move-member", type=int, nargs=4, metavar=("PLAYLIST_ID", "INDEX", "DESTINATION", "TRACK_ID"))
    parser.add_argument("--rename-playlist", nargs=3, metavar=("PLAYLIST_ID", "OLD_NAME", "NEW_NAME"))
    args = parser.parse_args()
    try:
        verify(args.pdb, args.xml, args.step, args.remove_member, args.create_playlist, args.append_member, args.move_member, args.rename_playlist)
    except (AssertionError, KeyError, ValueError, OSError, ET.ParseError) as error:
        print(f"M7 mismatch: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
