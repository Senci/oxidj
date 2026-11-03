//! OxiDJ provisional local-library UI (prototype; outside the workspace).

mod app;
mod beat_grid;
mod components;
mod control_gallery;
mod folder_ops;
#[cfg(test)]
mod keyboard_tests;
// Addressed by the tests as `super::f02_playlist_member_insert_oracle`.
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_member_insert_oracle.rs"]
mod f02_playlist_member_insert_oracle;
mod playlist_ops;
mod theme;
mod track_edit;
mod track_waveform;
mod view;
mod waveform_view;
mod widgets;
mod xml_order;

use app::*;
use components::{text_field, PrimaryActionButton, SectionHeading, StatusBadge, StatusKind};
use eframe::egui;
use folder_ops::*;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use playlist_ops::*;
use std::path::PathBuf;
use theme::ThemeMode;
use track_edit::*;
use widgets::*;
use xml_order::*;

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
