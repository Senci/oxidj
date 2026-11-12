//! Fail-closed local destination admission. Linux util-linux resolves the actual
//! mount (including bind mounts/Btrfs sources) and every backing block ancestor.
//! No shell, device writes, or dependency on the PDB's directory layout.
use serde_json::Value;
use std::{ffi::OsString, path::Path};

pub(crate) const REFUSAL: &str = "destination must be on verified local non-removable storage (not USB); storage classification failed or destination is unsupported";

pub(crate) fn verify(parent: &Path) -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    verify_with(parent, |program, args| {
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    })
}

pub(crate) fn verify_with(
    parent: &Path,
    run: impl Fn(&str, Vec<OsString>) -> Option<Value>,
) -> bool {
    let Some(mount) = run(
        "findmnt",
        vec![
            "--json".into(),
            "--target".into(),
            parent.into(),
            "--output".into(),
            "SOURCES,FSTYPE".into(),
        ],
    ) else {
        return false;
    };
    let Some(mounts) = mount["filesystems"].as_array() else {
        return false;
    };
    if mounts.len() != 1 {
        return false;
    }
    let mount = &mounts[0];
    // Memory-backed storage has no removable backing device. Unknown, network,
    // FUSE and overlay mounts cannot establish the local-only guarantee.
    if mount["fstype"] == "tmpfs" {
        return true;
    }
    if !matches!(
        mount["fstype"].as_str(),
        Some("ext2" | "ext3" | "ext4" | "btrfs" | "xfs" | "f2fs" | "vfat" | "exfat" | "ntfs3")
    ) {
        return false;
    }
    let Some(sources) = mount["sources"].as_array().filter(|a| !a.is_empty()) else {
        return false;
    };
    sources.iter().all(|source| {
        let Some(source) = source.as_str() else {
            return false;
        };
        // findmnt decorates bind/subvolume sources with [/sub/path].
        let source = source.split('[').next().unwrap_or("");
        if !source.starts_with("/dev/") {
            return false;
        }
        let Some(devices) = run(
            "lsblk",
            vec![
                "--inverse".into(),
                "--tree".into(),
                "--json".into(),
                "--output".into(),
                "TYPE,TRAN,RM,HOTPLUG".into(),
                "--".into(),
                source.into(),
            ],
        ) else {
            return false;
        };
        let Some(devices) = devices["blockdevices"].as_array().filter(|a| !a.is_empty()) else {
            return false;
        };
        devices.iter().all(local_device)
    })
}

fn local_device(device: &Value) -> bool {
    if device["rm"] != false || device["hotplug"] != false {
        return false;
    }
    // USB disks can report RM=0. Require a known internal transport at leaves;
    // mapper/RAID stacks must expose and validate ALL their backing ancestors.
    if let Some(children) = device["children"].as_array().filter(|a| !a.is_empty()) {
        return device["tran"] != "usb" && children.iter().all(local_device);
    }
    matches!(device["type"].as_str(), Some("disk" | "part"))
        && matches!(
            device["tran"].as_str(),
            Some("nvme" | "sata" | "ata" | "sas" | "virtio")
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classification_checks_all_backing_devices_and_fails_closed() {
        let local = json!({"type":"disk","tran":"nvme","rm":false,"hotplug":false});
        let usb_fixed = json!({"type":"disk","tran":"usb","rm":false,"hotplug":false});
        let removable = json!({"type":"disk","tran":"sata","rm":true,"hotplug":false});
        let hotplug = json!({"type":"disk","tran":"sata","rm":false,"hotplug":true});
        let mapper = |child: Value| json!({"type":"crypt","tran":null,"rm":false,"hotplug":false,"children":[child]});
        for (device, expected) in [
            (local.clone(), true),
            (usb_fixed.clone(), false),
            (removable, false),
            (hotplug, false),
            (mapper(local.clone()), true),
            (mapper(usb_fixed.clone()), false),
            (
                json!({"type":"crypt","tran":null,"rm":false,"hotplug":false}),
                false,
            ),
        ] {
            assert_eq!(local_device(&device), expected, "{device}");
        }
        let mount = json!({"filesystems":[{"fstype":"btrfs","sources":["/dev/mapper/root[/@home]","/dev/sdb"]}]});
        assert!(!verify_with(Path::new("/local"), |program, args| {
            if program == "findmnt" {
                return Some(mount.clone());
            }
            let device = if args.last().unwrap() == "/dev/sdb" {
                usb_fixed.clone()
            } else {
                local.clone()
            };
            Some(json!({"blockdevices":[device]}))
        }));
        for fstype in ["nfs", "fuseblk", "overlay", "proc"] {
            assert!(!verify_with(Path::new("/local"), |_, _| Some(
                json!({"filesystems":[{"fstype":fstype,"sources":["/dev/sda"]}]})
            )));
        }
        assert!(!verify_with(Path::new("/local"), |_, _| None));
        assert!(!verify_with(Path::new("/local"), |_, _| Some(json!({}))));
    }
}
