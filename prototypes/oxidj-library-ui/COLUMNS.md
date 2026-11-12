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
  Existing catalog ID/Filename/Duration left-click sorting is preserved.

## Metadata audit / honest scope

The scanned catalog path is `Catalog::list_track_details` / `TrackDetails`:
reference ID, filesystem path, optional duration/sample-rate/channels/bit-depth,
and availability. It does not contain imported artist/album/genre/key/BPM tags.
The catalog choices therefore remain Availability, ID, Filename, Folder,
Duration and Format; no synthetic tag values or implicit per-row metadata queries
are added.

Imported-playlist browsing uses `LibraryNavigationTrack` from
`Catalog::library_navigation`: title/location, optional artist/album/key,
integer BPM hundredths and duration seconds. Its available display choices are
Title, Artist, Album, BPM, Key and Duration. Source membership positions and
repeated members retain their identity. Absent optionals, zero/unspecified BPM
and zero/unspecified imported duration display `—`, not an invented value.
Genre and scanned-file availability are not present in this lightweight DTO.
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

Owner-approved enhanced sorting and BPM/key/genre/availability quick filters are
additional scope recorded on the same card, not delivered by the core column
layout change. Existing search, availability filter and catalog sorting remain;
new filter controls or new metadata DTO fields need their own bounded follow-on.
Meme treatments remain unapproved proposals. No installation or push is claimed.
