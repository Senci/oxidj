//! No-clobber file publication shared by every export path.

use std::{fs, io, path::Path};

/// Why a no-clobber publish failed.
#[derive(Debug)]
pub enum PublishError {
    /// The destination already exists; it was left untouched.
    DestinationExists,
    /// Any other I/O failure; no partial destination is left behind.
    Io(io::Error),
}

impl core::fmt::Display for PublishError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DestinationExists => f.write_str("destination already exists"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PublishError {}

impl From<io::Error> for PublishError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Publish the complete `temporary` file at `destination` without ever
/// replacing an existing path.
///
/// Prefers an atomic same-directory hard link. Filesystems without hard links
/// (FAT/exFAT USB sticks) fall back to an `O_EXCL` create plus copy; a failed
/// copy removes the destination this call created. On success the temporary
/// file is removed best-effort; on failure it is left for the caller to clean.
pub fn publish_no_clobber(temporary: &Path, destination: &Path) -> Result<(), PublishError> {
    publish_with(temporary, destination, true)
}

fn publish_with(
    temporary: &Path,
    destination: &Path,
    try_hard_link: bool,
) -> Result<(), PublishError> {
    if try_hard_link {
        match fs::hard_link(temporary, destination) {
            Ok(()) => {
                let _ = fs::remove_file(temporary);
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(PublishError::DestinationExists);
            }
            // Unsupported / not permitted / cross-device: use the copy path.
            Err(_) => {}
        }
    }
    let mut target = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(PublishError::DestinationExists);
        }
        Err(error) => return Err(error.into()),
    };
    let copied = (|| {
        let mut source = fs::File::open(temporary)?;
        io::copy(&mut source, &mut target)?;
        target.sync_all()
    })();
    drop(target);
    if let Err(error) = copied {
        // This call created the destination, so removing it is safe.
        let _ = fs::remove_file(destination);
        return Err(error.into());
    }
    let _ = fs::remove_file(temporary);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("oxidj-core-publish-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn both_paths_publish_and_refuse_existing_without_overwrite() {
        for hard in [true, false] {
            let d = dir(&format!("ok{hard}"));
            let (tmp, dest) = (d.join("t"), d.join("out"));
            fs::write(&tmp, b"new").unwrap();
            publish_with(&tmp, &dest, hard).unwrap();
            assert_eq!(fs::read(&dest).unwrap(), b"new");
            assert!(!tmp.exists());
            fs::write(&tmp, b"other").unwrap();
            let err = publish_with(&tmp, &dest, hard).unwrap_err();
            assert!(matches!(err, PublishError::DestinationExists));
            assert_eq!(fs::read(&dest).unwrap(), b"new");
            assert!(tmp.exists(), "temp left for caller on refusal");
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn failure_leaves_no_partial_destination() {
        for hard in [true, false] {
            let d = dir(&format!("fail{hard}"));
            let dest = d.join("out");
            let err = publish_with(&d.join("missing"), &dest, hard).unwrap_err();
            assert!(matches!(err, PublishError::Io(_)));
            assert!(!dest.exists());
            fs::remove_dir_all(d).unwrap();
        }
    }
}
