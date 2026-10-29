use eframe::egui;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use std::path::PathBuf;

mod theme;
use theme::ThemeMode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Id,
    Filename,
    Duration,
}
struct LibraryApp {
    db_path: PathBuf,
    catalog: Result<Catalog, String>,
    tracks: Vec<TrackDetails>,
    load_error: Option<String>,
    search: String,
    import_root: String,
    selected: Option<u64>,
    sort: Sort,
    descending: bool,
    notice: String,
    theme: ThemeMode,
    custom_tokens: theme::ColorTokens,
    show_theme_editor: bool,
}
impl LibraryApp {
    fn new(db_path: PathBuf, theme: ThemeMode, custom_tokens: theme::ColorTokens) -> Self {
        let catalog = Catalog::open(&db_path).map_err(|e| e.to_string());
        let mut app = Self {
            db_path,
            catalog,
            tracks: vec![],
            load_error: None,
            search: String::new(),
            import_root: String::new(),
            selected: None,
            sort: Sort::Id,
            descending: false,
            notice: String::new(),
            theme,
            custom_tokens,
            show_theme_editor: false,
        };
        app.reload();
        app
    }
    fn active_tokens(&self) -> theme::ColorTokens {
        if self.theme == ThemeMode::Custom {
            self.custom_tokens
        } else {
            self.theme.tokens()
        }
    }

    fn reload(&mut self) {
        self.tracks.clear();
        self.load_error = None;
        match &self.catalog {
            Ok(db) => match db.list_track_details() {
                Ok(rows) => self.tracks = rows,
                Err(e) => self.load_error = Some(e.to_string()),
            },
            Err(e) => self.load_error = Some(e.clone()),
        }
    }
    fn import(&mut self) {
        match &mut self.catalog {
            Ok(db) => match db.import_audio_directory(&self.import_root) {
                Ok(report) => {
                    self.notice = format!(
                        "Import: {} outcomes{}",
                        report.outcomes.len(),
                        if report.complete {
                            ""
                        } else {
                            " (incomplete; missing states unchanged)"
                        }
                    );
                    self.reload();
                }
                Err(e) => self.notice = format!("Import failed: {e}"),
            },
            Err(e) => self.notice = format!("Catalog unavailable: {e}"),
        }
    }
}
impl eframe::App for LibraryApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.theme.save(self.custom_tokens, storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.heading("OxiDJ · Local Library");
            ui.horizontal(|ui| {
                ui.label("Theme");
                egui::ComboBox::from_id_salt("theme-selection")
                    .selected_text(self.theme.label())
                    .show_ui(ui, |ui| {
                        let previous = self.theme;
                        ui.selectable_value(&mut self.theme, ThemeMode::TokyoNight, "Tokyo Night");
                        ui.selectable_value(&mut self.theme, ThemeMode::TokyoDay, "Tokyo Day");
                        ui.selectable_value(&mut self.theme, ThemeMode::Custom, "Custom");
                        if self.theme == ThemeMode::Custom && previous != ThemeMode::Custom {
                            self.custom_tokens = previous.tokens();
                        }
                    });
                if self.theme == ThemeMode::Custom {
                    if ui.button("Edit palette…").clicked() {
                        self.show_theme_editor = true;
                    }
                }
            });
            if self.show_theme_editor {
                egui::Window::new("Custom palette")
                    .open(&mut self.show_theme_editor)
                    .show(ctx, |ui| {
                        ui.label("Semantic colors");
                        edit_color(ui, "Background", &mut self.custom_tokens.background);
                        edit_color(ui, "Surface", &mut self.custom_tokens.surface);
                        edit_color(ui, "Text", &mut self.custom_tokens.text);
                        edit_color(ui, "Muted text", &mut self.custom_tokens.muted_text);
                        edit_color(ui, "Primary", &mut self.custom_tokens.primary);
                        edit_color(ui, "Secondary", &mut self.custom_tokens.secondary);
                        edit_color(ui, "Border", &mut self.custom_tokens.border);
                        edit_color(ui, "Focus", &mut self.custom_tokens.focus);
                        edit_color(ui, "Success", &mut self.custom_tokens.success);
                        edit_color(ui, "Warning", &mut self.custom_tokens.warning);
                        edit_color(ui, "Error", &mut self.custom_tokens.error);
                        ui.horizontal(|ui| {
                            if ui.button("Reset to Tokyo Night").clicked() {
                                self.custom_tokens = ThemeMode::TokyoNight.tokens();
                            }
                            if ui.button("Reset to Tokyo Day").clicked() {
                                self.custom_tokens = ThemeMode::TokyoDay.tokens();
                            }
                        });
                    });
            }
            ui.horizontal(|ui| {
                ui.label("Catalog:");
                ui.monospace(self.db_path.display().to_string());
                if ui.button("Reload").clicked() {
                    self.reload();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Search");
                ui.add_sized(
                    [ui.available_width().min(160.0), 24.0],
                    egui::TextEdit::singleline(&mut self.search).hint_text("Filter tracks"),
                );
                ui.label("Import folder");
                ui.add_sized(
                    [ui.available_width().min(200.0), 24.0],
                    egui::TextEdit::singleline(&mut self.import_root).hint_text("Folder path"),
                );
                if ui.button("Import references").clicked() {
                    self.import();
                }
            });
            if !self.notice.is_empty() {
                let color = if self.notice.starts_with("Import failed:")
                    || self.notice.starts_with("Catalog unavailable:")
                {
                    tokens.error
                } else if self.notice.contains("incomplete") {
                    tokens.warning
                } else {
                    tokens.success
                };
                ui.colored_label(color, &self.notice);
            }
        });
        egui::SidePanel::right("details")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Track details");
                if let Some(id) = self.selected {
                    if let Some(item) = self.tracks.iter().find(|x| x.track.id.as_u64() == id) {
                        ui.label(format!("ID: {id}"));
                        ui.label(format!(
                            "Filename: {}",
                            PathBuf::from(&item.track.path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                        ui.label(format!("Path: {}", item.track.path));
                        ui.label(format!(
                            "Duration: {}",
                            item.duration_seconds
                                .map(|v| format!("{v:.3} s"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Sample rate: {}",
                            item.sample_rate
                                .map(|v| format!("{v} Hz"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Channels: {}",
                            item.channels
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Bit depth: {}",
                            item.bit_depth
                                .map(|v| format!("{v} bit"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!("Size: {} bytes", item.track.byte_length));
                        let availability = match item.availability {
                            TrackAvailability::Available => ("Available", tokens.success),
                            TrackAvailability::Missing => ("Missing", tokens.warning),
                        };
                        ui.colored_label(
                            availability.1,
                            format!("Availability: {}", availability.0),
                        );
                        ui.monospace(format!(
                            "SHA-256: {}",
                            item.track
                                .sha256
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ));
                    } else {
                        ui.label("Selected track is no longer in the catalog.");
                    }
                } else {
                    ui.label("Select a track to inspect its details.");
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.load_error {
                ui.colored_label(tokens.error, format!("Catalog error: {error}"));
                return;
            }
            ui.horizontal(|ui| {
                ui.label("Sort by");
                egui::ComboBox::from_id_salt("sort")
                    .selected_text(match self.sort {
                        Sort::Id => "Stable ID",
                        Sort::Filename => "Filename",
                        Sort::Duration => "Duration",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort, Sort::Id, "Stable ID");
                        ui.selectable_value(&mut self.sort, Sort::Filename, "Filename");
                        ui.selectable_value(&mut self.sort, Sort::Duration, "Duration");
                    });
                if ui
                    .button(if self.descending {
                        "Descending"
                    } else {
                        "Ascending"
                    })
                    .clicked()
                {
                    self.descending = !self.descending;
                }
                ui.label(format!("{} tracks", self.tracks.len()));
            });
            let query = self.search.to_lowercase();
            let mut visible: Vec<_> = self
                .tracks
                .iter()
                .filter(|t| {
                    query.is_empty()
                        || t.track.path.to_lowercase().contains(&query)
                        || t.track.id.as_u64().to_string().contains(&query)
                })
                .collect();
            visible.sort_by(|a, b| {
                let order = match self.sort {
                    Sort::Id => a.track.id.as_u64().cmp(&b.track.id.as_u64()),
                    Sort::Filename => PathBuf::from(&a.track.path)
                        .file_name()
                        .cmp(&PathBuf::from(&b.track.path).file_name())
                        .then(a.track.id.as_u64().cmp(&b.track.id.as_u64())),
                    Sort::Duration => a
                        .duration_seconds
                        .partial_cmp(&b.duration_seconds)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.track.id.as_u64().cmp(&b.track.id.as_u64())),
                };
                if self.descending {
                    order.reverse()
                } else {
                    order
                }
            });
            if visible.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(if self.tracks.is_empty() {
                        "Library is empty. Import a folder of supported audio references."
                    } else {
                        "No tracks match this search."
                    })
                });
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("tracks").striped(true).show(ui, |ui| {
                    ui.strong("ID");
                    ui.strong("Filename / path");
                    ui.strong("Duration");
                    ui.strong("Format facts");
                    ui.strong("Availability");
                    ui.end_row();
                    for item in visible {
                        let id = item.track.id.as_u64();
                        if ui
                            .selectable_label(self.selected == Some(id), id.to_string())
                            .clicked()
                        {
                            self.selected = Some(id);
                        }
                        ui.vertical(|ui| {
                            let name = PathBuf::from(&item.track.path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if ui
                                .selectable_label(self.selected == Some(id), name)
                                .clicked()
                            {
                                self.selected = Some(id);
                            }
                            ui.small(&item.track.path);
                        });
                        ui.label(
                            item.duration_seconds
                                .map(|v| format!("{v:.2} s"))
                                .unwrap_or_else(|| "—".into()),
                        );
                        ui.label(match (item.sample_rate, item.channels, item.bit_depth) {
                            (Some(r), Some(c), Some(b)) => format!("{r} Hz · {c} ch · {b} bit"),
                            _ => "Facts unavailable".into(),
                        });
                        let (availability, color) = match item.availability {
                            TrackAvailability::Available => ("Available", tokens.success),
                            TrackAvailability::Missing => ("Missing", tokens.warning),
                        };
                        ui.colored_label(color, availability);
                        ui.end_row();
                    }
                });
            });
        });
    }
}
fn edit_color(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_srgba(color);
    });
}

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let db = match (args.next().as_deref(), args.next()) {
        (Some("--db"), Some(path)) => PathBuf::from(path),
        _ => PathBuf::from("oxidj-library.sqlite"),
    };
    eframe::run_native(
        "OxiDJ Local Library",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            cc.egui_ctx.enable_accesskit();
            let (theme, custom_tokens) = ThemeMode::load(cc.storage);
            Ok(Box::new(LibraryApp::new(db, theme, custom_tokens)))
        }),
    )
}
