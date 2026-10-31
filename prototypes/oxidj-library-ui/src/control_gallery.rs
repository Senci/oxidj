use crate::theme::ColorTokens;
use eframe::egui::{self, Color32, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Default)]
pub struct GalleryState {
    pub selected: Concept,
    pub value: f32,
}
impl Default for Concept {
    fn default() -> Self {
        Self::StudioRack
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut GalleryState, tokens: ColorTokens) -> egui::Rect {
    ui.heading("Control concepts · preview only");
    ui.label("Ten interaction/layout directions. No concept is a production control system.");
    ui.horizontal_wrapped(|ui| {
        for concept in Concept::ALL {
            ui.selectable_value(&mut state.selected, concept, concept.name())
                .on_hover_text(concept.philosophy());
        }
    });
    ui.separator();
    let width = ui.available_width();
    let compact = width < 560.0;
    let frame_color = match state.selected {
        Concept::PaperConsole => Color32::from_rgb(0x30, 0x32, 0x3c),
        Concept::NeonDeck | Concept::StagePanel => tokens.background,
        _ => tokens.surface,
    };
    let frame = egui::Frame::new()
        .fill(frame_color)
        .stroke(Stroke::new(
            if state.selected == Concept::MinimalLine {
                0.5_f32
            } else {
                1.0_f32
            },
            tokens.border,
        ))
        .corner_radius(match state.selected {
            Concept::SoftTiles => 18.0,
            Concept::StagePanel => 2.0,
            _ => 8.0,
        })
        .inner_margin(egui::Margin::same(if compact { 10 } else { 20 }));
    frame
        .show(ui, |ui| {
            let header = match state.selected {
                Concept::Broadcast => "ON AIR  ·  DECK A",
                Concept::MeterBridge => "SIGNAL / TRANSPORT / CHANNEL",
                Concept::UtilityGrid => "CH 01  |  MASTER  |  TRANSPORT",
                _ => state.selected.name(),
            };
            ui.horizontal(|ui| {
                ui.strong(header);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(state.selected.philosophy());
                });
            });
            ui.separator();
            let stacked = compact
                || matches!(
                    state.selected,
                    Concept::PaperConsole
                        | Concept::Broadcast
                        | Concept::MeterBridge
                        | Concept::UtilityGrid
                );
            if stacked {
                ui.vertical(|ui| controls(ui, state, tokens));
            } else {
                ui.horizontal(|ui| controls(ui, state, tokens));
            }
        })
        .response
        .rect
}

fn controls(ui: &mut egui::Ui, state: &mut GalleryState, tokens: ColorTokens) {
    let style = state.selected;
    let (dial_rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 130.0), egui::Sense::hover());
    let painter = ui.painter_at(dial_rect);
    let center = egui::pos2(dial_rect.center().x, dial_rect.center().y - 6.0);
    let radius = if style == Concept::StagePanel {
        43.0
    } else {
        34.0
    };
    let ring = match style {
        Concept::NeonDeck => tokens.secondary,
        Concept::PaperConsole => tokens.primary,
        Concept::MeterBridge => tokens.success,
        _ => tokens.primary,
    };
    painter.circle(
        center,
        radius,
        tokens.background,
        Stroke::new(7.0_f32, tokens.border),
    );
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            if style == Concept::MinimalLine {
                1.0_f32
            } else {
                4.0_f32
            },
            ring,
        ),
    );
    let angle = -2.35 + state.value * 4.7;
    let pointer = center + egui::vec2(angle.cos(), angle.sin()) * (radius - 5.0);
    painter.line_segment([center, pointer], Stroke::new(3.0_f32, tokens.text));
    painter.circle_filled(center, 4.0, tokens.secondary);
    painter.text(
        egui::pos2(center.x, center.y + radius + 12.0),
        egui::Align2::CENTER_CENTER,
        format!("{}%", (state.value * 100.0).round()),
        egui::FontId::proportional(12.0),
        tokens.text,
    );
    let knob_label = ui.label("FILTER");
    ui.add(
        egui::Slider::new(&mut state.value, 0.0..=1.0)
            .show_value(false)
            .text("Value"),
    )
    .labelled_by(knob_label.id);

    let (fader_rect, _) = ui.allocate_exact_size(egui::vec2(76.0, 170.0), egui::Sense::hover());
    let fp = ui.painter_at(fader_rect);
    let x = fader_rect.center().x;
    fp.line_segment(
        [
            egui::pos2(x, fader_rect.top() + 8.0),
            egui::pos2(x, fader_rect.bottom() - 8.0),
        ],
        Stroke::new(5.0_f32, tokens.border),
    );
    for i in 0..5 {
        let y = fader_rect.top() + 12.0 + i as f32 * 34.0;
        fp.line_segment(
            [egui::pos2(x - 10.0, y), egui::pos2(x + 10.0, y)],
            Stroke::new(1.0_f32, tokens.muted_text),
        );
    }
    let thumb_y = egui::lerp(
        (fader_rect.bottom() - 12.0)..=(fader_rect.top() + 12.0),
        state.value,
    );
    let thumb = egui::Rect::from_center_size(
        egui::pos2(x, thumb_y),
        egui::vec2(
            if style == Concept::ModularBay {
                38.0
            } else {
                30.0
            },
            14.0,
        ),
    );
    fp.rect(
        thumb,
        3.0,
        tokens.primary,
        Stroke::new(1.0_f32, tokens.text),
        egui::StrokeKind::Middle,
    );
    ui.vertical(|ui| {
        ui.strong("CHANNEL 01");
        ui.add(
            egui::Slider::new(&mut state.value, 0.0..=1.0)
                .vertical()
                .show_value(false)
                .text("Level"),
        );
    });

    ui.vertical(|ui| {
        ui.strong("TRANSPORT");
        ui.horizontal(|ui| {
            for (label, icon) in [("Play", "▶"), ("Cue", "●")] {
                let response = ui.add_sized(
                    [64.0, 52.0],
                    egui::Button::new(egui::RichText::new(icon).size(22.0).color(tokens.text))
                        .fill(if style == Concept::Broadcast {
                            tokens.primary
                        } else {
                            tokens.surface
                        }),
                );
                response.on_hover_text(label).widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                });
            }
        });
        ui.label("Idle · hover · focus · pressed");
        ui.small(
            "Use Tab/Space to focus and activate controls; drag either slider to change value.",
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;
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
    fn controls_fit_narrow_and_wide_gallery_widths() {
        for width in [420.0, 1100.0] {
            let ctx = egui::Context::default();
            let mut state = GalleryState::default();
            let rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0));
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        rect.set(show(ui, &mut state, ThemeMode::TokyoNight.tokens()));
                    });
                },
            );
            assert!(
                screen.contains_rect(rect.get()),
                "gallery clipped at {width}px: {:?}",
                rect.get()
            );
        }
    }
}
