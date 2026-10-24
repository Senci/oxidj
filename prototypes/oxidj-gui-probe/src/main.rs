use std::time::{Duration, Instant};

use eframe::egui::{self, ScrollArea};

struct Probe {
    count: usize,
    started: Instant,
    frames: u64,
}

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "Synthetic virtualized track list: {} rows",
                self.count
            ));
            ui.label(
                "Frame rate = rendered update callbacks / monotonic elapsed wall time; see report.",
            );
            ui.label(format!(
                "{} rendered frames; {:.1} FPS",
                self.frames,
                self.frames as f64 / self.started.elapsed().as_secs_f64().max(0.001)
            ));
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, 24.0, self.count, |ui, rows| {
                    for index in rows {
                        let label =
                            format!("Track {index:06} — Synthetic Artist — Synthetic Album");
                        ui.selectable_label(false, label)
                            .on_hover_text("Synthetic row; no media access");
                    }
                });
        });
        if self.started.elapsed() < Duration::from_secs(15) {
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result {
    let count = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OxiDJ isolated GUI probe",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Probe {
                count,
                started: Instant::now(),
                frames: 0,
            }))
        }),
    )
}
