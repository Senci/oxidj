use eframe::egui;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use std::path::PathBuf;

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
}
impl LibraryApp {
    fn new(db_path: PathBuf) -> Self {
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
        };
        app.reload();
        app
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.heading("OxiDJ · Local Library");
            ui.horizontal(|ui| {
                ui.label("Catalog:");
                ui.monospace(self.db_path.display().to_string());
                if ui.button("Reload").clicked() {
                    self.reload();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Search");
                ui.text_edit_singleline(&mut self.search);
                ui.label("Import folder");
                ui.text_edit_singleline(&mut self.import_root);
                if ui.button("Import references").clicked() {
                    self.import();
                }
            });
            if !self.notice.is_empty() {
                ui.label(&self.notice);
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
                        ui.label(format!("Availability: {:?}", item.availability));
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
                ui.colored_label(egui::Color32::RED, format!("Catalog error: {error}"));
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
                        ui.label(match item.availability {
                            TrackAvailability::Available => "Available",
                            TrackAvailability::Missing => "Missing",
                        });
                        ui.end_row();
                    }
                });
            });
        });
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
            Ok(Box::new(LibraryApp::new(db)))
        }),
    )
}
