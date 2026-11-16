//! OxiDJ provisional local-library UI (prototype; outside the workspace).

mod app;
mod artwork;
mod beat_grid;
mod capture_preset;
mod components;
mod control_gallery;
mod deck_audio;
#[cfg(test)]
mod deck_tests;
mod decks;
mod folder_ops;
mod icons;
#[cfg(test)]
mod keyboard_tests;
mod mixer_view;
mod native_session;
mod overview_scale;
mod waveform_analysis;
// Addressed by the tests as `super::f02_playlist_member_insert_oracle`.
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_member_insert_oracle.rs"]
mod f02_playlist_member_insert_oracle;
mod playlist_ops;
mod settings;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod shell_tests;
mod theme;
mod track_edit;
mod track_waveform;
mod view;
mod waveform_view;
mod widgets;
mod xml_order;

use app::*;
use components::{PrimaryActionButton, SectionHeading, StatusBadge, StatusKind, text_field};
use eframe::egui;
use folder_ops::*;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use playlist_ops::*;
use std::path::PathBuf;
use theme::ThemeMode;
use track_edit::*;
use widgets::*;
use xml_order::*;

/// Headless frames have no renderer: explicitly consume the GPU texture work.
#[cfg(test)]
fn test_frame(
    ctx: &egui::Context,
    input: egui::RawInput,
    ui: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    output
}

fn main() -> eframe::Result<()> {
    // Validate the entire capture invocation before creating *any* engine.
    // With no capture flags, retain the original normal-startup CLI behavior.
    let args_os: Vec<_> = std::env::args_os().skip(1).collect();
    let capture = match capture_preset::cli(&args_os) {
        Ok(capture) => capture,
        Err(reason) => {
            eprintln!("capture: {reason}");
            std::process::exit(2);
        }
    };
    let (db, preset) = if let Some((db, preset)) = capture {
        (db, Some(preset))
    } else {
        let mut args = std::env::args().skip(1);
        let db = match (args.next().as_deref(), args.next()) {
            (Some("--db"), Some(path)) => PathBuf::from(path),
            _ => PathBuf::from("oxidj-library.sqlite"),
        };
        (db, None)
    };
    // Eframe opens storage before calling the AppCreator: App::save alone
    // cannot isolate window geometry, egui memory, or initial reads.
    let session = native_session::NativeSession::new(preset.is_some(), &db)
        .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    eframe::run_native(
        session.app_id,
        session.options,
        Box::new(move |cc| {
            cc.egui_ctx.enable_accesskit();
            let app = if let Some(preset) = preset {
                // Do not read saved preferences, environment output selection,
                // device discovery or CPAL in a capture session.
                cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                let theme = ThemeMode::TokyoNight;
                let mut app = LibraryApp::new_capture(db, theme, theme.tokens());
                app.decks
                    .attach(oxidj_audio::Engine::silent("capture session"));
                app.start_capture_preset(preset)
                    .map_err(std::io::Error::other)?;
                app
            } else {
                let (theme, custom_tokens) = ThemeMode::load(cc.storage);
                let mut app = LibraryApp::new(db, theme, custom_tokens);
                app.settings = settings::Settings::load(cc.storage);
                app.settings_ui.geom = settings::WindowGeom::load(cc.storage);
                let output = app
                    .settings
                    .output_config(oxidj_audio::OutputConfig::from_env());
                app.decks.attach(oxidj_audio::Engine::start_with(output));
                app
            };
            Ok(Box::new(app))
        }),
    )
}
