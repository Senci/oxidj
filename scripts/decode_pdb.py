#!/usr/bin/env python3
"""Independent DeviceSQL (export.pdb) decode baseline for OxiDJ M7 proof.

This is a *test-only* independent decoder. It shares no code with the Rust
`rekordbox-pdb` crate or the `oxidj-rekordbox` adapter: it reads the pinned
DeviceSQL page/row layout directly (Kaitai `rekordbox_pdb.ksy` reference,
Deep-Symmetry crate-digger, EPL/MPL/LGPL) and emits a JSON baseline of
aggregate counts plus ordered track/playlist structure.

Usage:
    python3 scripts/decode_pdb.py <export.pdb> [--json OUT]

The baseline is aggregate-only by default (counts and booleans). With
`--with-titles` it also emits per-track titles and per-playlist ordered
member TrackIDs, which is personal metadata: keep it local, never commit it.

Exit codes: 0 = decoded; 1 = malformed/unsupported input; 2 = usage error.
"""
from __future__ import annotations

import argparse
import json
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

PAGE_HEADER_SIZE = 0x28
ROW_GROUP_SIZE = 0x24
MAX_STRING_BYTES = 1 << 20


class PdbError(Exception):
    """Malformed or unsupported DeviceSQL input."""


@dataclass
class Table:
    table_type: int
    empty_candidate: int
    first_page: int
    last_page: int


@dataclass
class Track:
    id: int
    title: str
    artist_id: int
    album_id: int
    genre_id: int
    label_id: int
    key_id: int
    tempo: int
    duration: int
    file_path: str
    analyze_path: str
    date_added: str
    comment: str
    year: int
    bitrate: int
    sample_rate: int
    file_size: int
    track_number: int
    disc_number: int
    rating: int


@dataclass
class PlaylistNode:
    id: int
    parent_id: int
    sort_order: int
    is_folder: bool
    name: str


@dataclass
class PlaylistEntry:
    entry_index: int
    track_id: int
    playlist_id: int


@dataclass
class NamedRow:
    id: int
    name: str


@dataclass
class Database:
    page_size: int
    tables: list[Table]
    tracks: list[Track] = field(default_factory=list)
    artists: list[NamedRow] = field(default_factory=list)
    albums: list[NamedRow] = field(default_factory=list)
    genres: list[NamedRow] = field(default_factory=list)
    labels: list[NamedRow] = field(default_factory=list)
    keys: list[NamedRow] = field(default_factory=list)
    playlist_tree: list[PlaylistNode] = field(default_factory=list)
    playlist_entries: list[PlaylistEntry] = field(default_factory=list)


def u16le(data: bytes, off: int) -> int:
    return struct.unpack_from("<H", data, off)[0]


def u32le(data: bytes, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


def decode_string(data: bytes, off: int) -> str:
    if off >= len(data):
        raise PdbError(f"string offset {off} outside source")
    b0 = data[off]
    if b0 & 1:
        length = (b0 >> 1) - 1
        if length < 0 or off + 1 + length > len(data):
            raise PdbError(f"short string at {off} overruns source")
        return data[off + 1 : off + 1 + length].decode("utf-8", "replace")
    if off + 4 > len(data):
        raise PdbError(f"long string at {off} overruns source")
    total = u16le(data, off + 1)
    payload = data[off + 4 : off + total]
    if b0 == 0x40:
        return payload.decode("utf-8", "replace")
    if b0 == 0x90:
        return payload.decode("utf-16-le", "replace")
    raise PdbError(f"unknown string kind 0x{b0:02x} at {off}")


def row_offsets(page: bytes, page_size: int) -> list[int]:
    if page[0x1B] & 0x40:
        return []
    num_rows = page[0x18] + 0x100 * (page[0x19] & 1)
    if num_rows == 0:
        return []
    offsets: list[int] = []
    num_groups = (num_rows - 1) // 16 + 1
    for group in range(num_groups):
        base = page_size - group * ROW_GROUP_SIZE
        present = u16le(page, base - 4)
        in_group = 16 if group < num_groups - 1 else (num_rows - 1) % 16 + 1
        for i in range(in_group):
            if (present >> i) & 1:
                offsets.append(PAGE_HEADER_SIZE + u16le(page, base - 6 - 2 * i))
    return offsets


def table_row_offsets(data: bytes, page_size: int, table: Table) -> list[int]:
    locs: list[int] = []
    page_index = table.first_page
    visited: set[int] = set()
    while True:
        if page_index in visited:
            raise PdbError("cyclic table page chain")
        visited.add(page_index)
        page_off = page_index * page_size
        if page_off + page_size > len(data):
            raise PdbError(f"page {page_index} outside source")
        page = data[page_off : page_off + page_size]
        for ro in row_offsets(page, page_size):
            locs.append(page_off + ro)
        if page_index == table.last_page:
            break
        page_index = u32le(page, 12)
    return locs


def parse_track(data: bytes, row: int) -> Track:
    strings: list[str] = []
    for i in range(21):
        o = row + u16le(data, row + 0x5E + 2 * i)
        strings.append(decode_string(data, o))
    return Track(
        id=u32le(data, row + 0x48),
        title=strings[17],
        artist_id=u32le(data, row + 0x44),
        album_id=u32le(data, row + 0x40),
        genre_id=u32le(data, row + 0x3C),
        label_id=u32le(data, row + 0x28),
        key_id=u32le(data, row + 0x20),
        tempo=u32le(data, row + 0x38),
        duration=u16le(data, row + 0x54),
        file_path=strings[20],
        analyze_path=strings[14],
        date_added=strings[10],
        comment=strings[16],
        year=u16le(data, row + 0x50),
        bitrate=u32le(data, row + 0x30),
        sample_rate=u32le(data, row + 0x08),
        file_size=u32le(data, row + 0x10),
        track_number=u32le(data, row + 0x34),
        disc_number=u16le(data, row + 0x4C),
        rating=data[row + 0x59],
    )


def parse_named_row(data: bytes, row: int, id_off: int, name_off: int) -> NamedRow:
    return NamedRow(id=u32le(data, row + id_off), name=decode_string(data, row + name_off))


def parse_subtyped_named_row(
    data: bytes, row: int, id_off: int, near_off: int, far_off: int
) -> NamedRow:
    """Album/artist-style row: name offset is one byte (`near_off`) unless the
    subtype's 0x04 bit selects a two-byte far offset at `far_off`."""
    subtype = u16le(data, row)
    if subtype & 0x04:
        name_at = u16le(data, row + far_off)
    else:
        name_at = data[row + near_off]
    return NamedRow(id=u32le(data, row + id_off), name=decode_string(data, row + name_at))


def parse_playlist_tree_row(data: bytes, row: int) -> PlaylistNode:
    return PlaylistNode(
        id=u32le(data, row + 0x0C),
        parent_id=u32le(data, row + 0x00),
        sort_order=u32le(data, row + 0x08),
        is_folder=u32le(data, row + 0x10) != 0,
        name=decode_string(data, row + 0x14),
    )


def parse_playlist_entry_row(data: bytes, row: int) -> PlaylistEntry:
    return PlaylistEntry(
        entry_index=u32le(data, row + 0x00),
        track_id=u32le(data, row + 0x04),
        playlist_id=u32le(data, row + 0x08),
    )


def parse_pdb(data: bytes) -> Database:
    if len(data) < 0x2C or len(data) % 4096:
        raise PdbError("truncated or unaligned DeviceSQL source")
    page_size = u32le(data, 4)
    num_tables = u32le(data, 8)
    if page_size != 4096 or num_tables > 20 or 0x1C + num_tables * 16 > len(data):
        raise PdbError("unsupported DeviceSQL header")
    tables: list[Table] = []
    for i in range(num_tables):
        o = 0x1C + i * 16
        tables.append(
            Table(
                table_type=u32le(data, o),
                empty_candidate=u32le(data, o + 4),
                first_page=u32le(data, o + 8),
                last_page=u32le(data, o + 12),
            )
        )
    by_type = {t.table_type: t for t in tables}
    db = Database(page_size=page_size, tables=tables)

    def rows_of(table_type: int) -> list[int]:
        table = by_type.get(table_type)
        if table is None:
            return []
        return table_row_offsets(data, page_size, table)

    for o in rows_of(0):
        db.tracks.append(parse_track(data, o))
    for o in rows_of(2):
        # Artist rows: subtype at 0, id at 4, name offset at 9 (near) or 0x0a (far).
        db.artists.append(parse_subtyped_named_row(data, o, 4, 9, 0x0A))
    for o in rows_of(3):
        # Album rows: subtype at 0, id at 0x0c, name offset at 0x11 (near) or 0x12 (far).
        db.albums.append(parse_subtyped_named_row(data, o, 0x0C, 0x11, 0x12))
    for o in rows_of(1):
        db.genres.append(parse_named_row(data, o, 0, 4))
    for o in rows_of(4):
        db.labels.append(parse_named_row(data, o, 0, 4))
    for o in rows_of(5):
        # Key rows: id at 0, id2 at 4, name at 8.
        db.keys.append(parse_named_row(data, o, 0, 8))
    for o in rows_of(7):
        db.playlist_tree.append(parse_playlist_tree_row(data, o))
    for o in rows_of(8):
        db.playlist_entries.append(parse_playlist_entry_row(data, o))
    return db


def named_map(rows: list[NamedRow]) -> dict[int, Optional[str]]:
    """id -> name, or None when the id is duplicated (ambiguous)."""
    out: dict[int, Optional[str]] = {}
    for row in rows:
        if row.id in out:
            out[row.id] = None
        else:
            out[row.id] = row.name
    return out


def build_baseline(db: Database, with_titles: bool = False) -> dict:
    artists = named_map(db.artists)
    albums = named_map(db.albums)
    genres = named_map(db.genres)
    labels = named_map(db.labels)
    keys = named_map(db.keys)

    def resolve(mapping: dict[int, Optional[str]], track_id: int) -> Optional[str]:
        if track_id == 0:
            return None
        if track_id not in mapping or mapping[track_id] is None:
            return None
        name = mapping[track_id]
        return name if name else None

    track_rows = []
    unresolved = 0
    for t in db.tracks:
        fields = {
            "Artist": resolve(artists, t.artist_id),
            "Album": resolve(albums, t.album_id),
            "Genre": resolve(genres, t.genre_id),
            "Label": resolve(labels, t.label_id),
            "Tonality": resolve(keys, t.key_id),
        }
        # A lookup id with no/ambiguous row, or a non-ISO date, is unresolved.
        if any(
            fields[name] is None
            and getattr(t, f"{name.lower()}_id" if name != "Tonality" else "key_id") != 0
            for name in fields
        ):
            unresolved += 1
        if t.date_added and not _is_iso_date(t.date_added):
            unresolved += 1
        if t.rating not in (0, 1, 2, 3, 4, 5):
            unresolved += 1
        row = {
            "id": t.id,
            "tempo_hundredths_bpm": t.tempo,
            "duration_seconds": t.duration,
            "file_path": t.file_path,
            "analyze_path": t.analyze_path,
            "date_added": t.date_added,
            "comment": t.comment,
            "year": t.year,
            "bitrate": t.bitrate,
            "sample_rate": t.sample_rate,
            "file_size": t.file_size,
            "track_number": t.track_number,
            "disc_number": t.disc_number,
            "rating": t.rating,
            "resolved": {k: v for k, v in fields.items()},
        }
        if with_titles:
            row["title"] = t.title
        track_rows.append(row)

    # Playlist tree: folder/playlist nodes in sibling order per parent.
    nodes_by_parent: dict[int, list[PlaylistNode]] = {}
    for node in db.playlist_tree:
        nodes_by_parent.setdefault(node.parent_id, []).append(node)
    for siblings in nodes_by_parent.values():
        siblings.sort(key=lambda n: (n.sort_order, n.id))

    entries_by_playlist: dict[int, list[PlaylistEntry]] = {}
    for entry in db.playlist_entries:
        entries_by_playlist.setdefault(entry.playlist_id, []).append(entry)
    for members in entries_by_playlist.values():
        members.sort(key=lambda e: e.entry_index)

    playlist_rows = []
    memberships = 0
    for node in sorted(
        (n for n in db.playlist_tree if not n.is_folder), key=lambda n: n.id
    ):
        members = entries_by_playlist.get(node.id, [])
        memberships += len(members)
        row = {
            "id": node.id,
            "parent_id": node.parent_id,
            "sort_order": node.sort_order,
            "name": node.name,
            "member_track_ids": [e.track_id for e in members],
        }
        playlist_rows.append(row)

    folder_rows = [
        {
            "id": n.id,
            "parent_id": n.parent_id,
            "sort_order": n.sort_order,
            "name": n.name,
        }
        for n in sorted(
            (n for n in db.playlist_tree if n.is_folder), key=lambda n: n.id
        )
    ]

    return {
        "schema": "oxidj-m7-pdb-baseline-v1",
        "page_size": db.page_size,
        "table_count": len(db.tables),
        "counts": {
            "tracks": len(db.tracks),
            "folders": len(folder_rows),
            "playlists": len(playlist_rows),
            "memberships": memberships,
            "artists": len(db.artists),
            "albums": len(db.albums),
            "genres": len(db.genres),
            "labels": len(db.labels),
            "keys": len(db.keys),
        },
        "unresolved_fields": unresolved,
        "tracks": track_rows,
        "folders": folder_rows,
        "playlists": playlist_rows,
    }


def _is_iso_date(value: str) -> bool:
    if len(value) != 10 or value[4] != "-" or value[7] != "-":
        return False
    if not value[0:4].isdigit() or not value[5:7].isdigit() or not value[8:10].isdigit():
        return False
    month = int(value[5:7])
    day = int(value[8:10])
    return 1 <= month <= 12 and 1 <= day <= 31


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdb", type=Path)
    parser.add_argument("--json", type=Path, help="write the baseline JSON here")
    parser.add_argument(
        "--with-titles",
        action="store_true",
        help="include per-track titles and per-playlist names (personal metadata; keep local)",
    )
    args = parser.parse_args(argv)
    try:
        data = args.pdb.read_bytes()
    except OSError as error:
        print(f"error: could not read {args.pdb}: {error}", file=sys.stderr)
        return 1
    try:
        db = parse_pdb(data)
    except PdbError as error:
        print(f"error: {args.pdb} rejected: {error}", file=sys.stderr)
        return 1
    baseline = build_baseline(db, with_titles=args.with_titles)
    text = json.dumps(baseline, indent=2, sort_keys=False)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
