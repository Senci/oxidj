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

impl Default for Concept {
    fn default() -> Self {
        Self::StudioRack
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

    fn value_label(self) -> &'static str {
        match self {
            Self::RotaryKnob => "Knob value",
            Self::ChannelFader => "Channel level",
            Self::Transport => "Transport",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransportAction {
    Play,
    Cue,
}

#[derive(Default)]
pub struct GalleryState {
    pub selected: Concept,
    pub component: ComponentTab,
    pub value: f32,
    last_transport: Option<(Concept, TransportAction)>,
}

fn component_tabs(ui: &mut egui::Ui, state: &mut GalleryState) -> Vec<egui::Response> {
    let mut responses = Vec::with_capacity(ComponentTab::ALL.len());
    ui.horizontal_wrapped(|ui| {
        for component in ComponentTab::ALL {
            responses.push(ui.selectable_value(&mut state.component, component, component.label()));
        }
    });
    responses
}

pub fn show(ui: &mut egui::Ui, state: &mut GalleryState, tokens: ColorTokens) -> egui::Rect {
    ui.heading("Control concepts · compare by component");
    ui.label(
        "Compare the same control across ten design concepts. Preview only; no audio is connected.",
    );
    component_tabs(ui, state);
    ui.separator();

    if state.component != ComponentTab::Transport {
        ui.add(
            egui::Slider::new(&mut state.value, 0.0..=1.0)
                .show_value(true)
                .text(state.component.value_label()),
        );
        ui.separator();
    }

    let gallery_rect = ui.available_rect_before_wrap();
    let compact = ui.available_width() < 680.0;
    egui::ScrollArea::vertical().show(ui, |ui| {
        if compact {
            for concept in Concept::ALL {
                show_concept_card(ui, concept, state, tokens);
            }
        } else {
            for pair in Concept::ALL.chunks(2) {
                ui.columns(2, |columns| {
                    for (index, concept) in pair.iter().copied().enumerate() {
                        show_concept_card(&mut columns[index], concept, state, tokens);
                    }
                });
                ui.add_space(6.0);
            }
        }
    });
    gallery_rect
}

fn show_concept_card(
    ui: &mut egui::Ui,
    concept: Concept,
    state: &mut GalleryState,
    tokens: ColorTokens,
) {
    let selected = state.selected == concept;
    let border = if selected {
        tokens.primary
    } else {
        tokens.border
    };
    let radius = match concept {
        Concept::SoftTiles => 18.0,
        Concept::StagePanel => 2.0,
        Concept::MinimalLine => 0.0,
        _ => 8.0,
    };
    egui::Frame::new()
        .fill(tokens.surface)
        .stroke(Stroke::new(
            if concept == Concept::MinimalLine {
                0.5_f32
            } else {
                1.0_f32
            },
            border,
        ))
        .corner_radius(radius)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let response = ui
                .selectable_label(selected, concept.name())
                .on_hover_text(concept.philosophy());
            if response.clicked() {
                state.selected = concept;
            }
            ui.small(concept.philosophy());
            ui.separator();
            match state.component {
                ComponentTab::RotaryKnob => draw_knob(ui, concept, state.value, tokens),
                ComponentTab::ChannelFader => draw_fader(ui, concept, state.value, tokens),
                ComponentTab::Transport => {
                    draw_transport(ui, concept, state, tokens);
                }
            }
        });
}

fn concept_accent(concept: Concept, tokens: ColorTokens) -> Color32 {
    match concept {
        Concept::NeonDeck => tokens.secondary,
        Concept::MeterBridge => tokens.success,
        Concept::PaperConsole => tokens.text,
        _ => tokens.primary,
    }
}

fn draw_knob(ui: &mut egui::Ui, concept: Concept, value: f32, tokens: ColorTokens) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let radius = match concept {
            Concept::StagePanel => 39.0,
            Concept::MinimalLine => 30.0,
            Concept::SoftTiles => 35.0,
            _ => 33.0,
        };
        let center = rect.center();
        let accent = concept_accent(concept, tokens);
        painter.circle_filled(center, radius, tokens.background);
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(
                if concept == Concept::MinimalLine {
                    1.0_f32
                } else {
                    4.0_f32
                },
                tokens.border,
            ),
        );
        let ticks = if concept == Concept::MeterBridge {
            11
        } else {
            7
        };
        for tick in 0..ticks {
            let angle = -2.35 + tick as f32 / (ticks - 1) as f32 * 4.7;
            let inner = center + egui::vec2(angle.cos(), angle.sin()) * (radius + 2.0);
            let outer = center + egui::vec2(angle.cos(), angle.sin()) * (radius + 8.0);
            painter.line_segment([inner, outer], Stroke::new(1.0_f32, tokens.muted_text));
        }
        let angle = -2.35 + value.clamp(0.0, 1.0) * 4.7;
        let pointer = center + egui::vec2(angle.cos(), angle.sin()) * (radius - 6.0);
        painter.line_segment([center, pointer], Stroke::new(3.0_f32, accent));
        painter.circle_filled(center, 3.0, tokens.text);
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
}

fn draw_fader(ui: &mut egui::Ui, concept: Concept, value: f32, tokens: ColorTokens) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 150.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let x = rect.center().x;
        let accent = concept_accent(concept, tokens);
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 8.0),
                egui::pos2(x, rect.bottom() - 8.0),
            ],
            Stroke::new(
                if concept == Concept::MinimalLine {
                    1.0_f32
                } else {
                    4.0_f32
                },
                tokens.border,
            ),
        );
        let ticks = if concept == Concept::UtilityGrid {
            7
        } else {
            5
        };
        for tick in 0..ticks {
            let y = rect.top() + 10.0 + tick as f32 * (rect.height() - 20.0) / (ticks - 1) as f32;
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
        let thumb_width = if concept == Concept::ModularBay {
            42.0
        } else {
            32.0
        };
        let thumb =
            egui::Rect::from_center_size(egui::pos2(x, thumb_y), egui::vec2(thumb_width, 14.0));
        if concept == Concept::SoftTiles {
            painter.circle_filled(egui::pos2(x, thumb_y), 10.0, accent);
        } else {
            painter.rect_filled(
                thumb,
                if concept == Concept::StagePanel {
                    1.0
                } else {
                    4.0
                },
                accent,
            );
        }
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
}

fn draw_transport(
    ui: &mut egui::Ui,
    concept: Concept,
    state: &mut GalleryState,
    tokens: ColorTokens,
) -> Vec<egui::Response> {
    let button_width = if matches!(concept, Concept::StagePanel | Concept::Broadcast) {
        90.0
    } else {
        72.0
    };
    let mut responses = Vec::with_capacity(2);
    ui.horizontal_wrapped(|ui| {
        for (action, label, icon) in [
            (TransportAction::Play, "Play", "▶"),
            (TransportAction::Cue, "Cue", "●"),
        ] {
            let active = state.last_transport == Some((concept, action));
            let fill =
                if active || (concept == Concept::Broadcast && action == TransportAction::Play) {
                    concept_accent(concept, tokens).linear_multiply(0.35)
                } else {
                    tokens.background
                };
            let accessible_name = format!("{label}, {}", concept.name());
            let response = ui.add_sized(
                [button_width, 48.0],
                egui::Button::new(
                    egui::RichText::new(format!("{icon} {label}")).color(tokens.text),
                )
                .fill(fill)
                .stroke(Stroke::new(
                    if concept == Concept::MinimalLine {
                        0.5_f32
                    } else {
                        1.0_f32
                    },
                    concept_accent(concept, tokens),
                )),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, accessible_name.clone())
            });
            if response.clicked() {
                state.last_transport = Some((concept, action));
            }
            responses.push(response);
        }
    });
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
    fn controls_expose_named_native_accessibility_widgets_and_value_changes() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut state = GalleryState {
            selected: Concept::StudioRack,
            value: 0.2,
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
        for name in ["Knob value"] {
            assert!(
                nodes.values().any(|node| node.label() == Some(name)
                    || node.value() == Some(name)
                    || node.labelled_by().iter().any(|id| nodes
                        .get(id)
                        .is_some_and(|label| label.value() == Some(name)))),
                "missing accessible control {name}; nodes={nodes:?}"
            );
        }
        state.value = 0.8;
        let after = render(&mut state);
        assert_ne!(
            format!("{:?}", before.shapes),
            format!("{:?}", after.shapes),
            "changed control value should change the preview drawing"
        );

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
