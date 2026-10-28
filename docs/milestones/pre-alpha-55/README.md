# F-07 Battito no-edit byte retention (synthetic evidence)

Two synthetic XML cases exercise CLI preview and no-edit export. Exact `Metro="4/4" Battito="01"` is accepted and shown without losing its leading zero; exact non-4/4 `Metro="7/8" Battito="-2147483648"` is accepted at the signed-i32 minimum. In each case the output file is byte-identical to input and the source remains unchanged.

The meter string is preserved exactly. This evidence does not infer any non-4/4 beat-position semantics and does not demonstrate vendor compatibility. No normalization, tuple reconciliation, beat-grid conversion, private data, or vendor qualification is included. Global F-09 and ADR-005 qualification remain NOT DEMONSTRATED.

`card-1.png` and `card-2.png` are synthetic-only evidence cards, not Rekordbox application captures.