use eframe::egui;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use std::path::PathBuf;

mod components;
mod control_gallery;
mod theme;
use components::{PrimaryActionButton, SectionHeading, StatusBadge, StatusKind};
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
    primary_hex: String,
    secondary_hex: String,
    accent_error: Option<String>,
    xml_path: String,
    xml_preview: Option<Result<oxidj_application::Preview, String>>,
    show_xml_preview: bool,
    xml_edit_track: Option<String>,
    xml_edit_name: String,
    xml_edit_output: String,
    show_control_gallery: bool,
    control_gallery: control_gallery::GalleryState,
    availability_filter: Option<TrackAvailability>,
    deck_playing: [bool; 2],
    deck_cue: [bool; 2],
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
            primary_hex: format!(
                "#{:02X}{:02X}{:02X}",
                custom_tokens.primary.r(),
                custom_tokens.primary.g(),
                custom_tokens.primary.b()
            ),
            secondary_hex: format!(
                "#{:02X}{:02X}{:02X}",
                custom_tokens.secondary.r(),
                custom_tokens.secondary.g(),
                custom_tokens.secondary.b()
            ),
            accent_error: None,
            xml_path: String::new(),
            xml_preview: None,
            show_xml_preview: false,
            xml_edit_track: None,
            xml_edit_name: String::new(),
            xml_edit_output: String::new(),
            show_control_gallery: false,
            control_gallery: control_gallery::GalleryState::default(),
            availability_filter: None,
            deck_playing: [false; 2],
            deck_cue: [false; 2],
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

    fn select_theme(&mut self, mode: ThemeMode) {
        let previous = self.theme;
        if mode == ThemeMode::Custom {
            if previous != ThemeMode::Custom {
                self.custom_tokens = previous.tokens();
            }
            self.show_theme_editor = true;
        }
        self.theme = mode;
        self.sync_accent_fields();
    }

    fn sync_accent_fields(&mut self) {
        self.primary_hex = color_hex(self.custom_tokens.primary);
        self.secondary_hex = color_hex(self.custom_tokens.secondary);
        self.accent_error = None;
    }

    fn commit_accent(&mut self, primary: bool) {
        let text = if primary {
            &self.primary_hex
        } else {
            &self.secondary_hex
        };
        match parse_hex_color(text) {
            Some(color) => {
                self.theme = ThemeMode::Custom;
                if primary {
                    self.custom_tokens.primary = color;
                } else {
                    self.custom_tokens.secondary = color;
                }
                self.accent_error = None;
            }
            None => self.accent_error = Some("Enter a complete #RRGGBB color".into()),
        }
    }

    fn handle_theme_shortcuts(&mut self, ctx: &egui::Context) {
        let mode = ctx.input(|input| {
            if !input.modifiers.ctrl || !input.modifiers.alt {
                return None;
            }
            if input.key_pressed(egui::Key::Num1) {
                Some(ThemeMode::TokyoNight)
            } else if input.key_pressed(egui::Key::Num2) {
                Some(ThemeMode::TokyoDay)
            } else if input.key_pressed(egui::Key::Num3) {
                Some(ThemeMode::Custom)
            } else {
                None
            }
        });
        if let Some(mode) = mode {
            self.select_theme(mode);
        }
    }

    fn show_palette_fields(&mut self, ui: &mut egui::Ui, error_color: egui::Color32) {
        ui.horizontal(|ui| {
            let label = ui.label("Primary");
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut self.primary_hex)
                        .id(egui::Id::new("primary-hex"))
                        .desired_width(90.0)
                        .hint_text("#RRGGBB"),
                )
                .labelled_by(label.id);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.commit_accent(true);
            }
            if ui.button("Apply").clicked() {
                self.commit_accent(true);
            }
        });
        let primary_before = self.custom_tokens.primary;
        edit_color(ui, "Primary swatch", &mut self.custom_tokens.primary);
        if self.custom_tokens.primary != primary_before {
            self.primary_hex = color_hex(self.custom_tokens.primary);
            self.accent_error = None;
        }
        ui.horizontal(|ui| {
            let label = ui.label("Secondary");
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut self.secondary_hex)
                        .id(egui::Id::new("secondary-hex"))
                        .desired_width(90.0)
                        .hint_text("#RRGGBB"),
                )
                .labelled_by(label.id);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.commit_accent(false);
            }
            if ui.button("Apply").clicked() {
                self.commit_accent(false);
            }
        });
        let secondary_before = self.custom_tokens.secondary;
        edit_color(ui, "Secondary swatch", &mut self.custom_tokens.secondary);
        if self.custom_tokens.secondary != secondary_before {
            self.secondary_hex = color_hex(self.custom_tokens.secondary);
            self.accent_error = None;
        }
        if let Some(error) = &self.accent_error {
            ui.colored_label(error_color, error);
        }
        edit_color(ui, "Border", &mut self.custom_tokens.border);
        edit_color(ui, "Focus", &mut self.custom_tokens.focus);
        edit_color(ui, "Success", &mut self.custom_tokens.success);
        edit_color(ui, "Warning", &mut self.custom_tokens.warning);
        edit_color(ui, "Error", &mut self.custom_tokens.error);
        let mut reset = false;
        ui.horizontal(|ui| {
            if ui.button("Reset to Tokyo Night").clicked() {
                self.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoNight);
                reset = true;
            }
            if ui.button("Reset to Tokyo Day").clicked() {
                self.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
                reset = true;
            }
        });
        if reset {
            self.sync_accent_fields();
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

    fn preview_xml(&mut self) {
        self.xml_preview = Some(
            oxidj_application::preview_rekordbox_xml(&self.xml_path).map_err(|e| e.to_string()),
        );
        self.show_xml_preview = true;
    }

    fn export_xml_name_edit(&mut self, track_id: &str, old_name: &str) {
        let request = oxidj_application::TrackNameEditRequest {
            track_id: track_id.to_owned(),
            expected_old_name: old_name.to_owned(),
            new_name: self.xml_edit_name.clone(),
        };
        match oxidj_application::export_rekordbox_track_name_edit(
            &self.xml_path,
            &self.xml_edit_output,
            &request,
        ) {
            Ok(change) => {
                self.notice = format!(
                    "Exported track {} Name: {:?} → {:?} to {}",
                    change.track_id, change.old_name, change.new_name, self.xml_edit_output
                )
            }
            Err(error) => self.notice = format!("Track Name export failed: {error}"),
        }
    }

    fn theme_selection_ui(&mut self, ui: &mut egui::Ui) -> (egui::Rect, Option<[egui::Rect; 3]>) {
        let mut selected = self.theme;
        let mut option_rects = None;
        let label = ui.label("Theme (Ctrl+Alt+1/2/3)");
        let combo = egui::ComboBox::from_id_salt("theme-selection")
            .selected_text(self.theme.label())
            .show_ui(ui, |ui| {
                let night =
                    ui.selectable_value(&mut selected, ThemeMode::TokyoNight, "Tokyo Night");
                let day = ui.selectable_value(&mut selected, ThemeMode::TokyoDay, "Tokyo Day");
                let custom = ui.selectable_value(&mut selected, ThemeMode::Custom, "Custom");
                option_rects = Some([night.rect, day.rect, custom.rect]);
            });
        if selected != self.theme {
            self.select_theme(selected);
        }
        (combo.response.labelled_by(label.id).rect, option_rects)
    }
}

impl eframe::App for LibraryApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.theme.save(self.custom_tokens, storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_theme_shortcuts(ctx);
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.heading("OxiDJ · Local Library");
            ui.horizontal(|ui| {
                self.theme_selection_ui(ui);
                if ui.button("Control concepts…").clicked() {
                    self.show_control_gallery = true;
                }
                if self.theme == ThemeMode::Custom {
                    if ui.button("Edit palette…").clicked() {
                        self.show_theme_editor = true;
                    }
                }
            });
            if self.show_theme_editor {
                let mut editor_open = self.show_theme_editor;
                egui::Window::new("Custom palette")
                    .open(&mut editor_open)
                    .show(ctx, |ui| {
                        ui.label("Semantic colors");
                        edit_color(ui, "Background", &mut self.custom_tokens.background);
                        edit_color(ui, "Surface", &mut self.custom_tokens.surface);
                        edit_color(ui, "Text", &mut self.custom_tokens.text);
                        edit_color(ui, "Muted text", &mut self.custom_tokens.muted_text);
                        self.show_palette_fields(ui, tokens.error);
                    });
                self.show_theme_editor = editor_open;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Catalog:");
                let full_path = self.db_path.display().to_string();
                let path_width = (ui.available_width() - 68.0).clamp(80.0, 360.0);
                ui.add_sized(
                    [path_width, 20.0],
                    egui::Label::new(egui::RichText::new(&full_path).monospace()).truncate(),
                )
                .on_hover_text(&full_path);
                if ui.button("Reload").clicked() {
                    self.reload();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Search");
                ui.add_sized(
                    [ui.available_width().clamp(100.0, 160.0), 24.0],
                    egui::TextEdit::singleline(&mut self.search).hint_text("Filter tracks"),
                );
                ui.label("Import folder");
                ui.add_sized(
                    [ui.available_width().clamp(120.0, 200.0), 24.0],
                    egui::TextEdit::singleline(&mut self.import_root).hint_text("Folder path"),
                );
                if (PrimaryActionButton {
                    label: "Import references",
                })
                .show(ui, tokens)
                .clicked()
                {
                    self.import();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Rekordbox XML (read-only)");
                ui.add_sized(
                    [ui.available_width().clamp(140.0, 360.0), 24.0],
                    egui::TextEdit::singleline(&mut self.xml_path).hint_text("XML file path"),
                );
                if (PrimaryActionButton {
                    label: "Preview XML",
                })
                .show(ui, tokens)
                .clicked()
                {
                    self.preview_xml();
                }
            });
            if !self.notice.is_empty() {
                let kind = if self.notice.starts_with("Import failed:")
                    || self.notice.starts_with("Track Name export failed:")
                    || self.notice.starts_with("Catalog unavailable:")
                {
                    StatusKind::Error
                } else if self.notice.contains("incomplete") {
                    StatusKind::Warning
                } else {
                    StatusKind::Success
                };
                StatusBadge {
                    label: &self.notice,
                    kind,
                }
                .show(ui, tokens);
            }
        });
        if self.show_control_gallery {
            let mut open = self.show_control_gallery;
            egui::Window::new("OxiDJ · Control concept gallery")
                .open(&mut open)
                .default_width(900.0)
                .min_width(360.0)
                .show(ctx, |ui| {
                    control_gallery::show(ui, &mut self.control_gallery, tokens)
                });
            self.show_control_gallery = open;
        }
        if self.show_xml_preview {
            let mut open = self.show_xml_preview;
            egui::Window::new("Rekordbox XML preview · guarded Name export")
                .open(&mut open)
                .default_width(720.0)
                .show(ctx, |ui| match self.xml_preview.clone() {
                    Some(Err(error)) => {
                        StatusBadge {
                            label: &format!("Preview failed: {error}"),
                            kind: StatusKind::Error,
                        }
                        .show(ui, tokens);
                    }
                    Some(Ok(preview)) => {
                        ui.label(format!(
                            "{} tracks · {} folders · {} playlists",
                            preview.tracks.len(),
                            preview.folders.len(),
                            preview.playlists.len()
                        ));
                        if preview.tracks.is_empty() {
                            ui.label("No tracks in this XML file.");
                        }
                        egui::ScrollArea::vertical()
                            .max_height(260.0)
                            .show(ui, |ui| {
                                egui::Grid::new("rekordbox-preview-tracks")
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.strong("Select");
                                        ui.strong("Track");
                                        ui.strong("Artist");
                                        ui.strong("Raw location");
                                        ui.end_row();
                                        for track in &preview.tracks {
                                            let selected = self.xml_edit_track.as_deref() == Some(&track.id);
                                            if ui.selectable_label(selected, "Select").clicked() {
                                                self.xml_edit_track = Some(track.id.clone());
                                                self.xml_edit_name = track.name.clone();
                                            }
                                            ui.label(&track.name);
                                            ui.label(track.artist.as_deref().unwrap_or("—"));
                                            ui.monospace(&track.location);
                                            ui.end_row();
                                        }
                                    });
                            });
                        ui.separator();
                        ui.strong("Export one guarded Name edit");
                        if let Some(track_id) = self.xml_edit_track.clone() {
                            ui.label(format!("Selected TrackID: {track_id}"));
                            ui.horizontal(|ui| {
                                ui.label("New Name");
                                ui.text_edit_singleline(&mut self.xml_edit_name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Output path");
                                ui.text_edit_singleline(&mut self.xml_edit_output);
                            });
                            if (PrimaryActionButton { label: "Export to new file" }).show(ui, tokens).clicked() {
                                if let Some(track) = preview.tracks.iter().find(|track| track.id == track_id) {
                                    self.export_xml_name_edit(&track.id, &track.name);
                                } else {
                                    self.notice = "Track Name export failed: selected track is no longer in the preview".into();
                                }
                            }
                        } else {
                            ui.label("Select one track above to edit its Name.");
                        }
                        ui.separator();
                        SectionHeading { title: "Folders" }.show(ui, tokens);
                        if preview.folders.is_empty() {
                            ui.label("No folders.");
                        }
                        for folder in &preview.folders {
                            ui.label(format!("{} · {}", folder.path.join(" / "), folder.name));
                        }
                        SectionHeading { title: "Playlists" }.show(ui, tokens);
                        if preview.playlists.is_empty() {
                            ui.label("No playlists.");
                        }
                        for playlist in &preview.playlists {
                            ui.label(format!(
                                "{} · {} ({} tracks)",
                                playlist.path.join(" / "),
                                playlist.name,
                                playlist.tracks.len()
                            ));
                        }
                        if !preview.unsupported.is_empty() {
                            StatusBadge {
                                label: "Unsupported content detected:",
                                kind: StatusKind::Warning,
                            }
                            .show(ui, tokens);
                            for warning in &preview.unsupported {
                                StatusBadge {
                                    label: warning,
                                    kind: StatusKind::Warning,
                                }
                                .show(ui, tokens);
                            }
                        }
                    }
                    None => {
                        ui.label("Enter an XML path and select Preview XML.");
                    }
                });
            self.show_xml_preview = open;
        }
        egui::SidePanel::left("library-navigation")
            .default_width(176.0)
            .resizable(true)
            .show(ctx, |ui| {
                SectionHeading {
                    title: "Collection",
                }
                .show(ui, tokens);
                if ui
                    .selectable_label(
                        self.availability_filter.is_none(),
                        format!("All tracks · {}", self.tracks.len()),
                    )
                    .clicked()
                {
                    self.availability_filter = None;
                }
                ui.separator();
                ui.small("LOCAL FILTERS");
                let available = self
                    .tracks
                    .iter()
                    .filter(|track| track.availability == TrackAvailability::Available)
                    .count();
                let missing = self.tracks.len().saturating_sub(available);
                if ui
                    .selectable_label(
                        self.availability_filter == Some(TrackAvailability::Available),
                        format!("Available · {available}"),
                    )
                    .clicked()
                {
                    self.availability_filter = Some(TrackAvailability::Available);
                }
                if ui
                    .selectable_label(
                        self.availability_filter == Some(TrackAvailability::Missing),
                        format!("Missing · {missing}"),
                    )
                    .clicked()
                {
                    self.availability_filter = Some(TrackAvailability::Missing);
                }
            });
        egui::TopBottomPanel::bottom("demo-decks")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.small("LOCAL DEMO · NO AUDIO CONNECTED");
                    for deck in 0..2 {
                        ui.separator();
                        ui.strong(format!("DECK {}", if deck == 0 { "A" } else { "B" }));
                        ui.label(
                            self.selected
                                .and_then(|id| {
                                    self.tracks
                                        .iter()
                                        .find(|track| track.track.id.as_u64() == id)
                                })
                                .map(|track| {
                                    PathBuf::from(&track.track.path)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                })
                                .unwrap_or_else(|| "No track selected".into()),
                        );
                        if ui
                            .selectable_label(
                                self.deck_playing[deck],
                                if self.deck_playing[deck] {
                                    "Playing · demo"
                                } else {
                                    "Play"
                                },
                            )
                            .clicked()
                        {
                            self.deck_playing[deck] = !self.deck_playing[deck];
                        }
                        if ui.selectable_label(self.deck_cue[deck], "Cue").clicked() {
                            self.deck_cue[deck] = !self.deck_cue[deck];
                        }
                    }
                });
            });
        egui::SidePanel::right("details")
            .default_width(300.0)
            .show(ctx, |ui| {
                SectionHeading {
                    title: "Track details",
                }
                .show(ui, tokens);
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
                        let (availability, kind) = match item.availability {
                            TrackAvailability::Available => ("Available", StatusKind::Success),
                            TrackAvailability::Missing => ("Missing", StatusKind::Warning),
                        };
                        StatusBadge {
                            label: &format!("Availability: {availability}"),
                            kind,
                        }
                        .show(ui, tokens);
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
                StatusBadge {
                    label: &format!("Catalog error: {error}"),
                    kind: StatusKind::Error,
                }
                .show(ui, tokens);
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
                .filter(|track| {
                    self.availability_filter
                        .is_none_or(|filter| track.availability == filter)
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
            SectionHeading { title: "Tracks" }.show(ui, tokens);
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
                        let (availability, kind) = match item.availability {
                            TrackAvailability::Available => ("Available", StatusKind::Success),
                            TrackAvailability::Missing => ("Missing", StatusKind::Warning),
                        };
                        StatusBadge {
                            label: availability,
                            kind,
                        }
                        .show(ui, tokens);
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

fn color_hex(color: egui::Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn parse_hex_color(value: &str) -> Option<egui::Color32> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    Some(egui::Color32::from_rgb(
        u8::from_str_radix(&digits[0..2], 16).ok()?,
        u8::from_str_radix(&digits[2..4], 16).ok()?,
        u8::from_str_radix(&digits[4..6], 16).ok()?,
    ))
}

#[cfg(test)]
mod keyboard_tests {
    use super::*;

    fn app() -> LibraryApp {
        LibraryApp::new(
            PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        )
    }

    #[test]
    fn guarded_name_export_reports_success_and_refuses_existing_output() {
        use std::{fs, time::SystemTime};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-name-edit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("library.xml");
        let output = root.join("edited.xml");
        let original =
            include_bytes!("../../../xtask/tests/fixtures/f02-track-year-edit/source.xml");
        fs::write(&source, original).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.xml_edit_name = "New & Improved".into();
        app.xml_edit_output = output.display().to_string();
        app.export_xml_name_edit("100", "A");
        assert!(
            app.notice.starts_with("Exported track 100 Name:"),
            "{}",
            app.notice
        );
        assert_eq!(fs::read(&source).unwrap(), original);
        let exported = fs::read_to_string(&output).unwrap();
        assert!(exported.contains("Name=\"New &amp; Improved\""));

        let existing = root.join("existing.xml");
        fs::write(&existing, b"keep me").unwrap();
        app.xml_edit_output = existing.display().to_string();
        app.export_xml_name_edit("7", "Old");
        assert!(
            app.notice.starts_with("Track Name export failed:"),
            "{}",
            app.notice
        );
        assert_eq!(fs::read(existing).unwrap(), b"keep me");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn name_export_reports_guard_refusal_when_preview_value_is_stale() {
        use std::{fs, time::SystemTime};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-name-stale-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("library.xml");
        let output = root.join("edited.xml");
        fs::write(&source, br#"<DJ_PLAYLISTS Version="1.0.0"><COLLECTION Entries="1"><TRACK TrackID="7" Name="Changed" Location="x"/></COLLECTION></DJ_PLAYLISTS>"#).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.xml_edit_output = output.display().to_string();
        app.xml_edit_name = "New".into();
        app.export_xml_name_edit("7", "Old");
        assert!(
            app.notice.starts_with("Track Name export failed:"),
            "{}",
            app.notice
        );
        assert!(!output.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn toolbar_controls_fit_narrow_and_wide_viewports() {
        for width in [800.0, 1440.0] {
            let ctx = egui::Context::default();
            let mut app = app();
            let mut rects = Vec::new();
            let _ = ctx.run(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0))),
                ..Default::default()
            }, |ctx| {
                egui::TopBottomPanel::top("test-toolbar").show(ctx, |ui| {
                    let (theme_rect, _) = app.theme_selection_ui(ui);
                    rects.push(theme_rect);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Catalog:");
                        let path = "/a/very/long/catalog/path/that/must/not/push/actions/offscreen/catalog.sqlite";
                        let path_width = (ui.available_width() - 68.0).clamp(80.0, 360.0);
                        rects.push(ui.add_sized([path_width, 20.0], egui::Label::new(egui::RichText::new(path).monospace()).truncate()).on_hover_text(path).rect);
                        rects.push(ui.button("Reload").rect);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Search");
                        rects.push(ui.add_sized([ui.available_width().clamp(100.0, 160.0), 24.0], egui::TextEdit::singleline(&mut String::new()).hint_text("Filter tracks")).rect);
                        ui.label("Import folder");
                        rects.push(ui.add_sized([ui.available_width().clamp(120.0, 200.0), 24.0], egui::TextEdit::singleline(&mut String::new()).hint_text("Folder path")).rect);
                        rects.push(ui.button("Import references").rect);
                    });
                });
            });
            let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
            assert!(
                rects.iter().all(|rect| content.contains_rect(*rect)),
                "toolbar control clipped at {width}px: {rects:?}"
            );
            assert!(
                rects.windows(2).all(|pair| !pair[0].intersects(pair[1])),
                "toolbar controls overlap at {width}px: {rects:?}"
            );
        }
    }

    #[test]
    fn day_to_custom_combo_selection_initializes_tokens_and_opens_editor() {
        let mut app = app();
        app.theme = ThemeMode::TokyoDay;
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.custom_tokens.secondary = egui::Color32::from_rgb(4, 5, 6);
        let ctx = egui::Context::default();
        let render = |ctx: &egui::Context, app: &mut LibraryApp, input| {
            let mut result = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    result = Some(app.theme_selection_ui(ui));
                });
            });
            result.unwrap()
        };
        let (combo_rect, _) = render(&ctx, &mut app, egui::RawInput::default());
        let click = |pos| egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let _ = render(&ctx, &mut app, click(combo_rect.center()));
        let (_, option_rects) = render(&ctx, &mut app, egui::RawInput::default());
        let custom_pos = option_rects.unwrap()[2].center();
        let _ = render(&ctx, &mut app, click(custom_pos));
        assert_eq!(app.theme, ThemeMode::Custom);
        assert!(app.show_theme_editor);
        assert_eq!(app.custom_tokens, ThemeMode::TokyoDay.tokens());
    }

    #[test]
    fn theme_controls_have_accesskit_names_and_control_roles() {
        let mut app = app();
        app.theme = ThemeMode::Custom;
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.theme_selection_ui(ui);
                app.show_palette_fields(ui, egui::Color32::RED);
            });
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit update");
        let nodes = update
            .nodes
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let assert_named = |name, role| {
            assert!(
                nodes.values().any(|control| {
                    control.role() == role
                        && control.labelled_by().iter().any(|label_id| {
                            nodes
                                .get(label_id)
                                .is_some_and(|label| label.value() == Some(name))
                        })
                }),
                "missing {role:?} labelled by {name:?}"
            );
        };
        assert_named("Theme (Ctrl+Alt+1/2/3)", egui::accesskit::Role::ComboBox);
        assert_named("Primary", egui::accesskit::Role::TextInput);
        assert_named("Secondary", egui::accesskit::Role::TextInput);
    }

    #[test]
    fn shortcut_raw_input_selects_custom_and_preserves_existing_custom_tokens() {
        let mut app = app();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
        assert_eq!(app.theme, ThemeMode::Custom);
        assert!(app.show_theme_editor);
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.show_theme_editor = false;
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
        assert!(app.show_theme_editor);
        assert_eq!(app.custom_tokens.primary, egui::Color32::from_rgb(1, 2, 3));
    }

    #[test]
    fn raw_text_events_commit_field_and_storage_round_trip() {
        use std::collections::HashMap;
        #[derive(Default)]
        struct Store(HashMap<String, String>);
        impl eframe::Storage for Store {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.into(), value);
            }
            fn flush(&mut self) {}
        }
        let mut app = app();
        app.select_theme(ThemeMode::Custom);
        let ctx = egui::Context::default();
        let run_editor = |ctx: &egui::Context, app: &mut LibraryApp, input| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| app.show_palette_fields(ui, egui::Color32::RED));
            })
        };
        app.primary_hex.clear();
        let _ = run_editor(&ctx, &mut app, egui::RawInput::default());
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = run_editor(&ctx, &mut app, input);
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Text("#123456".into()));
        let _ = run_editor(&ctx, &mut app, input);
        assert_eq!(app.primary_hex, "#123456");
        let mut enter = egui::RawInput::default();
        enter.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });
        let _ = run_editor(&ctx, &mut app, enter);
        assert_eq!(
            app.custom_tokens.primary,
            egui::Color32::from_rgb(0x12, 0x34, 0x56)
        );
        assert_eq!(app.theme, ThemeMode::Custom);
        let mut store = Store::default();
        eframe::App::save(&mut app, &mut store);
        assert_eq!(
            ThemeMode::load(Some(&store)),
            (app.theme, app.custom_tokens)
        );

        // Invalid focused input is rejected on Enter and leaves the committed token intact.
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
        let mut invalid = egui::RawInput::default();
        invalid.events.extend([
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            },
            egui::Event::Text("#12".into()),
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            },
        ]);
        let _ = run_editor(&ctx, &mut app, invalid);
        assert_eq!(
            app.custom_tokens.primary,
            egui::Color32::from_rgb(0x12, 0x34, 0x56)
        );
        assert!(app.accent_error.is_some());
    }

    #[test]
    fn palette_swatches_and_reset_keep_text_synchronized() {
        let mut app = app();
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.sync_accent_fields();
        assert_eq!(app.primary_hex, "#010203");
        app.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
        app.sync_accent_fields();
        assert_eq!(
            app.primary_hex,
            color_hex(ThemeMode::TokyoDay.tokens().primary)
        );
        assert_eq!(
            app.secondary_hex,
            color_hex(ThemeMode::TokyoDay.tokens().secondary)
        );
    }
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
