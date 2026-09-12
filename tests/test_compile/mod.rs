#[cfg(feature = "bits")]
use std::{env, fs, path::PathBuf};

#[test]
#[cfg(feature = "bits")]
#[cfg_attr(miri, ignore)]
fn test_compile() {
    let profile = env::var("DEKU_TRYBUILD_PROFILE").unwrap_or_else(|_| "stable".to_owned());
    let _snapshots = install_snapshots(&profile);

    let t = trybuild::TestCases::new();
    t.pass("tests/test_compile/pass_cases/*.rs");
    t.compile_fail("tests/test_compile/cases/*.rs");
}

#[cfg(feature = "bits")]
struct SnapshotGuard {
    originals: Vec<(PathBuf, Vec<u8>)>,
}

#[cfg(feature = "bits")]
impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        for (path, contents) in &self.originals {
            fs::write(path, contents).expect("restore trybuild snapshot");
        }
    }
}

#[cfg(feature = "bits")]
fn install_snapshots(profile: &str) -> SnapshotGuard {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_compile/cases");
    let mut originals = Vec::new();

    if profile == "stable" {
        return SnapshotGuard { originals };
    }

    for entry in fs::read_dir(&source_dir).expect("read trybuild case directory") {
        let source = entry.expect("read trybuild case entry").path();
        if source.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        let snapshot = source.with_extension("stderr");
        originals.push((
            snapshot.clone(),
            fs::read(&snapshot).expect("read trybuild snapshot"),
        ));

        let profile_snapshot = source.with_extension(format!("stderr.{profile}"));
        fs::copy(profile_snapshot, snapshot).expect("install trybuild snapshot");
    }

    SnapshotGuard { originals }
}
