# Pre-alpha 44 — playlist NODE raw-byte retention

Synthetic F-02 no-edit export verifies byte preservation for accepted raw
numeric playlist `Entries` values `03` and `0`, while required `KeyType="0"`
bytes remain unchanged. Export rejects non-numeric or structurally mismatched
Entries, missing/empty Entries, and other KeyType values. No private library
data was used. This does not establish Rekordbox or
OneLibrary compatibility, global F-09, or ADR-005 qualification.

Screenshots: `01-node-attribute-retention.png` and `02-scope-and-proof.png`.
