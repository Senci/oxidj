use crate::theme::ColorTokens;
use eframe::egui::{self, Color32, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Concept {
    StudioRack,
    NeonDeck,
    PaperConsole,
    ModularBay,
    Broadcast,
    MinimalLine,
    MeterBridge,
    SoftTiles,
    UtilityGrid,
    StagePanel,
}

impl Concept {
    pub const ALL: [Self; 10] = [
        Self::StudioRack,
        Self::NeonDeck,
        Self::PaperConsole,
        Self::ModularBay,
        Self::Broadcast,
        Self::MinimalLine,
        Self::MeterBridge,
        Self::SoftTiles,
        Self::UtilityGrid,
        Self::StagePanel,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::StudioRack => "Studio rack",
            Self::NeonDeck => "Neon deck",
            Self::PaperConsole => "Paper console",
            Self::ModularBay => "Modular bay",
            Self::Broadcast => "Broadcast",
            Self::MinimalLine => "Minimal line",
            Self::MeterBridge => "Meter bridge",
            Self::SoftTiles => "Soft tiles",
            Self::UtilityGrid => "Utility grid",
            Self::StagePanel => "Stage panel",
        }
    }
    fn philosophy(self) -> &'static str {
        match self {
            Self::StudioRack => "Hardware-inspired stacked rack units",
            Self::NeonDeck => "High-energy club deck with glowing focus rails",
            Self::PaperConsole => "Editorial, bright control sheet with strong labels",
            Self::ModularBay => "Independent patchable modules and connector rhythm",
            Self::Broadcast => "Broadcast desk with status strip and large transport",
            Self::MinimalLine => "Sparse typography and hairline control geometry",
            Self::MeterBridge => "Instrument panel organized around signal meters",
            Self::SoftTiles => "Friendly rounded touch-sized control surfaces",
            Self::UtilityGrid => "Dense aligned service-console grid",
            Self::StagePanel => "Large, high-contrast performance-stage controls",
        }
    }
}

impl Default for Concept {
    fn default() -> Self {
        Self::StudioRack
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardLayout {
    Rack,
    Neon,
    Editorial,
    Module,
    Broadcast,
    Minimal,
    Meter,
    Soft,
    Grid,
    Stage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnobMotif {
    Bezel,
    Arc,
    Scale,
    Pins,
    OnAir,
    Line,
    Meter,
    Soft,
    Grid,
    Stage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThumbMotif {
    Rect,
    Pill,
    Paper,
    Module,
    Broadcast,
    Line,
    Meter,
    Soft,
    Grid,
    Stage,
}

const CARD_LAYOUTS: [CardLayout; 10] = [
    CardLayout::Rack,
    CardLayout::Neon,
    CardLayout::Editorial,
    CardLayout::Module,
    CardLayout::Broadcast,
    CardLayout::Minimal,
    CardLayout::Meter,
    CardLayout::Soft,
    CardLayout::Grid,
    CardLayout::Stage,
];
const CARD_RADII: [f32; 10] = [4.0, 8.0, 0.0, 6.0, 3.0, 0.0, 2.0, 18.0, 1.0, 2.0];
const CARD_MARGINS: [i8; 10] = [12, 9, 16, 8, 10, 4, 8, 14, 6, 18];
const CARD_STROKES: [f32; 10] = [2.0, 1.0, 1.0, 3.0, 4.0, 0.5, 2.0, 1.0, 1.0, 3.0];
const KNOB_SIZES: [f32; 10] = [
    100.0, 104.0, 88.0, 98.0, 108.0, 84.0, 102.0, 110.0, 92.0, 120.0,
];
const KNOB_RADII: [f32; 10] = [32.0, 38.0, 26.0, 31.0, 41.0, 28.0, 34.0, 40.0, 30.0, 46.0];
const KNOB_RINGS: [f32; 10] = [5.0, 2.0, 1.0, 7.0, 5.0, 1.0, 3.0, 8.0, 2.0, 6.0];
const KNOB_TICKS: [usize; 10] = [9, 12, 5, 8, 6, 3, 13, 7, 10, 4];
const KNOB_TICK_LENGTHS: [f32; 10] = [6.0, 10.0, 7.0, 5.0, 8.0, 4.0, 12.0, 5.0, 4.0, 14.0];
const KNOB_MOTIFS: [KnobMotif; 10] = [
    KnobMotif::Bezel,
    KnobMotif::Arc,
    KnobMotif::Scale,
    KnobMotif::Pins,
    KnobMotif::OnAir,
    KnobMotif::Line,
    KnobMotif::Meter,
    KnobMotif::Soft,
    KnobMotif::Grid,
    KnobMotif::Stage,
];
const FADER_SIZES: [[f32; 2]; 10] = [
    [64.0, 138.0],
    [70.0, 146.0],
    [52.0, 124.0],
    [78.0, 142.0],
    [66.0, 152.0],
    [48.0, 122.0],
    [60.0, 134.0],
    [74.0, 148.0],
    [56.0, 138.0],
    [84.0, 160.0],
];
const FADER_TRACKS: [f32; 10] = [6.0, 3.0, 2.0, 8.0, 4.0, 1.0, 4.0, 7.0, 2.0, 8.0];
const FADER_TICKS: [usize; 10] = [9, 11, 5, 9, 7, 3, 13, 7, 15, 5];
const THUMB_SIZES: [[f32; 2]; 10] = [
    [40.0, 14.0],
    [32.0, 12.0],
    [28.0, 10.0],
    [46.0, 18.0],
    [38.0, 20.0],
    [22.0, 8.0],
    [30.0, 12.0],
    [28.0, 28.0],
    [24.0, 10.0],
    [48.0, 24.0],
];
const THUMB_MOTIFS: [ThumbMotif; 10] = [
    ThumbMotif::Rect,
    ThumbMotif::Pill,
    ThumbMotif::Paper,
    ThumbMotif::Module,
    ThumbMotif::Broadcast,
    ThumbMotif::Line,
    ThumbMotif::Meter,
    ThumbMotif::Soft,
    ThumbMotif::Grid,
    ThumbMotif::Stage,
];
const BUTTON_SIZES: [[f32; 2]; 10] = [
    [76.0, 44.0],
    [84.0, 50.0],
    [72.0, 40.0],
    [68.0, 44.0],
    [90.0, 56.0],
    [54.0, 40.0],
    [64.0, 42.0],
    [88.0, 58.0],
    [62.0, 40.0],
    [100.0, 66.0],
];
const BUTTON_CORNERS: [u8; 10] = [2, 10, 0, 4, 3, 0, 1, 18, 1, 2];

impl Concept {
    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComponentTab {
    #[default]
    RotaryKnob,
    ChannelFader,
    Transport,
}

impl ComponentTab {
    const ALL: [Self; 3] = [Self::RotaryKnob, Self::ChannelFader, Self::Transport];

    fn label(self) -> &'static str {
        match self {
            Self::RotaryKnob => "Rotary knob",
            Self::ChannelFader => "Channel fader",
            Self::Transport => "Play/Cue buttons",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransportAction {
    Play,
    Cue,
}

pub struct GalleryState {
    pub selected: Concept,
    pub component: ComponentTab,
    minimal_line_view: bool,
    minimal_line_filter: f32,
    minimal_line_filter_drag_angle: Option<f32>,
    minimal_line_channel: f32,
    minimal_line_filter_id: Option<egui::Id>,
    minimal_line_filter_rect: Option<egui::Rect>,
    minimal_line_channel_id: Option<egui::Id>,
    minimal_line_channel_rect: Option<egui::Rect>,
    values: [f32; Concept::ALL.len()],
    last_transport: Option<(Concept, TransportAction)>,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            selected: Concept::default(),
            component: ComponentTab::default(),
            minimal_line_view: false,
            minimal_line_filter: 0.5,
            minimal_line_filter_drag_angle: None,
            minimal_line_channel: 0.5,
            minimal_line_filter_id: None,
            minimal_line_filter_rect: None,
            minimal_line_channel_id: None,
            minimal_line_channel_rect: None,
            values: [0.5; Concept::ALL.len()],
            last_transport: None,
        }
    }
}

fn component_tabs(ui: &mut egui::Ui, state: &mut GalleryState) -> Vec<egui::Response> {
    let mut responses = Vec::with_capacity(ComponentTab::ALL.len());
    ui.horizontal_wrapped(|ui| {
        for component in ComponentTab::ALL {
            responses.push(ui.selectable_value(&mut state.component, component, component.label()));
        }
    });
    let focused_id = ui.ctx().memory(|memory| memory.focused());
    let focused = responses
        .iter()
        .position(|response| Some(response.id) == focused_id);
    let direction = ui.input(|input| {
        if input.key_pressed(egui::Key::ArrowRight) {
            Some(1_isize)
        } else if input.key_pressed(egui::Key::ArrowLeft) {
            Some(-1_isize)
        } else {
            None
        }
    });
    if let (true, Some(direction)) = (focused.is_some(), direction) {
        let length = ComponentTab::ALL.len();
        let current = state.component as isize;
        let next = (current + direction).rem_euclid(length as isize) as usize;
        state.component = ComponentTab::ALL[next];
        ui.ctx()
            .memory_mut(|memory| memory.request_focus(responses[next].id));
    }
    responses
}

pub fn show(ui: &mut egui::Ui, state: &mut GalleryState, tokens: ColorTokens) -> egui::Rect {
    show_with_card_bounds(ui, state, tokens, |_| {})
}

fn show_with_card_bounds(
    ui: &mut egui::Ui,
    state: &mut GalleryState,
    tokens: ColorTokens,
    mut record_card: impl FnMut(egui::Rect),
) -> egui::Rect {
    ui.heading("Control concepts · compare by component");
    ui.label(
        "Compare the same control across ten design concepts. Preview only; no audio is connected.",
    );
    ui.horizontal(|ui| {
        if ui
            .selectable_label(!state.minimal_line_view, "Compare all ten")
            .clicked()
        {
            state.minimal_line_view = false;
        }
        if ui
            .selectable_label(
                state.minimal_line_view,
                "Minimal Line · interactive preview",
            )
            .clicked()
        {
            state.minimal_line_view = true;
        }
    });
    ui.separator();
    if state.minimal_line_view {
        show_minimal_line(ui, state, tokens);
        return ui.min_rect();
    }
    component_tabs(ui, state);
    ui.separator();

    let gallery_rect = ui.available_rect_before_wrap();
    let compact = ui.available_width() < 680.0;
    egui::ScrollArea::vertical().show(ui, |ui| {
        if compact {
            for concept in Concept::ALL {
                record_card(show_concept_card(ui, concept, state, tokens));
            }
        } else {
            for pair in Concept::ALL.chunks(2) {
                ui.columns(2, |columns| {
                    for (index, concept) in pair.iter().copied().enumerate() {
                        record_card(show_concept_card(
                            &mut columns[index],
                            concept,
                            state,
                            tokens,
                        ));
                    }
                });
                ui.add_space(6.0);
            }
        }
    });
    gallery_rect
}

fn show_minimal_line(ui: &mut egui::Ui, state: &mut GalleryState, tokens: ColorTokens) {
    ui.heading("Minimal Line");
    ui.label("Local interaction preview · no audio is connected.");
    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.vertical(|ui| {
            ui.small("FILTER");
            let response = draw_minimal_rotary(
                ui,
                &mut state.minimal_line_filter,
                &mut state.minimal_line_filter_drag_angle,
                tokens,
            );
            ui.label(format!("{}%", (state.minimal_line_filter * 100.0).round()));
            state.minimal_line_filter_id = Some(response.id);
            state.minimal_line_filter_rect = Some(response.rect);
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.small("CHANNEL 01");
            let response = draw_fader(
                ui,
                Concept::MinimalLine,
                &mut state.minimal_line_channel,
                tokens,
            );
            state.minimal_line_channel_id = Some(response.id);
            state.minimal_line_channel_rect = Some(response.rect);
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.small("TRANSPORT · DEMO ONLY");
            let _ = draw_transport(ui, Concept::MinimalLine, state, tokens);
        });
    });
}

fn concept_selector(ui: &mut egui::Ui, concept: Concept, selected: bool) -> egui::Response {
    ui.selectable_label(selected, concept.name())
        .on_hover_text(concept.philosophy())
}

fn draw_card_header(
    ui: &mut egui::Ui,
    concept: Concept,
    selected: bool,
    tokens: ColorTokens,
) -> egui::Response {
    match CARD_LAYOUTS[concept.index()] {
        CardLayout::Rack => {
            ui.horizontal(|ui| {
                ui.small("CH 01");
                concept_selector(ui, concept, selected)
            })
            .inner
        }
        CardLayout::Neon => {
            ui.horizontal(|ui| {
                ui.colored_label(tokens.secondary, "● LIVE");
                concept_selector(ui, concept, selected)
            })
            .inner
        }
        CardLayout::Editorial => {
            ui.vertical(|ui| {
                ui.small("CONTROL STUDY / 03");
                let response = concept_selector(ui, concept, selected);
                ui.small(concept.philosophy());
                response
            })
            .inner
        }
        CardLayout::Module => {
            egui::Frame::new()
                .fill(tokens.background)
                .stroke(Stroke::new(1.0_f32, tokens.border))
                .corner_radius(4.0)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.small("PATCH BAY A");
                        concept_selector(ui, concept, selected)
                    })
                    .inner
                })
                .inner
        }
        CardLayout::Broadcast => {
            ui.horizontal(|ui| {
                ui.colored_label(tokens.success, "ON AIR");
                concept_selector(ui, concept, selected)
            })
            .inner
        }
        CardLayout::Minimal => concept_selector(ui, concept, selected),
        CardLayout::Meter => {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(34.0, 16.0), egui::Sense::hover());
                for bar in 0..4 {
                    let height = 4.0 + bar as f32 * 3.0;
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + bar as f32 * 8.0, rect.bottom() - height),
                        egui::vec2(5.0, height),
                    );
                    ui.painter().rect_filled(bar_rect, 1.0, tokens.success);
                }
                concept_selector(ui, concept, selected)
            })
            .inner
        }
        CardLayout::Soft => {
            egui::Frame::new()
                .fill(tokens.background)
                .corner_radius(12.0)
                .inner_margin(8)
                .show(ui, |ui| {
                    let response = concept_selector(ui, concept, selected);
                    ui.small("Soft-touch surface");
                    response
                })
                .inner
        }
        CardLayout::Grid => {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.small("CH 01  /  BUS A");
                    concept_selector(ui, concept, selected)
                })
                .inner
            })
            .inner
        }
        CardLayout::Stage => {
            ui.vertical_centered(|ui| {
                ui.small("PERFORMANCE CONTROL");
                concept_selector(ui, concept, selected)
            })
            .inner
        }
    }
}

fn show_concept_card(
    ui: &mut egui::Ui,
    concept: Concept,
    state: &mut GalleryState,
    tokens: ColorTokens,
) -> egui::Rect {
    let selected = state.selected == concept;
    let index = concept.index();
    let layout = CARD_LAYOUTS[index];
    let border = if selected {
        concept_accent(concept, tokens)
    } else {
        tokens.border
    };
    let component = state.component;
    let response = egui::Frame::new()
        .fill(tokens.surface)
        .stroke(Stroke::new(CARD_STROKES[index], border))
        .corner_radius(CARD_RADII[index])
        .inner_margin(egui::Margin::same(CARD_MARGINS[index]))
        .show(ui, |ui| {
            let response = draw_card_header(ui, concept, selected, tokens);
            if response.clicked() {
                state.selected = concept;
            }
            if !matches!(layout, CardLayout::Minimal | CardLayout::Soft) {
                ui.separator();
            }
            match component {
                ComponentTab::RotaryKnob => {
                    let _ = draw_knob(ui, concept, &mut state.values[index], tokens);
                }
                ComponentTab::ChannelFader => {
                    let _ = draw_fader(ui, concept, &mut state.values[index], tokens);
                }
                ComponentTab::Transport => {
                    draw_transport(ui, concept, state, tokens);
                }
            }
        });
    response.response.rect
}

fn concept_accent(concept: Concept, tokens: ColorTokens) -> Color32 {
    match concept {
        Concept::NeonDeck => tokens.secondary,
        Concept::MeterBridge => tokens.success,
        Concept::PaperConsole => tokens.text,
        _ => tokens.primary,
    }
}

fn draw_knob_motif(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    motif: KnobMotif,
    tokens: ColorTokens,
) {
    match motif {
        KnobMotif::Bezel => {
            painter.circle_stroke(center, radius + 8.0, Stroke::new(1.5_f32, tokens.border));
        }
        KnobMotif::Arc => {
            for segment in 0..6 {
                let start = -2.5 + segment as f32 * 0.32;
                let end = start + 0.22;
                let point =
                    |angle: f32| center + egui::vec2(angle.cos(), angle.sin()) * (radius + 7.0);
                painter.line_segment(
                    [point(start), point(end)],
                    Stroke::new(3.0_f32, tokens.secondary),
                );
            }
        }
        KnobMotif::Scale => {
            for side in [-1.0_f32, 1.0] {
                let y = center.y + side * radius * 0.55;
                painter.line_segment(
                    [
                        egui::pos2(center.x - radius - 10.0, y),
                        egui::pos2(center.x - radius - 3.0, y),
                    ],
                    Stroke::new(1.0_f32, tokens.text),
                );
                painter.line_segment(
                    [
                        egui::pos2(center.x + radius + 3.0, y),
                        egui::pos2(center.x + radius + 10.0, y),
                    ],
                    Stroke::new(1.0_f32, tokens.text),
                );
            }
        }
        KnobMotif::Pins => {
            for offset in [
                egui::vec2(-radius * 0.75, -radius * 0.75),
                egui::vec2(radius * 0.75, -radius * 0.75),
                egui::vec2(-radius * 0.75, radius * 0.75),
                egui::vec2(radius * 0.75, radius * 0.75),
            ] {
                painter.rect_filled(
                    egui::Rect::from_center_size(center + offset, egui::vec2(4.0, 4.0)),
                    1.0,
                    tokens.muted_text,
                );
            }
        }
        KnobMotif::OnAir => {
            let badge = egui::Rect::from_center_size(
                center + egui::vec2(radius + 7.0, -radius - 2.0),
                egui::vec2(14.0, 6.0),
            );
            painter.rect_filled(badge, 2.0, tokens.success);
        }
        KnobMotif::Line => {
            painter.line_segment(
                [
                    center + egui::vec2(-radius - 9.0, 0.0),
                    center + egui::vec2(radius + 9.0, 0.0),
                ],
                Stroke::new(1.0_f32, tokens.muted_text),
            );
        }
        KnobMotif::Meter => {
            for mark in 0..5 {
                let mark_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        center.x + radius + 7.0,
                        center.y + (mark as f32 - 2.0) * 5.0,
                    ),
                    egui::vec2(8.0, 2.0),
                );
                painter.rect_filled(mark_rect, 0.0, tokens.success);
            }
        }
        KnobMotif::Soft => {
            painter.circle_stroke(
                center,
                radius * 0.62,
                Stroke::new(3.0_f32, tokens.secondary),
            );
        }
        KnobMotif::Grid => {
            for offset in [-radius - 7.0, radius + 7.0] {
                painter.line_segment(
                    [
                        egui::pos2(center.x + offset, center.y - 4.0),
                        egui::pos2(center.x + offset, center.y + 4.0),
                    ],
                    Stroke::new(1.0_f32, tokens.muted_text),
                );
                painter.line_segment(
                    [
                        egui::pos2(center.x - 4.0, center.y + offset),
                        egui::pos2(center.x + 4.0, center.y + offset),
                    ],
                    Stroke::new(1.0_f32, tokens.muted_text),
                );
            }
        }
        KnobMotif::Stage => {
            painter.circle_stroke(center, radius + 10.0, Stroke::new(3.0_f32, tokens.primary));
        }
    }
}

const MINIMAL_KNOB_START: f32 = -std::f32::consts::FRAC_PI_2 - 3.0 * std::f32::consts::FRAC_PI_4;
const MINIMAL_KNOB_SWEEP: f32 = 3.0 * std::f32::consts::FRAC_PI_2;

fn minimal_knob_angle(value: f32) -> f32 {
    MINIMAL_KNOB_START + value.clamp(0.0, 1.0) * MINIMAL_KNOB_SWEEP
}

fn minimal_knob_value(angle: f32) -> f32 {
    let normalized = (angle - MINIMAL_KNOB_START).rem_euclid(std::f32::consts::TAU);
    if normalized <= MINIMAL_KNOB_SWEEP {
        normalized / MINIMAL_KNOB_SWEEP
    } else {
        // A static pointer position in the dead sector resolves to its nearest
        // endpoint. Held drags use `minimal_knob_drag_value` to retain the
        // endpoint reached before entering the sector instead of wrapping.
        let dead_sector_progress = normalized - MINIMAL_KNOB_SWEEP;
        if dead_sector_progress <= (std::f32::consts::TAU - MINIMAL_KNOB_SWEEP) * 0.5 {
            1.0
        } else {
            0.0
        }
    }
}

fn minimal_knob_drag_value(angle: f32, previous_angle: f32, current: f32) -> f32 {
    let delta = (angle - previous_angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    (current + delta / MINIMAL_KNOB_SWEEP).clamp(0.0, 1.0)
}

fn draw_minimal_rotary(
    ui: &mut egui::Ui,
    value: &mut f32,
    drag_angle: &mut Option<f32>,
    tokens: ColorTokens,
) -> egui::Response {
    let (_, rect) = ui.allocate_space(egui::vec2(76.0, 76.0));
    let response = ui.interact(
        rect,
        egui::Id::new("minimal-line-filter"),
        egui::Sense::click_and_drag(),
    );
    let center = rect.center();
    let radius = 25.0;
    if let Some(pointer) = response.interact_pointer_pos() {
        let angle = (pointer.y - center.y).atan2(pointer.x - center.x);
        if response.drag_started() || response.dragged() {
            if let Some(previous_angle) = *drag_angle {
                *value = minimal_knob_drag_value(angle, previous_angle, *value);
            }
            *drag_angle = Some(angle);
        } else if response.clicked() {
            *value = minimal_knob_value(angle);
        } else if response.hovered() && ui.input(|input| input.pointer.primary_down()) {
            *drag_angle = Some(angle);
        }
    }
    if response.drag_stopped() {
        *drag_angle = None;
    }
    if response.has_focus() {
        ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowRight) || input.key_pressed(egui::Key::ArrowUp) {
                *value = (*value + 0.01).clamp(0.0, 1.0);
            }
            if input.key_pressed(egui::Key::ArrowLeft) || input.key_pressed(egui::Key::ArrowDown) {
                *value = (*value - 0.01).clamp(0.0, 1.0);
            }
        });
    }
    response.widget_info(|| {
        egui::WidgetInfo::slider(true, *value as f64, "Filter value · Minimal line")
    });
    let painter = ui.painter_at(rect);
    painter.circle_filled(center, radius, tokens.background);
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0_f32,
            if response.has_focus() {
                tokens.primary
            } else {
                tokens.border
            },
        ),
    );
    for tick in 0..=6 {
        let angle = minimal_knob_angle(tick as f32 / 6.0);
        let direction = egui::vec2(angle.cos(), angle.sin());
        painter.line_segment(
            [
                center + direction * (radius + 3.0),
                center + direction * (radius + 8.0),
            ],
            Stroke::new(1.0_f32, tokens.muted_text),
        );
    }
    let angle = minimal_knob_angle(*value);
    painter.line_segment(
        [
            center,
            center + egui::vec2(angle.cos(), angle.sin()) * (radius - 6.0),
        ],
        Stroke::new(2.0_f32, tokens.primary),
    );
    response
}

fn draw_knob(
    ui: &mut egui::Ui,
    concept: Concept,
    value: &mut f32,
    tokens: ColorTokens,
) -> egui::Response {
    if concept == Concept::MinimalLine {
        ui.vertical(|ui| {
            let drag_id = egui::Id::new("minimal-line-gallery-knob-drag-angle");
            let mut drag_angle = ui.ctx().data(|data| data.get_temp::<f32>(drag_id));
            let response = draw_minimal_rotary(ui, value, &mut drag_angle, tokens);
            ui.ctx().data_mut(|data| {
                if let Some(angle) = drag_angle {
                    data.insert_temp(drag_id, angle);
                } else {
                    data.remove::<f32>(drag_id);
                }
            });
            ui.label(format!("{}%", (*value * 100.0).round()));
            response
        })
        .inner
    } else {
        ui.vertical(|ui| {
            let slider_response = ui.add(
                egui::Slider::new(&mut *value, 0.0..=1.0)
                    .show_value(true)
                    .text(format!("Filter value · {}", concept.name())),
            );
            let value = *value;
            ui.horizontal(|ui| {
                let index = concept.index();
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(KNOB_SIZES[index], KNOB_SIZES[index]),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                let radius = KNOB_RADII[index];
                let center = rect.center();
                let accent = concept_accent(concept, tokens);
                painter.circle_filled(center, radius, tokens.background);
                painter.circle_stroke(
                    center,
                    radius,
                    Stroke::new(KNOB_RINGS[index], tokens.border),
                );
                draw_knob_motif(&painter, center, radius, KNOB_MOTIFS[index], tokens);
                let ticks = KNOB_TICKS[index];
                for tick in 0..ticks {
                    let angle = -2.35 + tick as f32 / (ticks - 1) as f32 * 4.7;
                    let inner = center + egui::vec2(angle.cos(), angle.sin()) * (radius + 2.0);
                    let major = tick % 3 == 0;
                    let length = KNOB_TICK_LENGTHS[index] * if major { 1.0 } else { 0.65 };
                    let outer = center + egui::vec2(angle.cos(), angle.sin()) * (radius + length);
                    painter.line_segment(
                        [inner, outer],
                        Stroke::new(if major { 1.5_f32 } else { 0.75_f32 }, tokens.muted_text),
                    );
                }
                let angle = -2.35 + value.clamp(0.0, 1.0) * 4.7;
                let pointer = center + egui::vec2(angle.cos(), angle.sin()) * (radius - 6.0);
                let pointer_width = if KNOB_MOTIFS[index] == KnobMotif::Line {
                    1.0_f32
                } else if KNOB_MOTIFS[index] == KnobMotif::Stage {
                    5.0_f32
                } else {
                    3.0_f32
                };
                painter.line_segment([center, pointer], Stroke::new(pointer_width, accent));
                if KNOB_MOTIFS[index] != KnobMotif::Line {
                    painter.circle_filled(
                        center,
                        if KNOB_MOTIFS[index] == KnobMotif::Soft {
                            6.0
                        } else {
                            3.0
                        },
                        tokens.text,
                    );
                }
                ui.vertical(|ui| {
                    ui.strong("FILTER");
                    ui.label(format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round()));
                    ui.small(match concept {
                        Concept::StudioRack => "machined ring",
                        Concept::NeonDeck => "cyan pointer",
                        Concept::PaperConsole => "editorial scale",
                        Concept::ModularBay => "module dial",
                        Concept::Broadcast => "broadcast trim",
                        Concept::MinimalLine => "single stroke",
                        Concept::MeterBridge => "meter ticks",
                        Concept::SoftTiles => "soft rim",
                        Concept::UtilityGrid => "indexed dial",
                        Concept::StagePanel => "oversized face",
                    });
                });
            });
            slider_response
        })
        .inner
    }
}

fn draw_fader_thumb(
    painter: &egui::Painter,
    rect: egui::Rect,
    motif: ThumbMotif,
    accent: Color32,
    tokens: ColorTokens,
) {
    let center = rect.center();
    match motif {
        ThumbMotif::Rect => {
            painter.rect_filled(rect, 2.0, accent);
        }
        ThumbMotif::Pill => {
            painter.rect_filled(rect, rect.height() * 0.5, accent);
        }
        ThumbMotif::Paper => {
            painter.rect_filled(rect, 0.0, tokens.text);
            for side in [-1.0_f32, 1.0] {
                painter.line_segment(
                    [
                        center + egui::vec2(side * (rect.width() * 0.5 + 4.0), -4.0),
                        center + egui::vec2(side * (rect.width() * 0.5 + 4.0), 4.0),
                    ],
                    Stroke::new(1.0_f32, tokens.text),
                );
            }
        }
        ThumbMotif::Module => {
            painter.rect_filled(rect, 2.0, accent);
            for side in [-1.0_f32, 1.0] {
                let tab = egui::Rect::from_center_size(
                    center + egui::vec2(side * (rect.width() * 0.5 + 3.0), 0.0),
                    egui::vec2(6.0, rect.height() * 0.55),
                );
                painter.rect_filled(tab, 1.0, tokens.text);
            }
        }
        ThumbMotif::Broadcast => {
            painter.rect_filled(rect, 1.0, accent);
            painter.circle_filled(
                center + egui::vec2(rect.width() * 0.32, 0.0),
                2.0,
                tokens.success,
            );
        }
        ThumbMotif::Line => {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), center.y),
                    egui::pos2(rect.right(), center.y),
                ],
                Stroke::new(2.0_f32, accent),
            );
        }
        ThumbMotif::Meter => {
            painter.rect_filled(rect, 1.0, accent);
            for step in 0..3 {
                let y = rect.top() + 3.0 + step as f32 * 3.0;
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 4.0, y),
                        egui::pos2(rect.right() - 4.0, y),
                    ],
                    Stroke::new(1.0_f32, tokens.background),
                );
            }
        }
        ThumbMotif::Soft => {
            painter.circle_filled(center, rect.width().min(rect.height()) * 0.5, accent);
        }
        ThumbMotif::Grid => {
            painter.rect_filled(rect, 0.0, accent);
            painter.line_segment(
                [
                    egui::pos2(center.x, rect.top()),
                    egui::pos2(center.x, rect.bottom()),
                ],
                Stroke::new(1.0_f32, tokens.background),
            );
        }
        ThumbMotif::Stage => {
            painter.rect_filled(rect, 2.0, accent);
            for side in [-1.0_f32, 1.0] {
                painter.line_segment(
                    [
                        center + egui::vec2(side * (rect.width() * 0.34), -4.0),
                        center + egui::vec2(side * (rect.width() * 0.34), 4.0),
                    ],
                    Stroke::new(1.5_f32, tokens.text),
                );
            }
        }
    }
}

fn draw_fader(
    ui: &mut egui::Ui,
    concept: Concept,
    value: &mut f32,
    tokens: ColorTokens,
) -> egui::Response {
    if concept == Concept::MinimalLine {
        let [_, height] = FADER_SIZES[concept.index()];
        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, height), egui::Sense::hover());
        let response = ui.interact(
            rect,
            egui::Id::new("minimal-line-channel"),
            egui::Sense::click_and_drag(),
        );
        if (response.dragged() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            *value = ((rect.bottom() - pointer.y) / rect.height()).clamp(0.0, 1.0);
        }
        if response.has_focus() {
            ui.input(|input| {
                if input.key_pressed(egui::Key::ArrowUp) || input.key_pressed(egui::Key::ArrowRight)
                {
                    *value = (*value + 0.01).clamp(0.0, 1.0);
                }
                if input.key_pressed(egui::Key::ArrowDown)
                    || input.key_pressed(egui::Key::ArrowLeft)
                {
                    *value = (*value - 0.01).clamp(0.0, 1.0);
                }
            });
        }
        response.widget_info(|| {
            egui::WidgetInfo::slider(true, *value as f64, "Channel level · Minimal line")
        });
        let painter = ui.painter_at(rect);
        let track =
            egui::Rect::from_center_size(rect.center(), egui::vec2(2.0, rect.height() - 18.0));
        painter.rect_filled(track, 1.0, tokens.border);
        let y = egui::lerp(track.bottom()..=track.top(), *value);
        painter.circle_filled(egui::pos2(rect.center().x, y), 7.0, tokens.surface);
        painter.circle_stroke(
            egui::pos2(rect.center().x, y),
            7.0,
            Stroke::new(
                1.5_f32,
                if response.has_focus() {
                    tokens.primary
                } else {
                    tokens.secondary
                },
            ),
        );
        return response;
    }
    ui.vertical(|ui| {
        let slider_response = ui.add(
            egui::Slider::new(&mut *value, 0.0..=1.0)
                .show_value(true)
                .text(format!("Channel level · {}", concept.name())),
        );
        let value = *value;
        ui.horizontal(|ui| {
            let index = concept.index();
            let [width, height] = FADER_SIZES[index];
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let x = rect.center().x;
            let accent = concept_accent(concept, tokens);
            painter.line_segment(
                [
                    egui::pos2(x, rect.top() + 8.0),
                    egui::pos2(x, rect.bottom() - 8.0),
                ],
                Stroke::new(FADER_TRACKS[index], tokens.border),
            );
            let ticks = FADER_TICKS[index];
            for tick in 0..ticks {
                let y =
                    rect.top() + 10.0 + tick as f32 * (rect.height() - 20.0) / (ticks - 1) as f32;
                let half = if tick % 2 == 0 { 11.0 } else { 7.0 };
                painter.line_segment(
                    [egui::pos2(x - half, y), egui::pos2(x + half, y)],
                    Stroke::new(1.0_f32, tokens.muted_text),
                );
            }
            let thumb_y = egui::lerp(
                (rect.bottom() - 14.0)..=(rect.top() + 14.0),
                value.clamp(0.0, 1.0),
            );
            let [thumb_width, thumb_height] = THUMB_SIZES[index];
            let thumb = egui::Rect::from_center_size(
                egui::pos2(x, thumb_y),
                egui::vec2(thumb_width, thumb_height),
            );
            draw_fader_thumb(&painter, thumb, THUMB_MOTIFS[index], accent, tokens);
            ui.vertical(|ui| {
                ui.strong("CHANNEL 01");
                ui.label(format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round()));
                ui.small(match concept {
                    Concept::StudioRack => "rack slot",
                    Concept::NeonDeck => "lit throw",
                    Concept::PaperConsole => "marked scale",
                    Concept::ModularBay => "wide cap",
                    Concept::Broadcast => "long throw",
                    Concept::MinimalLine => "hairline rail",
                    Concept::MeterBridge => "metered level",
                    Concept::SoftTiles => "round handle",
                    Concept::UtilityGrid => "indexed steps",
                    Concept::StagePanel => "large grip",
                });
            });
        });
        slider_response
    })
    .inner
}

fn transport_button(
    ui: &mut egui::Ui,
    concept: Concept,
    action: TransportAction,
    label: &str,
    icon: &str,
    state: &mut GalleryState,
    tokens: ColorTokens,
) -> egui::Response {
    let index = concept.index();
    let layout = CARD_LAYOUTS[index];
    let [mut width, mut height] = BUTTON_SIZES[index];
    if layout == CardLayout::Minimal {
        width = 44.0;
        height = 40.0;
    } else if layout == CardLayout::Broadcast && action == TransportAction::Play {
        width *= 1.15;
        height *= 1.05;
    } else if layout == CardLayout::Stage {
        height *= 1.1;
    }
    let active = state.last_transport == Some((concept, action));
    let accent = concept_accent(concept, tokens);
    let highlighted =
        active || (layout == CardLayout::Broadcast && action == TransportAction::Play);
    let fill = if highlighted {
        accent.linear_multiply(0.35)
    } else {
        tokens.background
    };
    let visible_label = if layout == CardLayout::Minimal {
        icon.to_owned()
    } else {
        format!("{icon} {label}")
    };
    let button = egui::Button::new(egui::RichText::new(visible_label).color(tokens.text))
        .fill(fill)
        .stroke(Stroke::new(CARD_STROKES[index], accent))
        .corner_radius(BUTTON_CORNERS[index]);
    let response = ui.add_sized([width, height], button);
    let interaction_feedback = if response.is_pointer_button_down_on() {
        Some((tokens.secondary, 3.0_f32))
    } else if response.has_focus() {
        Some((tokens.primary, 2.0_f32))
    } else if response.hovered() {
        Some((tokens.secondary, 1.5_f32))
    } else {
        None
    };
    if let Some((color, width)) = interaction_feedback {
        ui.painter().rect_stroke(
            response.rect.shrink(2.0),
            BUTTON_CORNERS[index],
            Stroke::new(width, color),
            egui::StrokeKind::Inside,
        );
    }
    let accessible_name = format!("{label}, {}", concept.name());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, accessible_name.clone())
    });
    if response.clicked() {
        state.last_transport = Some((concept, action));
    }
    if layout == CardLayout::Minimal {
        ui.small(label);
    }
    response.on_hover_text(accessible_name)
}

fn draw_transport(
    ui: &mut egui::Ui,
    concept: Concept,
    state: &mut GalleryState,
    tokens: ColorTokens,
) -> Vec<egui::Response> {
    let layout = CARD_LAYOUTS[concept.index()];
    let actions = [
        (TransportAction::Play, "Play", "▶"),
        (TransportAction::Cue, "Cue", "●"),
    ];
    let mut responses = Vec::with_capacity(actions.len());
    match layout {
        CardLayout::Editorial | CardLayout::Stage => ui.vertical(|ui| {
            for (action, label, icon) in actions {
                responses.push(transport_button(
                    ui, concept, action, label, icon, state, tokens,
                ));
            }
        }),
        CardLayout::Grid => egui::Grid::new(("transport-grid", concept.index()))
            .num_columns(2)
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                for (action, label, icon) in actions {
                    responses.push(transport_button(
                        ui, concept, action, label, icon, state, tokens,
                    ));
                    ui.end_row();
                }
            }),
        CardLayout::Module => ui.horizontal_wrapped(|ui| {
            for (action, label, icon) in actions {
                egui::Frame::new()
                    .fill(tokens.background)
                    .inner_margin(3)
                    .show(ui, |ui| {
                        responses.push(transport_button(
                            ui, concept, action, label, icon, state, tokens,
                        ));
                    });
            }
        }),
        _ => ui.horizontal_wrapped(|ui| {
            for (action, label, icon) in actions {
                responses.push(transport_button(
                    ui, concept, action, label, icon, state, tokens,
                ));
            }
        }),
    };
    let result = match state.last_transport {
        Some((selected, action)) if selected == concept => Some(match action {
            TransportAction::Play => "Play",
            TransportAction::Cue => "Cue",
        }),
        _ => None,
    };
    ui.small(
        result.map_or("Idle · hover · focus · pressed", |action| match action {
            "Play" => "Play selected",
            _ => "Cue selected",
        }),
    );
    responses
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn every_knob_and_fader_preview_exposes_its_own_accessible_value_control() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState::default();
        for (component, prefix) in [
            (ComponentTab::RotaryKnob, "Filter value · "),
            (ComponentTab::ChannelFader, "Channel level · "),
        ] {
            state.component = component;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 2000.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut state, ThemeMode::TokyoNight.tokens());
                    });
                },
            );
            let names = output
                .platform_output
                .accesskit_update
                .expect("accessibility tree")
                .nodes
                .into_iter()
                .map(|(_, node)| {
                    node.value()
                        .or_else(|| node.label())
                        .unwrap_or_default()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            for concept in Concept::ALL {
                let accessible_name = format!("{prefix}{}", concept.name());
                assert!(
                    names.iter().any(|name| *name == accessible_name),
                    "{} tab lacks {accessible_name:?}",
                    component.label()
                );
            }
        }
    }

    fn geometry_signature(
        component: ComponentTab,
        concept: Concept,
    ) -> Vec<(String, i32, i32, i32, i32)> {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        state.component = component;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| match component {
                    ComponentTab::RotaryKnob => {
                        let _ = draw_knob(
                            ui,
                            concept,
                            &mut state.values[concept as usize],
                            ThemeMode::TokyoNight.tokens(),
                        );
                    }
                    ComponentTab::ChannelFader => {
                        let _ = draw_fader(
                            ui,
                            concept,
                            &mut state.values[concept as usize],
                            ThemeMode::TokyoNight.tokens(),
                        );
                    }
                    ComponentTab::Transport => {
                        let _ =
                            draw_transport(ui, concept, &mut state, ThemeMode::TokyoNight.tokens());
                    }
                });
            },
        );
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| {
                if matches!(clipped.shape, egui::Shape::Text(_) | egui::Shape::Noop) {
                    return None;
                }
                let rect = clipped.shape.visual_bounding_rect();
                let quantize = |coordinate: f32| (coordinate * 2.0).round() as i32;
                Some((
                    format!("{:?}", std::mem::discriminant(&clipped.shape)),
                    quantize(rect.left()),
                    quantize(rect.top()),
                    quantize(rect.right()),
                    quantize(rect.bottom()),
                ))
            })
            .collect()
    }

    #[test]
    fn each_component_uses_distinct_geometry_for_all_ten_concepts() {
        for component in ComponentTab::ALL {
            let mut signatures = std::collections::HashSet::new();
            for concept in Concept::ALL {
                let signature = geometry_signature(component, concept);
                assert!(
                    signatures.insert(signature),
                    "{} reused another concept's geometry: {:?}",
                    component.label(),
                    concept
                );
            }
        }
    }

    #[test]
    fn keyboard_adjusts_a_named_per_concept_knob_control() {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 1600.0));
        let slider_id = std::cell::Cell::new(None);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    slider_id.set(Some(
                        draw_knob(
                            ui,
                            Concept::StudioRack,
                            &mut state.values[Concept::StudioRack as usize],
                            ThemeMode::TokyoNight.tokens(),
                        )
                        .id,
                    ));
                });
            },
        );
        ctx.memory_mut(|memory| memory.request_focus(slider_id.get().expect("slider response id")));
        let focused = std::cell::Cell::new(false);
        let initial_value = state.values[Concept::StudioRack as usize];
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    focused.set(
                        draw_knob(
                            ui,
                            Concept::StudioRack,
                            &mut state.values[Concept::StudioRack as usize],
                            ThemeMode::TokyoNight.tokens(),
                        )
                        .has_focus(),
                    );
                });
            },
        );
        assert!(focused.get(), "slider should retain keyboard focus");
        assert!(state.values[Concept::StudioRack as usize] > initial_value);
        assert_eq!(state.values[Concept::NeonDeck as usize], 0.5);
    }

    #[test]
    fn component_comparison_view_exposes_three_tabs_and_all_ten_concepts() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &mut state, ThemeMode::TokyoNight.tokens());
                });
            },
        );
        let update = output
            .platform_output
            .accesskit_update
            .expect("accessibility tree");
        let names = update
            .nodes
            .into_iter()
            .map(|(_, node)| {
                node.value()
                    .or_else(|| node.label())
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<Vec<_>>();

        for tab in ["Rotary knob", "Channel fader", "Play/Cue buttons"] {
            assert!(
                names.iter().any(|name| *name == tab),
                "missing component tab {tab:?}"
            );
        }
        for concept in Concept::ALL {
            assert!(
                names.iter().any(|name| *name == concept.name()),
                "missing concept preview {:?}",
                concept.name()
            );
        }

        for component in ComponentTab::ALL {
            state.component = component;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 1600.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut state, ThemeMode::TokyoNight.tokens());
                    });
                },
            );
            let names = output
                .platform_output
                .accesskit_update
                .expect("active component accessibility tree")
                .nodes
                .into_iter()
                .map(|(_, node)| {
                    node.value()
                        .or_else(|| node.label())
                        .unwrap_or_default()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            for concept in Concept::ALL {
                assert!(
                    names.iter().any(|name| *name == concept.name()),
                    "{} tab omitted {:?}",
                    component.label(),
                    concept.name()
                );
            }
        }
    }

    #[test]
    fn selection_is_keyboard_navigable_and_switches_concept() {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        let rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let run = |ctx: &egui::Context, state: &mut GalleryState, input| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = ui.selectable_value(
                        &mut state.selected,
                        Concept::NeonDeck,
                        Concept::NeonDeck.name(),
                    );
                    rect.set(response.rect);
                });
            })
        };
        let _ = run(&ctx, &mut state, egui::RawInput::default());
        let pos = rect.get().center();
        let _ = run(
            &ctx,
            &mut state,
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
        );
        assert_eq!(state.selected, Concept::NeonDeck);
        assert_eq!(Concept::ALL.len(), 10);
    }

    #[test]
    fn minimal_knob_midpoint_is_twelve_oclock_and_endpoints_clamp() {
        assert!((minimal_knob_angle(0.5) + std::f32::consts::FRAC_PI_2).abs() < 0.0001);
        assert_eq!(minimal_knob_angle(-1.0), minimal_knob_angle(0.0));
        assert_eq!(minimal_knob_angle(2.0), minimal_knob_angle(1.0));
        assert_eq!(minimal_knob_value(minimal_knob_angle(0.0)), 0.0);
        assert_eq!(minimal_knob_value(minimal_knob_angle(1.0)), 1.0);
        assert_eq!(minimal_knob_value(60.0_f32.to_radians()), 1.0);
        assert_eq!(minimal_knob_value(120.0_f32.to_radians()), 0.0);
    }

    #[test]
    fn minimal_line_controls_fit_their_compact_component_bounds() {
        let ctx = egui::Context::default();
        let mut state = GalleryState {
            minimal_line_view: true,
            ..Default::default()
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 500.0));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &mut state, ThemeMode::TokyoNight.tokens());
                });
            },
        );
        let knob = state.minimal_line_filter_rect.expect("rotary bounds");
        let fader = state.minimal_line_channel_rect.expect("fader bounds");
        assert!((knob.width() / knob.height() - 1.0).abs() < 0.01);
        assert!(knob.width() <= 84.0 && knob.height() <= 84.0);
        assert!(
            fader.width() < fader.height(),
            "vertical fader response bounds: {}×{}",
            fader.width(),
            fader.height()
        );
        assert!(fader.width() <= 48.0 && fader.height() <= 122.0);
        assert!(screen.contains_rect(knob) && screen.contains_rect(fader));
    }

    #[test]
    fn minimal_line_fader_endpoint_thumb_and_focus_stroke_fit_paint_bounds() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 300.0));
        for endpoint in [0.0_f32, 1.0] {
            let value = std::cell::Cell::new(endpoint);
            let bounds = std::cell::Cell::new(None);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut current = value.get();
                        let response = draw_fader(
                            ui,
                            Concept::MinimalLine,
                            &mut current,
                            ThemeMode::TokyoNight.tokens(),
                        );
                        bounds.set(Some(response.rect));
                    });
                },
            );
            let rect = bounds.get().expect("actual MinimalLine fader bounds");
            let circles = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Circle(_) => Some(clipped.shape.visual_bounding_rect()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                circles.len(),
                2,
                "endpoint {endpoint} should paint fill and stroke circles"
            );
            for circle in circles {
                assert!(
                    rect.contains_rect(circle),
                    "endpoint {endpoint}: thumb {circle:?} exceeds fader paint bounds {rect:?}"
                );
            }
        }
    }

    #[test]
    fn minimal_line_gallery_rotary_drag_persists_across_frames() {
        let ctx = egui::Context::default();
        let value = std::cell::Cell::new(0.2_f32);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 300.0));
        let bounds = std::cell::Cell::new(None);
        let render = |events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut current = value.get();
                        let response = draw_knob(
                            ui,
                            Concept::MinimalLine,
                            &mut current,
                            ThemeMode::TokyoNight.tokens(),
                        );
                        value.set(current);
                        bounds.set(Some(response.rect));
                    });
                },
            )
        };
        let _ = render(Vec::new());
        let rect = bounds.get().expect("gallery rotary bounds");
        let center = rect.center();
        let at = |amount: f32| {
            let angle = minimal_knob_angle(amount);
            center + egui::vec2(angle.cos(), angle.sin()) * 20.0
        };
        let start = at(0.2);
        let target = at(0.65);
        let _ = render(vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ]);
        let _ = render(vec![egui::Event::PointerMoved(target)]);
        assert!(
            value.get() > 0.45,
            "gallery rotary held drag should change its value, got {}",
            value.get()
        );
        let _ = render(vec![egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
    }

    #[test]
    fn minimal_rotary_pointer_drag_updates_actual_widget() {
        let ctx = egui::Context::default();
        let mut state = GalleryState {
            minimal_line_view: true,
            minimal_line_filter: 0.2,
            ..Default::default()
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let render = |state: &mut GalleryState, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, state, ThemeMode::TokyoNight.tokens());
                    });
                },
            )
        };
        let _ = render(&mut state, Vec::new());
        let rect = state
            .minimal_line_filter_rect
            .expect("rotary response bounds");
        let center = rect.center();
        let point = |value: f32| {
            let angle = minimal_knob_angle(value);
            center + egui::vec2(angle.cos(), angle.sin()) * 20.0
        };
        let start = point(0.2);
        let target = point(0.8);
        let _ = render(
            &mut state,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = render(&mut state, vec![egui::Event::PointerMoved(target)]);
        let _ = render(
            &mut state,
            vec![egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(
            (state.minimal_line_filter - 0.8).abs() < 0.05,
            "actual rotary drag should track angle, got {}",
            state.minimal_line_filter
        );

        let at_angle = |degrees: f32| {
            let angle = degrees.to_radians();
            center + egui::vec2(angle.cos(), angle.sin()) * 20.0
        };
        // Clockwise traversal from the high endpoint must remain clamped
        // across the dead-sector midpoint instead of wrapping to zero.
        state.minimal_line_filter = 1.0;
        let high = at_angle(45.0);
        let _ = render(
            &mut state,
            vec![
                egui::Event::PointerMoved(high),
                egui::Event::PointerButton {
                    pos: high,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(89.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(91.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(135.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(160.0))]);
        assert_eq!(state.minimal_line_filter, 1.0);
        // Reversing into the same active arc should resume smoothly, not
        // switch to the opposite endpoint when crossing the gap boundary.
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(130.0))]);
        assert!(state.minimal_line_filter < 1.0 && state.minimal_line_filter > 0.85);
        let _ = render(
            &mut state,
            vec![egui::Event::PointerButton {
                pos: at_angle(130.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );

        // Repeat in the opposite direction from the low endpoint.
        state.minimal_line_filter = 0.0;
        let low = at_angle(135.0);
        let _ = render(
            &mut state,
            vec![
                egui::Event::PointerMoved(low),
                egui::Event::PointerButton {
                    pos: low,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(91.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(89.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(45.0))]);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(20.0))]);
        assert_eq!(state.minimal_line_filter, 0.0);
        let _ = render(&mut state, vec![egui::Event::PointerMoved(at_angle(50.0))]);
        assert!(state.minimal_line_filter > 0.0 && state.minimal_line_filter < 0.15);
        let _ = render(
            &mut state,
            vec![egui::Event::PointerButton {
                pos: at_angle(50.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );

        // Entering the dead sector from an interior value clamps at the
        // endpoint instead of freezing at the last interior value.
        state.minimal_line_filter = 0.98;
        let near_high = at_angle(39.6);
        let into_gap = at_angle(60.0);
        let _ = render(
            &mut state,
            vec![
                egui::Event::PointerMoved(near_high),
                egui::Event::PointerButton {
                    pos: near_high,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = render(&mut state, vec![egui::Event::PointerMoved(into_gap)]);
        assert_eq!(state.minimal_line_filter, 1.0);
        let _ = render(
            &mut state,
            vec![egui::Event::PointerButton {
                pos: into_gap,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );

        let fader = state
            .minimal_line_channel_rect
            .expect("vertical fader bounds");
        let y_for = |level: f32| fader.bottom() - level * fader.height();
        let start = egui::pos2(fader.center().x, y_for(0.2));
        let target = egui::pos2(fader.center().x, y_for(0.8));
        let _ = render(
            &mut state,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = render(&mut state, vec![egui::Event::PointerMoved(target)]);
        let _ = render(
            &mut state,
            vec![egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(
            (state.minimal_line_channel - 0.8).abs() < 0.05,
            "vertical fader drag should follow its track"
        );
    }

    #[test]
    fn minimal_line_filter_and_channel_widgets_update_independently() {
        let ctx = egui::Context::default();
        let mut state = GalleryState {
            minimal_line_view: true,
            ..Default::default()
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let render = |state: &mut GalleryState, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, state, ThemeMode::TokyoNight.tokens());
                    });
                },
            )
        };
        let arrow_right = || egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let _ = render(&mut state, Vec::new());
        let filter_id = state
            .minimal_line_filter_id
            .expect("rendered FILTER response");
        let channel_before = state.minimal_line_channel;
        ctx.memory_mut(|memory| memory.request_focus(filter_id));
        let _ = render(&mut state, vec![arrow_right()]);
        assert_ne!(
            state.minimal_line_filter, 0.5,
            "FILTER widget accepted keyboard input"
        );
        assert_eq!(state.minimal_line_channel, channel_before);

        let filter_before = state.minimal_line_filter;
        let channel_id = state
            .minimal_line_channel_id
            .expect("rendered CHANNEL response");
        ctx.memory_mut(|memory| memory.request_focus(channel_id));
        let _ = render(
            &mut state,
            vec![egui::Event::Key {
                key: egui::Key::ArrowUp,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        assert_ne!(
            state.minimal_line_channel, channel_before,
            "CHANNEL widget accepted keyboard input"
        );
        assert_eq!(state.minimal_line_filter, filter_before);
    }

    #[test]
    fn controls_expose_named_native_accessibility_widgets() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState {
            selected: Concept::StudioRack,
            ..Default::default()
        };
        let render = |state: &mut GalleryState| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, state, ThemeMode::TokyoNight.tokens());
                    });
                },
            )
        };
        let before = render(&mut state);
        let update = before
            .platform_output
            .accesskit_update
            .expect("accessibility tree");
        let nodes = update
            .nodes
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        for name in ["Filter value · Studio rack"] {
            assert!(
                nodes.values().any(|node| node.label() == Some(name)
                    || node.value() == Some(name)
                    || node.labelled_by().iter().any(|id| nodes
                        .get(id)
                        .is_some_and(|label| label.value() == Some(name)))),
                "missing accessible control {name}; nodes={nodes:?}"
            );
        }
        state.component = ComponentTab::Transport;
        let transport = render(&mut state);
        let nodes = transport
            .platform_output
            .accesskit_update
            .expect("transport accessibility tree")
            .nodes;
        for action in ["Play, Studio rack", "Cue, Studio rack"] {
            assert!(
                nodes.iter().any(|(_, node)| {
                    node.label() == Some(action) || node.value() == Some(action)
                }),
                "missing accessible transport action {action}"
            );
        }
    }

    #[test]
    fn clicking_component_tab_switches_the_comparison() {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        let rects = std::cell::RefCell::new(Vec::new());
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                *rects.borrow_mut() = component_tabs(ui, &mut state)
                    .into_iter()
                    .map(|response| response.rect)
                    .collect();
            });
        });
        let target = rects.borrow()[1].center();
        let _ = ctx.run(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(target),
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ],
                ..input()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    component_tabs(ui, &mut state);
                });
            },
        );
        assert_eq!(state.component, ComponentTab::ChannelFader);
    }

    #[test]
    fn arrow_keys_navigate_the_gallery_component_tabs() {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0));
        let first_tab_id = std::cell::Cell::new(None);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    first_tab_id.set(Some(component_tabs(ui, &mut state)[0].id));
                });
            },
        );
        ctx.memory_mut(|memory| memory.request_focus(first_tab_id.get().unwrap()));
        let press = |key, state: &mut GalleryState| {
            for pressed in [true, false] {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        events: vec![egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            component_tabs(ui, state);
                        });
                    },
                );
            }
        };
        press(egui::Key::ArrowRight, &mut state);
        assert_eq!(state.component, ComponentTab::ChannelFader);
        press(egui::Key::ArrowRight, &mut state);
        assert_eq!(state.component, ComponentTab::Transport);
        press(egui::Key::ArrowRight, &mut state);
        assert_eq!(state.component, ComponentTab::RotaryKnob);
        press(egui::Key::ArrowLeft, &mut state);
        assert_eq!(state.component, ComponentTab::Transport);
    }

    #[test]
    fn held_cue_button_paints_differently_from_hovered_cue_button() {
        let ctx = egui::Context::default();
        let mut state = GalleryState::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 400.0));
        let cue_rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let cue_id = std::cell::Cell::new(None);
        let pointer_is_held_on_cue = std::cell::Cell::new(false);
        let cue_has_focus = std::cell::Cell::new(false);
        let mut render = |events: Vec<egui::Event>, capture_pressed: bool| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let cue_responses = draw_transport(
                            ui,
                            Concept::StudioRack,
                            &mut state,
                            ThemeMode::TokyoNight.tokens(),
                        );
                        let cue = &cue_responses[1];
                        cue_rect.set(cue.rect);
                        cue_id.set(Some(cue.id));
                        if capture_pressed {
                            pointer_is_held_on_cue.set(cue.is_pointer_button_down_on());
                            cue_has_focus.set(cue.has_focus());
                        }
                    });
                },
            )
        };

        let idle = render(Vec::new(), true);
        ctx.memory_mut(|memory| memory.request_focus(cue_id.get().expect("Cue response id")));
        let focused = render(Vec::new(), true);
        assert!(cue_has_focus.get(), "focus must reach the Cue button");
        let target = cue_rect.get().center();
        let hovered = render(vec![egui::Event::PointerMoved(target)], false);
        let pressed = render(
            vec![
                egui::Event::PointerMoved(target),
                egui::Event::PointerButton {
                    pos: target,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            true,
        );
        let button_rectangles = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter(|clipped| {
                    matches!(&clipped.shape, egui::Shape::Rect(_))
                        && cue_rect
                            .get()
                            .contains(clipped.shape.visual_bounding_rect().center())
                })
                .map(|clipped| format!("{:?}", clipped.shape))
                .collect::<Vec<_>>()
        };
        assert!(
            pointer_is_held_on_cue.get(),
            "press event must reach the Cue button"
        );
        assert_ne!(button_rectangles(&idle), button_rectangles(&focused));
        assert_ne!(button_rectangles(&hovered), button_rectangles(&pressed));
    }

    #[test]
    fn cue_button_updates_its_selected_state() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState::default();
        let rects = std::cell::RefCell::new(Vec::new());
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 400.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                *rects.borrow_mut() = draw_transport(
                    ui,
                    Concept::StudioRack,
                    &mut state,
                    ThemeMode::TokyoNight.tokens(),
                )
                .into_iter()
                .map(|response| response.rect)
                .collect();
            });
        });
        let target = rects.borrow()[1].center();
        let output = ctx.run(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(target),
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ],
                ..input()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_transport(
                        ui,
                        Concept::StudioRack,
                        &mut state,
                        ThemeMode::TokyoNight.tokens(),
                    );
                });
            },
        );
        assert_eq!(
            state.last_transport,
            Some((Concept::StudioRack, TransportAction::Cue))
        );
        let nodes = output
            .platform_output
            .accesskit_update
            .expect("transport state accessibility tree")
            .nodes;
        assert!(nodes.iter().any(|(_, node)| {
            node.label() == Some("Cue selected") || node.value() == Some("Cue selected")
        }));
    }

    // CHARACTERIZATION_GREEN_NOT_TDD: egui's native Button supplies keyboard activation.
    #[test]
    fn keyboard_activates_the_named_cue_button_and_announces_selection() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 400.0));
        let cue_id = std::cell::Cell::new(None);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    cue_id.set(Some(
                        draw_transport(
                            ui,
                            Concept::StudioRack,
                            &mut state,
                            ThemeMode::TokyoNight.tokens(),
                        )[1]
                        .id,
                    ));
                });
            },
        );
        ctx.memory_mut(|memory| memory.request_focus(cue_id.get().expect("Cue response id")));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_transport(
                        ui,
                        Concept::StudioRack,
                        &mut state,
                        ThemeMode::TokyoNight.tokens(),
                    );
                });
            },
        );
        assert_eq!(
            state.last_transport,
            Some((Concept::StudioRack, TransportAction::Cue))
        );
        let nodes = output
            .platform_output
            .accesskit_update
            .expect("transport accessibility update")
            .nodes;
        assert!(nodes.iter().any(|(_, node)| {
            node.label() == Some("Cue selected") || node.value() == Some("Cue selected")
        }));
    }

    #[test]
    fn minimal_line_preview_is_named_and_accessible_without_removing_gallery() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState {
            minimal_line_view: true,
            ..Default::default()
        };
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &mut state, ThemeMode::TokyoNight.tokens());
                });
            },
        );
        let nodes = output
            .platform_output
            .accesskit_update
            .expect("accessible Minimal Line preview")
            .nodes;
        let names = nodes
            .iter()
            .flat_map(|(_, node)| [node.label(), node.value()])
            .flatten()
            .collect::<Vec<_>>();
        for expected in [
            "Filter value · Minimal line",
            "Channel level · Minimal line",
            "Play, Minimal line",
            "Cue, Minimal line",
        ] {
            assert!(names.contains(&expected), "missing {expected:?}: {names:?}");
        }
        assert!(state.minimal_line_view);
        assert_eq!(Concept::ALL.len(), 10, "comparison concepts remain intact");
    }

    #[test]
    fn cards_and_controls_fit_narrow_and_wide_layouts_in_all_tabs() {
        for width in [420.0, 1100.0] {
            for component in ComponentTab::ALL {
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                let mut state = GalleryState {
                    component,
                    ..Default::default()
                };
                let card_rects = std::cell::RefCell::new(Vec::new());
                let card_index = std::cell::Cell::new(0);
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 4000.0));
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let viewport = show_with_card_bounds(
                                ui,
                                &mut state,
                                ThemeMode::TokyoNight.tokens(),
                                |rect| {
                                    let index = card_index.get();
                                    let mut card_rects = card_rects.borrow_mut();
                                    if index == card_rects.len() {
                                        card_rects.push(rect);
                                    } else {
                                        card_rects[index] = rect;
                                    }
                                    card_index.set((index + 1) % Concept::ALL.len());
                                },
                            );
                            assert!(screen.contains_rect(viewport));
                        });
                    },
                );
                let cards = card_rects.into_inner();
                assert_eq!(
                    cards.len(),
                    Concept::ALL.len(),
                    "{component:?} at {width}px"
                );
                for rect in cards {
                    assert!(
                        screen.contains_rect(rect),
                        "card outside {width}px {component:?} viewport: {rect:?}"
                    );
                }

                let nodes = output
                    .platform_output
                    .accesskit_update
                    .expect("accessibility tree")
                    .nodes;
                let mut slider_count = 0;
                let mut button_count = 0;
                for (_, node) in nodes {
                    let role = format!("{:?}", node.role());
                    if role == "Slider" {
                        slider_count += 1;
                    } else if role == "Button" {
                        button_count += 1;
                    } else {
                        continue;
                    }
                    let bounds = node.bounds().expect("interactive control bounds");
                    assert!(
                        bounds.x0 >= screen.left() as f64 - 0.5
                            && bounds.x1 <= screen.right() as f64 + 0.5,
                        "{role} outside {width}px {component:?} viewport: {bounds:?}"
                    );
                }
                let expected_sliders = usize::from(component != ComponentTab::Transport) * 10;
                let expected_buttons = 15 + usize::from(component == ComponentTab::Transport) * 20;
                assert_eq!(
                    slider_count, expected_sliders,
                    "{component:?} sliders at {width}px"
                );
                assert_eq!(
                    button_count, expected_buttons,
                    "{component:?} buttons at {width}px"
                );
            }
        }
    }
}
