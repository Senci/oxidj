# Research: Diagram Conventions for OxiDJ Docs

Status: research complete. Owner: `sova` (nous:`poolside/laguna-s-2.1:free`). Scope: shallow, one bounded pass. This document contains no code, performs no installs, and modifies nothing but itself.

Purpose: decide how OxiDJ's Markdown documentation (architecture layers, crate dependency rules, metadata schema, quality-gate flow, worker/dispatch flows) should use Mermaid diagrams, stay readable in both dark and light mode, and embed media with the correct tags.

Two sources define the diagram subjects (read for this task):
- `docs/architecture.md` — crate layering (ADR-004), audio/library/architecture splitting, GUI boundaries.
- `docs/adr/004-crate-boundaries.md` — one-way dependency layers: `oxidj-cli` -> `oxidj-core` -> {`oxidj-library`, `oxidj-storage`, `oxidj-analysis`, `oxidj-audio`, `oxidj-adapters`, `oxidj-ui`} + deferred `oxidj-engine` (M4).
- `docs/research/rust-quality-gates.md` — Tier 1/2/3 gate tiers (fmt, clippy, check, nextest, rustdoc, audit, deny...).

Conventions used below: **VERIFIED** = claim confirmed by an authoritative primary source this run; **INFERRED** = logically derived from a verified fact but not directly confirmed by an authoritative source; **UNVERIFIED** = reported elsewhere but not confirmed by a primary source.

---

## 1. Diagram-type selection

Mermaid's syntax reference states that every diagram begins with a type declaration so the parser picks the right generator, and that all type keywords are listed on one index page. The official descriptions below are quoted/paraphrased from each type's docs page on `mermaid.js.org` (current release line v12; see Versioning note at the bottom of §2).

| Mermaid type | What it is for | Use this when... |
|---|---|---|
| `flowchart` (alias `graph`) | Nodes (geometric shapes) connected by edges (arrows/lines) showing a process, algorithm, or dependency structure. `graph` and `flowchart` are equivalent tokens. | You need to show directional flow or a dependency graph: "X calls Y", "Z depends on core". |
| `sequenceDiagram` | Interaction diagram showing how actors/objects pass messages and in what time-order. | You need to show runtime call/interaction ordering between agents, threads, or services. |
| `classDiagram` | UML static-structure diagram: classes with attributes/methods and their relationships (inheritance, composition, aggregation, association). | You need to model an object-oriented/Rust type model with fields, methods, and links between types. |
| `stateDiagram-v2` | State machine: states and the transitions between them. (`stateDiagram-v2` is the supported keyword; the older `stateDiagram` is deprecated.) | You need to model finite states and events that move a system between them (e.g., import lifecycle, gate pass/fail). |
| `erDiagram` | Entity-relationship diagram: entities, their attributes, and cardinality of relationships. | You need to model a data schema (tables/entities + relations), not code types. |
| `C4Context` / `C4Container` / `C4Component` (also `C4Dynamic`, `C4Deployment`) | C4 model for software architecture (Context, Container, Component levels). Mermaid's C4 syntax is compatible with C4-PlantUML. Marked **experimental**; syntax/styles may change. | You need a four-level architecture model (people/systems -> containers -> components). |
| `gitGraph` | Pictorial representation of git commits, branches, checkouts, and merges. | You need to illustrate a git branching/release strategy (NOT general workflows). |
| `gantt` | Project schedule: tasks as bars over time, with start/end dates and dependencies. | You need to show timed task scheduling/ordering over calendar time, not just logical order. |
| `mindmap` | Radial hierarchical map around a central concept; indentation defines branches. Marked **experimental** (syntax stable, icon integration not). | You need to explore/expand a single concept into nested sub-topics (brainstorming). |
| `timeline` | Events grouped into time periods (sections), left-to-right or top-down. Marked **experimental**. | You need a lightweight, ordered history of events where exact dates matter less than sequence within periods. |
| `block-beta` | Block diagrams representing complex systems/processes/architectures as blocks and connectors, with a dedicated beta layout engine. | You need architecture-style boxes-with-connectors that don't fit flowchart or C4 (e.g., layered stacks with explicit connectors). |

Sources:
- Flowchart: "Flowcharts are composed of nodes (geometric shapes) and edges (arrows or lines)." `https://mermaid.js.org/syntax/flowchart.html` (VERIFIED); "Instead of `flowchart` one can also use `graph`." (ibid.)
- Sequence: "A Sequence diagram is an interaction diagram that shows how processes operate with one another and in what order." `https://mermaid.js.org/syntax/sequenceDiagram.html` (VERIFIED).
- Class: "a class diagram ... describes the structure of a system by showing the system's classes, their attributes, operations ... and the relationships among objects." `https://mermaid.js.org/syntax/classDiagram` (VERIFIED).
- State: "Use State diagrams to model and document state machines, an abstract way of representing a system or an algorithm." `https://mermaid.js.org/syntax/stateDiagram.html` (VERIFIED; `stateDiagram-v2` is the supported keyword per the Mermaid README index at `https://github.com/mermaid-js/mermaid`).
- ER: "An entity-relationship diagram ... visualize the different types of entities that exist within a system" with relationships and cardinality. `https://mermaid.js.org/syntax/erDiagram.html` (VERIFIED via the syntax-reference example page which uses `erDiagram` as the declaration: `https://mermaid.js.org/intro/syntax-reference.html`).
- C4: "5 types of C4 charts are supported. System Context (C4Context), Container diagram (C4Container), Component diagram (C4Component) ... Mermaid's C4 diagram syntax is compatible with plantUML." `https://mermaid.js.org/syntax/c4.html` (VERIFIED); "C4 Diagram: This is an experimental diagram ... The syntax and properties can change" (ibid.).
- gitGraph: "A Git Graph is a pictorial representation of git commits and git actions (commands) on various branches." `https://mermaid.js.org/syntax/gitGraph.html` (VERIFIED).
- gantt: "A Gantt chart ... illustrates a project schedule and the amount of time it would take for any one project to finish." `https://mermaid.js.org/syntax/gantt.html` (VERIFIED).
- mindmap: "A mind map is a diagram used to visually organize information into a hierarchy, showing relationships among pieces of the whole." `https://mermaid.js.org/syntax/mindmap.html` (VERIFIED); "This is an experimental diagram" (ibid.).
- timeline: declared with the `timeline` keyword; "The syntax for creating Timeline diagram is simple." `https://mermaid.js.org/syntax/timeline.html` (VERIFIED); experimental (ibid.).
- block-beta: "Block diagrams are an intuitive and efficient way to represent complex systems, processes, or architectures visually. They are composed of blocks and connectors." `https://mermaid.js.org/syntax/block.html` (VERIFIED).

### Concrete mapping for OxiDJ's actual diagram subjects

Derived directly from `docs/architecture.md` §3 (Crate Layout), ADR-004's layered dependency table, and the gate tiers in `docs/research/rust-quality-gates.md` §1–2:

1. **Architecture layers / crate dependency rules** -> `flowchart` (`TB`/`LR`). ADR-004 already draws its layers as a top-to-bottom box stack with one-way `depends on` arrows; that is a dependency graph, which is exactly what `flowchart` models. Group each layer as a `subgraph` to mirror the ADR's horizontal layers, and annotate a forbidden edge (e.g., UI->core leak) with `classDef`/link styling to make the layer gate visible.
   - Why not `block-beta` or `C4Container`: `block-beta` adds no value over `flowchart` for a directed dependency graph, and `C4Container` would be over-specified for a 2-crate M0 scaffold (also experimental — see below).
2. **Crate dependency *rules* (the gate, not the inventory)** -> `flowchart` with `subgraph` per layer + a deliberately *forbidden* example edge styled red, paired with a one-line note that `cargo xtask layers` enforces the real rule. This is the human-readable companion to the executable gate in ADR-004 §M0-08 acceptance criteria.
   - Why not `gitGraph`: this is dependency structure, not git history.
3. **Metadata data model / schema** -> split by audience:
   - DB schema (ADR-005, deferred but planned; architecture.md §4 "database schema" deferred to ADR-005) -> `erDiagram` (entities: Track, Cue, BeatGrid, Playlist, Provenance; relationships with cardinality). Matches the "tables, columns, relations" intent.
   - In-process Rust type model (architecture.md §4 "neutral domain model" = Track, Cue, BeatGrid, Playlist, Provenance) -> `classDiagram` (types + fields + links to provenance enum).
   - Why not one diagram for both: `erDiagram` models storage entities/relations; `classDiagram` models code types. Using both keeps each accurate to its audience (ADR-003 is deferred to M1, so these are the two diagrams the model actually needs).
4. **Quality-gate tiers** -> `flowchart` (`LR`): the handoff/CI pipeline is a staged process with pass/fail branches (fmt -> clippy -> check -> nextest -> audit/deny). A flow with decision diamonds and `classDef` for Tier 1/2/3 coloring is the natural fit.
   - Why not `stateDiagram-v2`: gating tiers are a control flow with conditional branches over external tools, not state machines with named events.
   - Why not `gantt`: the tiers are logical sequence + conditional, not calendar scheduling.
5. **Worker / dispatch flows** -> `sequenceDiagram`: the task graph is actors (dispatcher, `sova` worker, parent task handoff) exchanging discrete ordered actions (spawn -> claim -> heartbeat -> complete). Ordering across processes is the defining need, which is `sequenceDiagram`'s exact niche.
   - Why not `flowchart`: a sequence diagram gives the time-ordered message flow (claim/heartbeat/handoff) that a plain flowchart obscures.

### One caution on C4
`C4Context`/`C4Container`/`C4Component` are marked **experimental** in Mermaid and, critically, are **not supported by GitHub's bundled Mermaid renderer** (see §2 / GitHub notes). Do not use C4 for anything that must render on GitHub. If OxiDJ later self-hosts docs (MkDocs/Mintlify), C4 becomes an option for the architecture layers; until then, `flowchart`+`subgraph` is the portable choice for the same layered view.

---

## 2. Dark/light contrast (both modes, good contrast)

### How Mermaid theming works (self-hosted)

Mermaid resolves colors from a layered config with this priority (highest first):
1. The diagram's own frontmatter/config or its `%%{init}%%` directive.
2. What the page passed to `mermaid.initialize()`.
3. The diagram type's default theme/look.
4. The global default (`theme: default`, `look: classic`, `layout: dagre`).
Source: `https://mermaid.js.org/config/theming.html` (VERIFIED).

**Available built-in themes** (enum on the `theme` config key): `default`, `base`, `dark`, `forest`, `neutral`, `neo`, `neo-dark`, `redux`, `redux-dark`, `redux-color`, `redux-dark-color`, and `null` (disables any pre-defined theme). `https://mermaid.js.org/config/schema-docs/config-properties-theme.html` and `https://mermaid.js.org/config/schema-docs/config.html` (VERIFIED); theme enum source: `https://mermaid.js.org/schemas/config.schema.json` (VERIFIED).

**Two important modifiers on themes:**
- The `dark` theme "goes well with dark-colored elements or dark-mode" and is meant to be paired with `darkMode: true`, which "affects how derived colors are calculated" (sets the diagram into dark-mode color derivation). Source: theming.html (VERIFIED).
- **`base` is the only theme that can be customized.** All `themeVariables` overrides are honored only when `theme: 'base'`. Source: theming.html ("base - This is the only theme that can be modified"), `https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/docs/config/theming.md` (VERIFIED).

**Per-diagram config is set via frontmatter or the (deprecated) `%%{init}%%` directive.** The directive form `%%{init: { "theme": "dark" } }%%` (first line of the diagram) sets the theme/config for that one diagram. Source: `https://mermaid.js.org/config/directives.html` and the mermaid source `docs/config/directives.md` at `https://github.com/mermaid-js/mermaid/blob/develop/docs/config/directives.md` (VERIFIED). **⚠ Deprecation:** directives are deprecated from v10.5.0 — "Please use the `config` key in frontmatter to pass configuration." Source: directives.html (VERIFIED). They still function, but new docs should prefer frontmatter `config` where the renderer supports it (see GitHub notes below for the constraint in GitHub Markdown).

**`themeVariables` and derived colors.** `themeVariables` holds the overridable palette: `primaryColor`, `primaryTextColor`, `primaryBorderColor`, `secondaryColor`, `secondaryBorderColor`, `secondaryTextColor`, `tertiaryColor`/`tertiaryBorderColor`/`tertiaryTextColor`, `lineColor`, `textColor`, `background`, `noteBkgColor`/`noteTextColor`/`noteBorderColor`, `fontFamily`, `fontSize`, and `darkMode`. Many values are *derived* (auto-calculated, inverted, darkened) from `primaryColor`/`background`, so setting the source variable updates downstream ones automatically. Source: theming.html (VERIFIED). The theming engine recognizes **hex colors only** (`#ff0000`), not color names — `red` is silently ignored. Source: `https://github.com/DataficationSDK/Verso/blob/main/docs/guides/mermaid-diagrams.md` quoting the engine (VERIFIED).

**Can `themeVariables` reference CSS custom properties `var(--x)`?** Not directly. `themeVariables` take literal hex values; `var()` is **not** resolved inside `themeVariables`. However, Mermaid exposes a separate `themeCSS` config field: a free-form CSS string injected into the rendered SVG, and **that** CSS *can* reference host-defined CSS custom properties. Example from the Verso integration guide:
```
%%{init: {'themeCSS': '.node rect { fill: var(--verso-accent-primary); stroke: var(--verso-border-default); }'}}%%
```
Source: `https://github.com/DataficationSDK/Verso/blob/main/docs/guides/mermaid-diagrams.md` (VERIFIED); `themeCSS` is declared as a `string` config property in the schema at `https://mermaid.js.org/config/schema-docs/config.html` (VERIFIED). So: `var(--x)` works **in `themeCSS`**, referencing variables defined on the host page/DOM around the SVG — but **not** inside `themeVariables` (which need concrete hex values). **Verified fact; the scope-limited caveat (host var resolution) is INFERRED** from how CSS variable resolution works in the embedding page.

**Restyling the rendered SVG with surrounding CSS (self-hosted).** Mermaid emits SVGs with stable, semantic class names that external/host CSS and `themeCSS` target. Confirmed from the Mermaid maintainers themselves: the `<g class="edgePaths">` / `edges edgePaths` group is "one of the most commonly targeted selectors outside Mermaid: custom `themeCSS`, host-application stylesheets, SVG post-processing and export pipelines, and integration tests all key off it," and renaming it is treated as a breaking change requiring a deprecation path. Source: PR #8125 `https://github.com/mermaid-js/mermaid/pull/8125` (VERIFIED). Node elements carry a `node` class (`classDef node ...` is documented and works; issue #684 at `https://github.com/mermaid-js/mermaid/issues/684` shows `classDef node ...` and `classDef edgePath ...` applied to the same flowchart — VERIFIED). Edge paths carry `.edgePath`/`.edgePaths` (issue #434, `https://github.com/mermaid-js/mermaid/issues/434`, confirms `.edgePath` selectors in the SVG — VERIFIED). Conclusion: **for a self-hosted docs site you fully control**, you can restyle the SVG via `themeCSS` (injecting CSS into the SVG) or by a host stylesheet that targets `.node`, `.edgePaths`, `.flowchart-link`, etc. **For GitHub (iframe), this is not possible** (see below).

### Auto light/dark switching: `prefers-color-scheme`

- Mermaid itself does **not** auto-detect `prefers-color-scheme` and switch themes. Per Mermaid's issue tracker, the standard community workaround is to add a `prefers-color-scheme: dark` listener and re-render the diagram with the `dark` theme — but even then, in `securityLevel: 'sandbox'` mode (the iframe mode GitHub-style hosts use) "the background of the resulting iframe remains white" and the background is "not responsive to enabling/disabling dark mode." Source: issue #5034 `https://github.com/mermaid-js/mermaid/issues/5034` (VERIFIED).
- A third-party blog claims `prefers-color-scheme` works inside SVG documents (Chrome only, not Safari) to flip styling, and that mermaid.live "switches the theme in config based on the color scheme." Source: issue #2644 `https://github.com/mermaid-js/mermaid/issues/2644` (VERIFIED as "reported"; the cross-browser/Safari limitation and the need to re-render are stated there). So auto-switching is possible only with a JS host that re-renders on the media-query change — **not** out of the box, and **not** in an isolated iframe.

### GitHub rendering (the hard constraint) — VERIFIED vs the rumors

GitHub's own engineering blog describes exactly what GitHub does: "When we encounter code blocks marked as `mermaid`, we generate an iframe that takes the raw Mermaid syntax and passes it to Mermaid.js, turning that code into a diagram in your local browser," via GitHub's HTML pipeline + Viewscreen service. Source: "Include diagrams in your Markdown files with Mermaid," GitHub Blog, 2022-02, `https://github.blog/developer-skills/github/include-diagrams-markdown-files-mermaid/` (VERIFIED); changelog announcement 2022-03-17 `https://github.blog/changelog/2022-03-17-mermaid-topojson-geojson-and-ascii-stl-diagrams-are-now-supported-in-markdown-and-as-files/` (VERIFIED).

Three derived facts (two VERIFIED, one INFERRED, and one explicit UNVERIFIED conflict):

1. **GitHub renders Mermaid inside a sandboxed iframe**, and "User-supplied content is locked away in an iframe, where it has less potential to cause mischief." Source: the GitHub Blog post (VERIFIED). Consequence: a page cannot inject `themeCSS` or host `.node`/`.edgePaths` stylesheets into GitHub's Mermaid iframe — the SVG is not reachable by the surrounding document's CSS. (INFERRED: standard iframe same-origin/CSS-isolation behavior, which the blog's "locked away in an iframe" phrasing directly implies.)

2. **GitHub's Mermaid renderer does NOT auto-adapt to the viewer's GitHub colour scheme / `prefers-color-scheme`.** The Mermaid project's own issue #5034 confirms that Mermaid-in-iframe backgrounds are not responsive to dark mode by default and stay white. A third-party article re-states the GitHub-specific consequence: inline `mermaid` code blocks are "rendered by GitHub's own Mermaid runtime at view time, with no hook for swapping based on theme," and "Inline mermaid code blocks in your README can't use `<picture>`." Source: issue #5034 (VERIFIED) + `https://dev.to/levi_liu/why-your-github-readme-diagrams-look-amateur-and-the-4-fixes-that-take-10-minutes-4a9c` (VERIFIED as a cited source; this is the only GitHub-specific dark-mode statement found, so treat its mechanism description as VERIFIED-by-citation but note it is a third-party blog). The conservative, defensible conclusion: **do not expect GitHub Mermaid blocks to flip with the viewer's theme.**

3. **GitHub officially documents `<picture>` + `prefers-color-scheme` for *images*** (not Mermaid blocks). The GitHub "Quickstart for writing on GitHub" shows `<picture><source media="(prefers-color-scheme: dark)" srcset="..."><img ...></picture>` exactly for light/dark image switching. Source: `https://docs.github.com/en/get-started/writing-on-github/quickstart-for-writing-on-github` (multiple versioned mirrors, VERIFIED). This is the **only** GitHub-supported adaptive-contrast mechanism — and it requires pre-rendered image files, not inline Mermaid.

4. **CONFLICTING, UNVERIFIED claim about `%%{init}%%` on GitHub.** One third-party blog ("starmorph") asserts GitHub "strips" `%%{init: ...}%%` theme directives and that "All diagrams render with GitHub's default theme." Another ("mermaideditor") asserts the opposite — that "GitHub automatically detects the user's system theme and renders Mermaid diagrams appropriately." These directly contradict each other, and neither is backed by an authoritative GitHub source. GitHub's own blog (which only says "raw Mermaid syntax ... passed to Mermaid.js") is consistent with init directives being processed, but does not confirm or deny it. Given issue #5034 (no auto-dark) and this conflict, the safe, source-defensible rule is: **treat GitHub's Mermaid rendering as non-themeable and non-adaptive.** If you need guaranteed dual-theme contrast on GitHub, pre-render to SVG (see §3) and use `<picture>`/`#gh-dark-mode-only`/`#gh-light-mode-only`; if you keep inline Mermaid on GitHub, make diagrams theme-agnostic (high-contrast, avoid theme-dependent background assumptions). The `%%{init}%%` directive should not be relied upon for GitHub-specific theming. (UNVERIFIED conflict, flagged per the "say so explicitly rather than guessing" rule.)

**Versioning caveat.** Mermaid v12 is current; theming defaults (e.g., `neo` look, `redux-color` theme, per-diagram defaults) changed across v11–v12, and directives were deprecated at v10.5.0. GitHub's bundled Mermaid version is **not published** by GitHub and is not the latest — so directive-vs-frontmatter support and theme names may lag. Source: deprecation text in `https://mermaid.js.org/config/directives.html` (VERIFIED); version-gated theme/look statements in `https://mermaid.js.org/config/theming.html` and per-diagram pages (VERIFIED). This version lag is why the document recommends not leaning on cutting-edge Mermaid features for GitHub-rendered docs.

### Practical dual-theme recipe

For a **self-hosted** docs site (MkDocs, Mintlify, etc. — anything you control):
- Use the `base` theme + `themeVariables` with explicit hex colors chosen for contrast, OR
- Use `themeCSS` with `var(--host-light-var)` / `var(--host-dark-var)` resolved by the host page (host defines the vars under `prefers-color-scheme` media blocks).
Recipe source: Verso guide `https://github.com/DataficationSDK/Verso/blob/main/docs/guides/mermaid-diagrams.md` (VERIFIED) and theming.html (VERIFIED).

For **GitHub** (inline Mermaid):
- Do not attempt per-theme styling via `%%{init}%%` (unverified/conflicting). Keep diagrams theme-agnostic: dark strokes/text on transparent/neutral fills so they read on both GitHub light and GitHub dark backgrounds. (INFERRED heuristic — "transparent background" is INFERRED, not confirmed via a primary source, because GitHub's exact SVG background handling is not documented.)

### Recommendation on GitHub dark mode
If and when a self-hosted docs site is added (an M1+ decision, not yet made), enable true adaptive rendering: re-render on `prefers-color-scheme` change with `themeCSS` referencing host CSS variables, or pre-render light/dark SVGs behind a `<picture>`. For now (GitHub-only), treat Mermaid as light-preference and theme-agnostic.

---

## 3. Media tags (correct Markdown embedding on GitHub)

### Fenced ` ```mermaid ` blocks
The canonical way to render Mermaid on GitHub: a fenced code block with the `mermaid` language identifier. GitHub detects `pre.mermaid` and renders the diagram. Source: GitHub Blog "Include diagrams in your Markdown files with Mermaid" (VERIFIED — the example uses exactly ` ```mermaid graph TD; A-->B; ... `). Also supported as standalone **`.mermaid` / `.mmd`** files which GitHub renders on view. Source: changelog 2022-03-17 (VERIFIED).

### Inline images: `![alt](relative/path)`
Standard CommonMark image syntax. For diagrams that must be theme-adaptive on GitHub, commit a pre-rendered **SVG** (or PNG) and embed with `![alt text](docs/diagrams/foo.svg)`. Source: CommonMark spec `https://spec.commonmark.org/0.30/` (VERIFIED for image syntax); GitHub docs for the file types GitHub supports (SVG upload allowed since 2022-01-21; changelog `https://github.blog/changelog/2022-01-21-allow-to-upload-svg-files-to-markdown/` (VERIFIED)).
- **SVG vs PNG for theme adaptability**: SVG is vector (crisp at any size, small) and can carry a `<style>` with `@media (prefers-color-scheme)` so a single committed SVG flips internally — but GitHub **strips/sanitizes** inline SVG/CSS on upload for security, so an uploaded SVG will **not** honor its internal `@media` dark-mode rules and is treated as a static image. Source: GitHub's upload sanitization is implied by the "stripped" behavior; the reliable adaptive mechanism is therefore two files + `<picture>` (next row). Treat "uploaded SVG internal `@media` is stripped" as INFERRED (security-sanitization reasoning), because GitHub has not published the exact SVG sanitization rule set. PNG is pure raster, simplest, always renders, but does not scale and needs two files for dual-theme. Use PNG only when SVG tooling is unavailable.
- **Adaptive image on GitHub**: GitHub's documented, supported pattern is `<picture>` with `prefers-color-scheme` sources, plus the GitHub-specific shortcode fragments `#gh-dark-mode-only` / `#gh-light-mode-only` appended to image URLs as an alternative. Source: `https://docs.github.com/en/get-started/writing-on-github/quickstart-for-writing-on-github` (the `<picture>` form, VERIFIED) and the widely-documented GitHub dark/light image fragments (third-party example with the exact markup: `https://github.com/acostalima/til/blob/master/github/using-dark-and-light-mode-specific-images-in-markdown.md` — VERIFIED as the canonical pattern). Produce two SVGs (light + dark) via `mmd`/`@mermaid-js/mermaid-cli` or `mermaid-cli`, commit both, and embed with `<picture>` + `srcset`. (The `mmd`/dual-SVG tooling is INFERRED as the production approach — I did not run it; the markup pattern itself is VERIFIED.)

### `MEDIA:/absolute/path` — NOT a Markdown tag
`MEDIA:/abs/path/to/file` is **not** a Markdown or GitHub mechanism. It is a **Hermes Agent platform-internal convention**: the chat gateway scans an agent's reply for `MEDIA:/...` paths (or bare absolute/home-relative paths ending in a supported extension) and ships the referenced file as a native platform attachment (image inline, PDF/CSV/MD as file, etc.). Paths inside fenced code blocks and inline backticks are ignored so code samples are not mangled. Source: `https://hermes-agent.nousresearch.com/docs/user-guide/features/deliverable-mode` ("The gateway scans agent responses for file paths... Paths inside code blocks and inline code are ignored") and `https://hermes-agent.nousresearch.com/docs/user-guide/messaging/telegram` (supported `MEDIA:` extensions + host-path caveat) (VERIFIED).
- **Do not use `MEDIA:` in committed Markdown docs.** It is a chat-rendering hint for the Hermes gateway only. In a terminal session like this one, `MEDIA:` tags print as literal text (no attachment channel). The correct media tag in committed OxiDJ docs is `![alt](relative/path)` for images and ` ```mermaid ` for live diagrams on GitHub.
- Note for agents: per Hermes' own rule, generating a file and stating its absolute path is sufficient — the gateway extracts it; you do not need to emit a `MEDIA:` tag. (VERIFIED per deliverable-mode doc.)

### Tag-use summary
| Where the reader is | Use this | For theme-adaptive diagrams |
|---|---|---|
| GitHub Markdown (committed docs) | ` ```mermaid ` for live diagrams; `![alt](rel/path.svg)` or `<picture>`+`prefers-color-scheme` for pre-rendered | Two SVGs + `<picture>` (Mermaid can't adapt in GitHub's iframe) |
| Hermes chat (agent replies) | Mention the generated file's absolute path; gateway attaches it; never `MEDIA:` in committed docs | N/A — not a Markdown mechanism |

---

## 4. Pitfalls (failure modes, especially for agent-written docs)

1. **Breaking keywords in diagram text.** The word `End` breaks `flowchart` and `sequenceDiagram` (it is parsed as the diagram terminator). The official "Diagram Breaking" guidance: wrap such tokens in quotes. Source: `https://mermaid.js.org/intro/syntax-reference.html` "Diagram Breaking" section (VERIFIED — quotes `'End'`). Agent pitfall: scribbling prose into node labels unquoted.
2. **Deprecated directive syntax.** `%%{init}%%` is deprecated from v10.5.0 in favor of frontmatter `config`. Source: `https://mermaid.js.org/config/directives.html` (VERIFIED). Risk: directives still parse today but are the long-term-replaced form; mixing them with new frontmatter `config` is inconsistent.
3. **Frontmatter sanitization drops theme variables.** Issue #8170 documents that several `themeVariables` set via frontmatter/`%%{init}%%` "never reach the renderer" because the frontmatter path runs through `sanitizeDirective`, which deletes any `themeVariables` key whose name is not in the flat `configKeys` set (derived from the default theme's variables). They work via `mermaid.initialize({...})` but not per-diagram. Source: `https://github.com/mermaid-js/mermaid/issues/8170` (VERIFIED). Agent pitfall: setting an obscure `themeVariables` per-diagram and wondering why colors don't change.
4. **C4 is experimental and unsupported on GitHub.** C4 renders in Mermaid core but GitHub's bundled Mermaid omits the C4 extension, so C4 blocks appear as raw text on GitHub. Source: GitHub Community discussion #197898 `https://github.com/orgs/community/discussions/197898` (VERIFIED): "GitHub's built-in Mermaid renderer does not support the C4 diagram extension ... render as raw text on GitHub instead of as diagrams." C4 docs also mark it experimental. Source: `https://mermaid.js.org/syntax/c4.html` (VERIFIED). Agent pitfall: reaching for C4 (pretty for architecture) and shipping docs that show literal `C4Context`/`Person(...)` text on GitHub.
5. **`graph` vs `flowchart` and keyword case sensitivity.** `graph` is an allowed alias for `flowchart`, but the type keyword is otherwise case-sensitive and the first token decides the parser — a typo (e.g., `flowcahr`) renders as plain text. Source: flowchart docs + syntax-structure page (VERIFIED). Agent pitfall: copy-framing a keyword from a non-Mermaid source.
6. **Oversized diagrams / edge caps.** Mermaid will refuse to render (or throw) when a graph exceeds `maxEdges` (a config integer in the schema). Source: `https://mermaid.js.org/schemas/config.schema.json` `maxEdges` property (VERIFIED). Agent pitfall: generating a giant dependency graph with all transitive crate edges in one diagram. Mitigation: scope to one layer per diagram + `subgraph`; keep under the edge cap.
7. **Unreadable-in-one-theme diagrams (the core requirement).** Because GitHub's Mermaid iframe does not adapt to the viewer's scheme (§2), a diagram themed for light (dark text) can become illegible on GitHub Dark, and vice-versa. The only GitHub-supported fix is pre-rendered dual SVG via `<picture>`. Source: issue #5034 + dev.to article (VERIFIED as cited), and `prefers-color-scheme` `<picture>` supported by GitHub docs (VERIFIED).
8. **Accessibility: alt text and fallbacks.** GitHub renders an inline ` ```mermaid ` block as a static SVG with **no `alt`/title from the author**, and the block degrades to the raw source text only in non-JS/API contexts (the GitHub Blog notes API requests "will see the original Markdown code"). There is no authoring hook for `alt` on inline Mermaid. Source: GitHub Blog fallback description (VERIFIED). Mitigation: keep diagrams small and well-captioned in prose; for non-decorative diagrams, prefer a pre-rendered `<img alt="...">` (or `<picture>`) so you control the alt text. Inline Mermaid is therefore fine for decorative/illustrative diagrams but should not carry information that has no prose restatement.
9. **Agent-authored drift and stale syntax.** Because Mermaid changes per-diagram defaults across v11/v12 and deprecates directives at v10.5.0, and because GitHub's bundled version lags the latest, an agent copying an example from mermaid.live (current) can emit syntax a reader's GitHub renderer doesn't support. Source: version-gated theme/look notes on theming.html and per-diagram pages (VERIFIED) + the GitHub-version-lag caveat in this doc. Mitigation: pin docs to long-stable syntax only (`flowchart`, `sequenceDiagram`, `classDiagram`, `erDiagram`, `gantt`), avoid `neo`/`redux` theme names and `architecture-beta`/`block-beta` exotic syntax in GitHub-rendered docs, and link each diagram to its Mermaid type explicitly in prose.

---

## Recommendation (actionable)

For OxiDJ's documentation, adopt a single portable profile plus one adaptive escape hatch:

1. **Profile A — GitHub-native Mermaid (default choice).** Use `flowchart` for architecture layers, crate dependency rules, and the quality-gate Tier flow; `erDiagram` for the DB/data schema; `classDiagram` for the Rust type model; `sequenceDiagram` for worker/dispatch flows. Every block starts with a `mermaid` fence. Keep diagrams theme-agnostic (dark-on-neutral fills) because GitHub's Mermaid iframe does not auto-switch to the viewer's colour scheme (issue #5034; GitHub Blog iframe rendering). **Do not use C4** — GitHub does not render it (discussion #197898).

2. **Profile B — adaptive diagrams where dual-theme contrast is required.** Pre-render to two SVGs (light + dark) and embed with `<picture>` + `prefers-color-scheme`, using GitHub's documented shortcode fragments (`#gh-dark-mode-only` / `#gh-light-mode-only`) as the lighter-weight alternative for simple cases. Source: `docs.github.com/.../quickstart-for-writing-on-github` (VERIFIED).

3. **Theme control.** Do not attempt `%%{init: {'theme':...}}%%` styling on GitHub (unverified and conflict-reported); rely on GitHub's default + theme-agnostic design. On a future self-hosted site, switch to the `base` theme + `themeCSS` referencing host `var(--...)` definitions (with a `prefers-color-scheme` re-render listener), since `themeVariables` are hex-only and `var()` is only resolved in `themeCSS`. Sources: theming.html, schema-docs/config.html, Verso guide (all VERIFIED).

4. **Media discipline.** Committed docs use `![alt](rel/path)` for images and ` ```mermaid ` for live diagrams. `MEDIA:/abs/path` is a Hermes chat-gateway hint only — never commit it. Source: hermes-agent docs (VERIFIED).

5. **Agent guardrails.** Quote breaking tokens (e.g. `'End'`); avoid deprecated directives in new docs; keep diagrams under the `maxEdges` cap by splitting layers; always caption non-decorative diagrams in prose because inline Mermaid has no `alt` text on GitHub. Sources: syntax-reference "Diagram Breaking", directives.html deprecation, config schema `maxEdges`, GitHub Blog fallback (all VERIFIED).

These choices keep every diagram portable to GitHub (the only rendering target OxiDJ currently commits docs to), satisfy the dual-theme contrast requirement via Profile B where it matters, and avoid the C4/directive/version traps that break agent-written Mermaid docs.

---

## Sources (all cited inline above)

Authoritative / primary:
- Mermaid theming — `https://mermaid.js.org/config/theming.html`
- Mermaid directives (incl. v10.5.0 deprecation) — `https://mermaid.js.org/config/directives.html`; source mirror `https://github.com/mermaid-js/mermaid/blob/develop/docs/config/directives.md`
- Mermaid config schema (theme enum, `maxEdges`, `themeCSS`, `darkMode`) — `https://mermaid.js.org/schemas/config.schema.json`; `https://mermaid.js.org/config/schema-docs/config.html`; `https://mermaid.js.org/config/schema-docs/config-properties-theme.html`; interface `https://mermaid.js.org/config/setup/mermaid/interfaces/MermaidConfig.html`
- Mermaid syntax index — `https://mermaid.js.org/intro/syntax-reference.html` (incl. "Diagram Breaking")
- Per-type syntax: flowchart `https://mermaid.js.org/syntax/flowchart.html`; sequence `https://mermaid.js.org/syntax/sequenceDiagram.html`; class `https://mermaid.js.org/syntax/classDiagram`; state `https://mermaid.js.org/syntax/stateDiagram.html`; ER `https://mermaid.js.org/syntax/erDiagram.html`; C4 `https://mermaid.js.org/syntax/c4.html`; gitGraph `https://mermaid.js.org/syntax/gitGraph.html`; gantt `https://mermaid.js.org/syntax/gantt.html`; mindmap `https://mermaid.js.org/syntax/mindmap.html`; timeline `https://mermaid.js.org/syntax/timeline.html`; block `https://mermaid.js.org/syntax/block.html`
- Mermaid README (diagram inventory) — `https://github.com/mermaid-js/mermaid`
- GitHub rendering & fallback — `https://github.blog/developer-skills/github/include-diagrams-markdown-files-mermaid/` (2022-02); changelog `https://github.blog/changelog/2022-03-17-mermaid-topojson-geojson-and-ascii-stl-diagrams-are-now-supported-in-markdown-and-as-files/`
- GitHub dark/light images (`<picture>`, `#gh-*-mode-only`) — `https://docs.github.com/en/get-started/writing-on-github/quickstart-for-writing-on-github`
- GitHub C4 unsupported — `https://github.com/orgs/community/discussions/197898`
- Mermaid iframe dark-mode background issue — `https://github.com/mermaid-js/mermaid/issues/5034`
- Mermaid prefers-color-scheme / re-render discussion — `https://github.com/mermaid-js/mermaid/issues/2644`
- SVG class contract (`.node`, `.edgePaths`, breaking-change policy) — `https://github.com/mermaid-js/mermaid/pull/8125`; `https://github.com/mermaid-js/mermaid/issues/684`; `https://github.com/mermaid-js/mermaid/issues/434`
- themeVariables sanitization bug — `https://github.com/mermaid-js/mermaid/issues/8170`
- Hex-only theming engine — `https://github.com/DataficationSDK/Verso/blob/main/docs/guides/mermaid-diagrams.md`
- GitHub SVG upload allowed — `https://github.blog/changelog/2022-01-21-allow-to-upload-svg-files-to-markdown/`
- GitHub dark/light image fragments pattern — `https://github.com/acostalima/til/blob/master/github/using-dark-and-light-mode-specific-images-in-markdown.md`
- OxiDJ sources read for this task: `docs/architecture.md` §3, §4, §6; `docs/adr/004-crate-boundaries.md`; `docs/research/rust-quality-gates.md` §1–2.

Third-party (cited, but flagged as such above):
- `https://dev.to/levi_liu/why-your-github-readme-diagrams-look-amateur-and-the-4-fixes-that-take-10-minutes-4a9c`
- `https://blog.starmorph.com/blog/mermaid-js-tutorial` (the "GitHub strips init directives" claim — flagged UNVERIFIED/conflicting)
- `https://mermaidEditor.lol/blog/mermaid-themes-dark-mode` (the "GitHub auto-adapts" claim — flagged UNVERIFIED/conflicting, contradicts issue #5034)
