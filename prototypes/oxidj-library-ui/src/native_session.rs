//! Native startup policy, determined before eframe can open persistence.
use std::path::Path;

pub(crate) struct NativeSession {
    pub(crate) app_id: &'static str,
    pub(crate) options: eframe::NativeOptions,
    // Must outlive run_native, including eframe's final persistence flush.
    _persistence: Option<tempfile::TempDir>,
}

impl NativeSession {
    pub(crate) fn new(capture: bool, db: &Path) -> std::io::Result<Self> {
        if capture {
            // Anchor the random private directory next to the explicitly supplied
            // disposable catalog, never in the user's normal eframe state tree.
            let canonical = db.canonicalize()?;
            if !db.is_absolute() || canonical != db || !db.symlink_metadata()?.is_file() {
                return Err(std::io::Error::other("capture catalog must be a canonical absolute regular file"));
            }
            let parent = canonical.parent().ok_or_else(|| std::io::Error::other("capture catalog has no parent"))?;
            let mut builder = tempfile::Builder::new();
            builder.prefix(".oxidj-capture-");
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt as _;
                builder.permissions(std::fs::Permissions::from_mode(0o700));
            }
            let private = builder.tempdir_in(parent)?;
            let dir = private.path().canonicalize()?;
            if dir.parent() != Some(parent) || !private.path().symlink_metadata()?.is_dir() {
                return Err(std::io::Error::other("capture persistence escaped disposable catalog directory"));
            }
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt as _;
                if dir.metadata()?.permissions().mode() & 0o777 != 0o700 {
                    return Err(std::io::Error::other("capture persistence is not private"));
                }
            }
            return Ok(Self {
                app_id: "OxiDJ Silent Capture",
                options: eframe::NativeOptions {
                    viewport: eframe::egui::ViewportBuilder::default()
                        .with_app_id("OxiDJ Silent Capture")
                        .with_maximized(true),
                    persistence_path: Some(dir.join("app.ron")),
                    persist_window: false,
                    ..Default::default()
                },
                _persistence: Some(private),
            });
        }
        Ok(Self {
            app_id: "OxiDJ Local Library",
            options: eframe::NativeOptions::default(),
            _persistence: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_native_options_isolate_framework_storage_before_creation() {
        let root = crate::deck_tests::temp_root("native-persistence");
        let db = root.join("disposable.sqlite");
        std::fs::write(&db, b"synthetic").unwrap();
        let normal = NativeSession::new(false, &db).unwrap();
        let capture = NativeSession::new(true, &db).unwrap();
        assert_ne!(capture.app_id, normal.app_id);
        assert!(!capture.options.persist_window);
        assert!(normal.options.persist_window);
        assert!(normal.options.persistence_path.is_none());
        let path = capture.options.persistence_path.clone().expect("no eframe default storage fallback");
        assert!(path.is_absolute());
        assert_ne!(path.parent().unwrap(), root);
        assert_eq!(path.parent().unwrap().parent().unwrap(), root);
        let other = NativeSession::new(true, &db).unwrap();
        assert_ne!(Some(&path), other.options.persistence_path.as_ref());
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
        }
        std::fs::write(&path, b"framework scratch").unwrap();
        drop(capture);
        assert!(!path.parent().unwrap().exists(), "session owns and removes persistence");
        drop(other);
        std::fs::remove_dir_all(root).unwrap();
    }
}
