# Dependency layer policy and the `cargo xtask layers` gate

M0-08. This is the **durable deliverable** of the crate-boundary decision
(ADR-004): a gate that fails on a forbidden dependency edge, and stays meaningful
as crates are added later.

## The layer model

Lower layer number = closer to the domain core. A crate may depend **only** on
crates at the **same layer or lower**. Depending on a higher layer is a forbidden
(upward) edge.

| Layer | Crate | Role |
|-------|-------|------|
| 0 | `oxidj-core` | Neutral domain model and rules; zero infrastructure deps. |
| 1 | `oxidj-library` | Shared catalog/application handlers; owns SQLite and migrations. |
| 1 | `oxidj-analysis` | Offline analysis (W1: streaming 3-band waveform peaks; B2: tempo/beat grid) over decoded mono samples. Depends on `oxidj-core` and the admitted, exactly pinned `taktus =0.1.0` **only** (ADR-004 B2 amendment): the gate rejects every other internal or external dependency of any kind, so it can neither reach the library/adapters nor add other DSP crates. |
| 1 | `oxidj-audio` | Realtime deck engine (M8/D2, ADR-008): CPAL device output, lock-free command ring, in-RAM decks. **No internal dependency**; only the admitted, exactly pinned `cpal =0.18.2` and `rtrb =0.4.1` (normal kind, crates.io, no default features, no features). Only layer 3+ crates (application, presentation) may depend on it; the out-of-workspace UI prototype uses it directly. |
| 2 | `oxidj-rekordbox` | Isolated vendor XML/DeviceSQL adapter (M4 preview). |
| 2 | `oxidj-onelibrary` | Read-only OneLibrary DTO and process-protocol boundary; no SQL dependency. SQLCipher lives only in `adapters/onelibrary-reader`, an independent Cargo workspace excluded from the root graph. `cargo xtask layers` checks the root resolved feature graph excludes SQLCipher and verifies the isolated pinned reader graph and locked license gate. |
| 3 | `oxidj-application` | Orchestrates shared handlers and adapter operations for callers. |
| 4 | `oxidj-cli` | Thin command-line presentation; calls application/library handlers. |
| exempt | `xtask` | Build-time tooling (`cargo xtask`), never a product dep. |

The dependency path for vendor operations is CLI → application handler → isolated
adapter, while catalog operations still use the library handlers; the adapter's
vendor parsing does not enter the CLI, application domain, or library. The Rekordbox
adapter is layer 2, the application orchestration layer is layer 3, and the CLI is
layer 4. Persistence remains owned by the library; the core cannot depend upward.
`oxidj-library` alone may depend on `rusqlite` and `rusqlite_migration`; all
other persistence dependencies remain forbidden. For the bounded M2 importer,
`oxidj-library` may also have normal dependencies on Hound `=3.5.1`, Claxon
`=0.4.3`, and nanomp3 `=0.2.0`, each with `default-features = false` and no
explicit features. These decoders remain forbidden in core and CLI; every other
audio crate remains forbidden in every product crate. `nanomp3-core` is not a
separate direct-dependency exception. The dependencies are not added by this
policy change. `xtask` is outside the product dependency graph.

```mermaid
flowchart TB
    cli["oxidj-cli<br/>layer 4"] --> app["oxidj-application<br/>layer 3"]
    cli --> library["oxidj-library<br/>layer 1"]
    app --> rekordbox
    app --> library
    app --> onelibrary["oxidj-onelibrary<br/>layer 2; DTO/protocol"]
    onelibrary --> reader["independent SQLCipher reader workspace<br/>layer 2 product process"]
    rekordbox --> xml["quick-xml"]
    rekordbox --> pdb["rekordbox-pdb (pinned read-only use)"]
    library --> core["oxidj-core<br/>layer 0"]
    analysis["oxidj-analysis<br/>layer 1; core + taktus"] --> core
    engine["oxidj-audio<br/>layer 1; realtime engine"] --> cpal["cpal =0.18.2 + rtrb =0.4.1"]
    library --> sqlite["rusqlite + rusqlite_migration"]
    core -. "forbidden upward edge" .-> library
    xtask["xtask<br/>exempt build tooling"]
```

The bounded M4 XML preview keeps XML parsing in `oxidj-rekordbox` at layer 2; the
application handler invokes that adapter and exposes it to the CLI without making
the parser a CLI or library dependency.

The M7 library import/export slice adds one downward edge, `oxidj-application`
(layer 3) → `oxidj-library` (layer 1): the application handler maps the adapter's
PDB preview into the library's vendor-neutral `LibrarySnapshot` and maps a stored
snapshot into the adapter's XML-writer DTO. The library never depends on an adapter
and no adapter depends on the library; the existing gate already permits this
downward edge.

The separate DeviceSQL `export.pdb`
preview uses the pinned `rekordbox-pdb` git revision only from this adapter. Its
read-only handler exposes source-scoped IDs and paths for display, never maps them
to OxiDJ catalog IDs, and does not expose the upstream writer API. This does not
claim OneLibrary support or hardware compatibility. Other vendor adapters, audio
analysis and UI remain outside this slice.

## What the gate enforces

1. **Internal edges point down or across, never up.** A path dependency to a
   workspace member whose layer is *higher* fails. (`cli` → `core` is allowed;
   `core` → `cli` is forbidden.)
2. **Infrastructure stays behind its owner.** Core and CLI deny audio, UI,
   persistence and vendor crates. Library allows only `rusqlite` and
   `rusqlite_migration` from persistence and the three named M2 decoders from
   audio; it denies all other audio, UI, other persistence and vendor crates.
   `oxidj-onelibrary` has no SQL dependency; only the independent reader workspace
   may use exactly `rusqlite =0.40.2` with defaults off and features
   `bundled-sqlcipher-vendored-openssl,hooks`. Main-workspace feature resolution
   must remain SQLCipher-free. Both workspaces use committed locks and their own
   license checks. These checks include normal, dev and build dependencies.
3. **The realtime engine stays isolated (ADR-004 D2 amendment).** `oxidj-audio`
   may declare only `cpal =0.18.2` and `rtrb =0.4.1` (exact, crates.io, normal
   kind, defaults and features off) — no internal crate, no other external
   crate, no dev/build dependency. A path edge to `oxidj-audio` from a crate
   below layer 3 (core, library, analysis, adapters) fails. `cpal` stays on the
   audio denylist for every other crate.
4. **Every new product crate must be classified.** A workspace member with no
   layer assignment fails until it is added to the layer table in
   `xtask/src/main.rs`. This is what keeps the gate meaningful as the crate set
   grows.

The gate reads `cargo metadata --no-deps` (offline; it does not resolve or fetch
dependencies), so it can detect a violation even when the offending crate is
declared but not yet available — which is exactly how the planted-violation demo
below works.

## Planting and removing a forbidden edge (the required demonstration)

The gate is shown **failing** on a deliberately introduced violation, and
**passing** once it is removed — not merely passing on a clean tree.

```sh
# 1. clean tree: passes
cargo xtask layers

# 2. plant a forbidden edge: an audio crate inside oxidj-core
#    (edit crates/oxidj-core/Cargo.toml -> [dependencies] -> cpal = "0.1")
#    Run the pre-built gate binary directly so Cargo does not re-resolve the
#    (intentionally unresolvable) planted dependency into Cargo.lock:
./target/debug/xtask layers        # FAILS: forbidden audio dependency 'cpal'

# 3. remove the planted line, then pass again (both invocations agree)
cargo xtask layers                 # PASS
```

The `oxidj-analysis` core-only rule is demonstrated the same way: adding
`oxidj-library = { workspace = true }` (a same-layer, otherwise permitted edge)
to `crates/oxidj-analysis/Cargo.toml` makes `cargo xtask layers` exit 1 with
`oxidj-analysis (layer 1) -> oxidj-library: oxidj-analysis may depend only on
oxidj-core and the admitted [taktus =0.1.0] (ADR-004)`; removing it passes
again. Unit test `analysis_may_depend_only_on_core` pins upward, same-layer and
external plants (including a path/git taktus and a second DSP crate next to
taktus), and `analysis_taktus_must_be_exactly_pinned_normal_without_features`
rejects a re-pinned, default-features, dev or featured taktus.

The direct-binary invocation is used only to isolate the *policy* failure from a
stale-lockfile side effect; the failure message is produced by the same
`cargo xtask layers` logic the canonical `cargo xtask ci` runs.

## When a crate is added (M1+)

1. Add the crate under `crates/`, a member of the workspace, with a one-way edge
   toward a lower-or-equal layer.
2. Assign it a layer in `layer_of` in `xtask/src/main.rs`.
3. Run `cargo xtask layers` and commit only when it passes.

The gate is the single source of truth for the boundary. Do not weaken the
denylists or the upward-edge rule to make a build pass; that defeats the ADR-004
review's whole point.
