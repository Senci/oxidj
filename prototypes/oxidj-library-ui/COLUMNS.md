# Persistent track-table columns

Core column-layout implementation for task `t_764eab32`; independent review and
integration are separate. This branch must not install or modify local master.

## Controls and persistence

- **Columns** opens a keyboard-focusable dropdown in the existing header band,
  pinned outside the horizontal scroller without consuming another table row. Right-click any column header
  opens the same choices. At least one column must remain visible.
- Left-drag a header onto another header to reorder. Drag its right-hand 5-point
  boundary to resize (24–2,000 logical points). Body/edge hit regions are
  disjoint, and a completed reorder does not sort. Releasing outside the header
  cancels the reorder.
- Middle-click a header to auto-fit that column. The dropdown/context menu has
  **Auto-fit columns** and **Reset columns**, without requiring a middle/right
  mouse button. Fitting measures real displayed data only when requested.
- Catalog and imported playlists have independent visibility/order/width
  settings in eframe's existing `ui_settings` storage. Missing or invalid layout
  records fall back per table; unrelated old preferences remain intact.
- Horizontal scrolling handles narrow windows; headers and virtualized rows
  share the same column geometry. Selection and deck loading remain row actions.
  Catalog headers and imported-playlist headers support ascending/descending sorting.

## Metadata audit / honest scope

The scanned catalog path is `Catalog::list_track_details` / `TrackDetails`:
reference ID, filesystem path, optional duration/sample-rate/channels/bit-depth,
and availability. It does not contain imported artist/album/genre/key/BPM tags.
The catalog choices therefore remain Availability, ID, Filename, Folder,
Duration and Format; no synthetic tag values or implicit per-row metadata queries
are added.

Imported-playlist browsing uses `LibraryNavigationTrack` from
`Catalog::library_navigation`: title/location, optional artist/album/key,
integer BPM hundredths, genre and duration seconds. Its available display choices are
Title, Artist, Album, BPM, Key and Duration. Source membership positions and
repeated members retain their identity. Absent optionals, zero/unspecified BPM
and zero/unspecified imported duration display `—`, not an invented value.
Genre is used by the playlist filter. Scanned-file availability is matched only
against exact catalog location URIs; an unmatched or conflicting reference is
unknown, not missing. This lookup does not import or probe files.
No database schema, vendor parser, audio path or catalog mutation is changed.

## Verification boundary

`src/column_tests.rs` drives production `LibraryApp::ui` over actual egui frames:
both tables' header reorder/resize/menu visibility, disk-backed App::save and
fresh app/context reload, row selection, left-click sorting, right/middle mouse
and keyboard menu alternatives, resize limits, reversed/cancelled reorder,
narrow-window horizontal scrolling and honest missing values. Malformed and
pre-column settings exercise fallback compatibility. Existing UI tests retain
deck loading and virtual scrolling coverage. These synthetic tests are not a
live desktop, screen-reader or presented-FPS qualification.

Imported-playlist quick filters combine inclusive BPM bounds, key and genre
substrings and scanned availability. Missing metadata does not match an active
filter; clearing filters restores the unfiltered list. Invalid BPM input shows a
warning and does not match tracks. Filter values are session-only.
Meme treatments remain unapproved proposals. No installation or push is claimed.
