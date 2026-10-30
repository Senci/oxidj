# Pre-alpha 56 — OxiDJ theme screenshots

These are genuine screenshots captured from the committed OxiDJ Local Library UI on the isolated Hermes Bot Screen. The app used an empty scratch SQLite catalog; no personal library or media was used. Each PNG is 800×600 and was visually inspected.

- [`tokyo-night.png`](tokyo-night.png) — Tokyo Night selected; empty library.
- [`tokyo-day.png`](tokyo-day.png) — Tokyo Day selected; empty library.
- [`custom-edited.png`](custom-edited.png) — Custom palette editor open after editing Primary to `#D3869B` and Secondary to `#8EC07C`.
- [`custom-restored.png`](custom-restored.png) — captured after closing and relaunching the UI against the same scratch catalog/config, then reopening the Custom editor. The visible values are `#D3869B` and `#8EC07C`.

The app-written configuration after relaunch identified the selected theme as `custom` and stored those two values in the Primary and Secondary token slots (`d3869b`, `8ec07c`). The restored capture independently shows the reopened editor and matching values. This archive records a provisional local UI state only; it is not production GUI, accessibility/platform, performance, or framework qualification evidence. See the M3 provisional UI boundary in [`plan/MVP.md`](../../../plan/MVP.md).
