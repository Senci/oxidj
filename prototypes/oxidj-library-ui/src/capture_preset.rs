//! Opt-in no-device silent startup; screenshot capture itself is external.
use crate::{app::LibraryApp, decks::DeckAudio};

/// An explicit disposable-catalog acknowledgement is deliberately mandatory.
/// There is no default-db or environment fallback for this opt-in route.
pub(crate) fn cli(
    args: &[std::ffi::OsString],
) -> Result<Option<(std::path::PathBuf, Preset)>, String> {
    if !args
        .iter()
        .any(|arg| arg == "--capture-preset" || arg == "--capture-disposable-db")
    {
        return Ok(None);
    }
    if args.len() != 5
        || args[0] != "--db"
        || args[2] != "--capture-preset"
        || args[4] != "--capture-disposable-db"
    {
        return Err("usage: --db /absolute/disposable.sqlite --capture-preset PRESET.json --capture-disposable-db".into());
    }
    let db = std::path::PathBuf::from(&args[1]);
    if !db.is_absolute() || !std::fs::symlink_metadata(&db).is_ok_and(|m| m.file_type().is_file()) {
        return Err(
            "--db must be an explicit absolute regular-file disposable catalog (no symlink)".into(),
        );
    }
    let raw = std::fs::read_to_string(&args[3]).map_err(|e| format!("cannot read preset: {e}"))?;
    Ok(Some((db, Preset::parse(&raw)?)))
}

pub(crate) struct Preset {
    ids: [u64; 2],
    positions: [f64; 2],
    crossfader: f32,
    master: f32,
    sync_deck: usize,
    tempo_range: crate::settings::TempoRange,
    faders: [f32; 2],
    filters: [f32; 2],
    eq: [[f32; 3]; 2],
    palette: crate::settings::DeckPalette,
    sections: crate::settings::DeckSections,
    zoom_bars: Option<usize>,
    show_artwork: Option<bool>,
    show_meta: Option<bool>,
    show_phase: Option<bool>,
    pub(crate) phase: Phase,
    since: std::time::Instant,
    sent: [bool; 2],
}

impl Preset {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let o = v.as_object().ok_or("preset must be an object")?;
        if o.keys().any(|k| {
            ![
                "track_ids",
                "positions_ms",
                "crossfader",
                "master",
                "sync_deck",
                "tempo_range",
                "faders",
                "filters",
                "eq_db",
                "palette",
                "sections",
                "zoom_bars",
                "show_artwork",
                "show_meta",
                "show_phase",
            ]
            .contains(&k.as_str())
        }) {
            return Err("unknown preset field".into());
        }
        let array = |k: &str| -> Result<&Vec<serde_json::Value>, String> {
            o.get(k)
                .and_then(|v| v.as_array())
                .filter(|a| a.len() == 2)
                .ok_or_else(|| format!("{k} must contain two values"))
        };
        let ids = array("track_ids")?;
        let id = |i: usize| {
            ids[i]
                .as_u64()
                .filter(|x| *x != 0)
                .ok_or("invalid track ID".to_owned())
        };
        let pos = array("positions_ms")?;
        let position = |i: usize| {
            pos[i]
                .as_f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .ok_or("invalid position".to_owned())
        };
        let unit = |k: &str| {
            o.get(k)
                .and_then(|v| v.as_f64())
                .filter(|x| (0.0..=1.0).contains(x))
                .map(|v| v as f32)
                .ok_or_else(|| format!("invalid {k}"))
        };
        let sync_deck = o
            .get("sync_deck")
            .and_then(|v| v.as_u64())
            .filter(|v| *v < 2)
            .ok_or("invalid sync_deck")? as usize;
        let tempo_range = match o.get("tempo_range") {
            None => crate::settings::TempoRange::Ten,
            Some(v) => match v.as_u64() {
                Some(6) => crate::settings::TempoRange::Six,
                Some(10) => crate::settings::TempoRange::Ten,
                Some(16) => crate::settings::TempoRange::Sixteen,
                _ => return Err("tempo_range must be 6, 10, or 16".into()),
            },
        };
        let number =
            |v: &serde_json::Value, min: f32, max: f32, name: &str| -> Result<f32, String> {
                v.as_f64()
                    .filter(|v| v.is_finite() && *v >= f64::from(min) && *v <= f64::from(max))
                    .map(|v| v as f32)
                    .ok_or_else(|| format!("invalid {name}"))
            };
        let f = array("faders")?;
        let filter = array("filters")?;
        let eq = array("eq_db")?;
        let band = |i: usize| -> Result<[f32; 3], String> {
            let a = eq[i]
                .as_array()
                .filter(|a| a.len() == 3)
                .ok_or("eq_db must contain two low/mid/high arrays")?;
            Ok([
                number(
                    &a[0],
                    oxidj_audio::EQ_KILL_DB,
                    oxidj_audio::EQ_MAX_DB,
                    "eq_db",
                )?,
                number(
                    &a[1],
                    oxidj_audio::EQ_KILL_DB,
                    oxidj_audio::EQ_MAX_DB,
                    "eq_db",
                )?,
                number(
                    &a[2],
                    oxidj_audio::EQ_KILL_DB,
                    oxidj_audio::EQ_MAX_DB,
                    "eq_db",
                )?,
            ])
        };
        let palette = match o.get("palette").and_then(|v| v.as_str()) {
            Some("classic") => crate::settings::DeckPalette::Classic,
            Some("warm-cool") => crate::settings::DeckPalette::WarmCool,
            _ => return Err("palette must be classic or warm-cool".into()),
        };
        let sections = o
            .get("sections")
            .and_then(|v| v.as_array())
            .filter(|a| a.len() == 4)
            .ok_or("sections must be four booleans: detail, overviews, strip, mixer")?;
        let section = |i: usize| sections[i].as_bool().ok_or("invalid section");
        let optional_bool = |k: &str| -> Result<Option<bool>, String> {
            o.get(k)
                .map(|v| v.as_bool().ok_or_else(|| format!("invalid {k}")))
                .transpose()
        };
        let zoom_bars = o
            .get("zoom_bars")
            .map(|v| {
                v.as_u64()
                    .and_then(|v| usize::try_from(v).ok())
                    .filter(|v| crate::settings::ZOOM_BARS.contains(v))
                    .ok_or("invalid zoom_bars")
            })
            .transpose()?;
        Ok(Self {
            faders: [
                number(&f[0], 0.0, 1.0, "faders")?,
                number(&f[1], 0.0, 1.0, "faders")?,
            ],
            filters: [
                number(&filter[0], -1.0, 1.0, "filters")?,
                number(&filter[1], -1.0, 1.0, "filters")?,
            ],
            eq: [band(0)?, band(1)?],
            ids: [id(0)?, id(1)?],
            positions: [position(0)?, position(1)?],
            crossfader: unit("crossfader")?,
            master: unit("master")?,
            sync_deck,
            tempo_range,
            palette,
            sections: crate::settings::DeckSections {
                detail: section(0)?,
                overviews: section(1)?,
                strip: section(2)?,
                mixer: section(3)?,
            },
            zoom_bars,
            show_artwork: optional_bool("show_artwork")?,
            show_meta: optional_bool("show_meta")?,
            show_phase: optional_bool("show_phase")?,
            phase: Phase::Loading,
            since: std::time::Instant::now(),
            sent: [false; 2],
        })
    }
}

/// Only the local START button can arm playback; no file/socket control API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Loading,
    Seeking,
    Syncing,
    Ready,
    Starting,
    Running,
    Failed,
}

impl Preset {
    fn advance(&mut self, phase: Phase) {
        self.phase = phase;
        self.since = std::time::Instant::now();
        self.sent = [false; 2];
    }
}

impl LibraryApp {
    pub(crate) fn start_capture_preset(&mut self, preset: Preset) -> Result<(), String> {
        if self.capture_preset.is_some() {
            return Err("capture: session already configured".into());
        }
        if self.decks.engine.output() != oxidj_audio::OutputKind::Silent {
            return Err("capture: requires the dedicated no-device silent engine".into());
        }
        for id in preset.ids {
            if !self.tracks.iter().any(|t| t.track.id.as_u64() == id) {
                return Err(format!("capture: track ID {id} not found"));
            }
        }
        self.show_decks = true;
        self.show_details = true;
        self.settings.sections = preset.sections;
        self.settings.crossfader = true;
        self.settings.deck_palette = preset.palette;
        if let Some(v) = preset.zoom_bars {
            self.settings.zoom_bars = v;
        }
        if let Some(v) = preset.show_artwork {
            self.settings.show_artwork = v;
        }
        if let Some(v) = preset.show_meta {
            self.settings.show_meta = v;
        }
        if let Some(v) = preset.show_phase {
            self.settings.show_phase = v;
        }
        self.settings.tempo_range = preset.tempo_range;
        self.decks.range = crate::decks::tempo_range(self.settings.tempo_range);
        self.decks.crossfader = preset.crossfader;
        self.decks.master = preset.master;
        for d in 0..2 {
            self.load_deck(d, preset.ids[d]);
            self.decks.decks[d].volume = crate::decks::Volume(preset.faders[d]);
            self.decks.decks[d].channel.filter = preset.filters[d];
            self.decks.decks[d].channel.eq_db = preset.eq[d];
        }
        self.notice = "capture: LOADING (silent, paused)".into();
        self.capture_preset = Some(preset);
        Ok(())
    }

    /// One-shot local signal, shared by the capture START button and tests.
    pub(crate) fn request_capture_start(&mut self) -> bool {
        let Some(p) = &mut self.capture_preset else {
            return false;
        };
        if p.phase != Phase::Ready {
            return false;
        }
        p.advance(Phase::Starting);
        self.notice = "capture: STARTING (silent)".into();
        true
    }

    pub(crate) fn pump_capture_preset(&mut self, ctx: &eframe::egui::Context) {
        let Some(mut p) = self.capture_preset.take() else {
            return;
        };
        if let Err(reason) = self.capture_step(&mut p) {
            // Cancel queued STARTs too. A toggle is NOT a safe pause operation.
            self.decks.abort_capture(&reason);
            p.advance(Phase::Failed);
            self.notice = format!("capture: FAILED (paused): {reason}");
        }
        if !matches!(p.phase, Phase::Ready | Phase::Failed) {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        self.capture_preset = Some(p);
    }

    fn capture_step(&mut self, p: &mut Preset) -> Result<(), String> {
        use oxidj_audio::{Command, OutputKind};
        if p.phase == Phase::Failed {
            return Ok(());
        }
        if self.decks.engine.output() != OutputKind::Silent {
            return Err("silent output changed".into());
        }
        if !matches!(p.phase, Phase::Ready | Phase::Running)
            && p.since.elapsed() > std::time::Duration::from_secs(30)
        {
            return Err(format!("{:?} timed out", p.phase));
        }
        for d in &self.decks.decks {
            if let DeckAudio::Failed(reason) = &d.audio {
                return Err(reason.clone());
            }
        }
        let acknowledged = (0..2).all(|d| {
            self.decks.engine.deck(d).id == Some(p.ids[d]) && !self.decks.engine.pending(d)
        });
        if !matches!(p.phase, Phase::Starting | Phase::Running)
            && (0..2).any(|d| {
                let s = self.decks.engine.deck(d);
                s.playing || s.previewing
            })
        {
            return Err("unexpected playback before START".into());
        }
        match p.phase {
            Phase::Loading if acknowledged && self.decks.decks.iter().all(|d| d.ready()) => {
                for d in 0..2 {
                    if p.positions[d] >= self.decks.decks[d].duration() {
                        return Err(format!("position outside deck {} track", d + 1));
                    }
                }
                crate::decks::sync_plan(&self.decks.decks, p.sync_deck, self.decks.range)?;
                p.advance(Phase::Seeking);
                self.notice = "capture: SEEKING (paused)".into();
            }
            Phase::Seeking => {
                // Never SYNC in the pump that submits a seek. Both decks must
                // acknowledge their target positions first (either follower).
                let previously_sent = p.sent.iter().all(|s| *s);
                for d in 0..2 {
                    if !p.sent[d] {
                        let DeckAudio::Ready { rate } = self.decks.decks[d].audio else {
                            return Err("deck no longer ready".into());
                        };
                        p.sent[d] = self.decks.engine.send(Command::Seek {
                            deck: d,
                            frame: p.positions[d] * f64::from(rate) / 1000.0,
                        });
                    }
                }
                if previously_sent && acknowledged {
                    for d in 0..2 {
                        let DeckAudio::Ready { rate } = self.decks.decks[d].audio else {
                            return Err("deck no longer ready".into());
                        };
                        let target = p.positions[d] * f64::from(rate) / 1000.0;
                        if (self.decks.engine.deck(d).frame - target).abs() > 1.0 {
                            return Ok(());
                        }
                    }
                    if self.decks.try_capture_sync(p.sync_deck)? {
                        p.advance(Phase::Syncing);
                        self.notice = "capture: SYNCING (paused)".into();
                    }
                }
            }
            Phase::Syncing if acknowledged => {
                p.advance(Phase::Ready);
                self.notice = "capture: READY (silent, paused) — press START once".into();
            }
            Phase::Starting => {
                let previously_sent = p.sent.iter().all(|s| *s);
                for d in 0..2 {
                    if !p.sent[d] {
                        p.sent[d] = self.decks.engine.send(Command::TogglePlay { deck: d });
                    }
                }
                if previously_sent
                    && acknowledged
                    && (0..2).all(|d| self.decks.engine.deck(d).playing)
                {
                    p.advance(Phase::Running);
                    self.notice = "capture: RUNNING (silent; no screenshot taken)".into();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
