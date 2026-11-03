# Rekordbox F-07 Battito validation (synthetic evidence)

The official Rekordbox XML format list declares `TEMPO.Battito` as a signed 32-bit integer. OxiDJ parses its decimal text as `i32`; empty text, non-integer text, and values outside `-2147483648..=2147483647` are rejected. For the exact raw meter string `Metro="4/4"`, the stated beat position range is `1..=4` inclusive. Other meter strings receive only signed-i32 validation; OxiDJ does not infer their beat-position semantics or normalize meter text.

Guarded Battito edits apply the same rule using the selected tempo point's exact raw meter. The edit remains guarded by the expected old value and creates output separately; it does not mutate input bytes. No tempo/grid recalculation or tuple reconciliation is performed.

The two cards in this directory document synthetic-only parser/edit behavior. They are not Rekordbox application captures or vendor qualification. Private library data, F-10 tuple reconciliation, meter normalization, beat-grid recalculation, and vendor qualification are out of scope. Global F-09 and ADR-005 qualification remain NOT DEMONSTRATED.

Source: [Rekordbox XML format list PDF](https://cdn.rekordbox.com/files/20200410160904/xml_format_list.pdf), M4 F-07.
