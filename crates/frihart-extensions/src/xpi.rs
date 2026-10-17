//! Read an `.xpi` (ZIP) or an unpacked extension directory.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use zip::ZipArchive;

use frihart_core::{FrihartError, Result};

use crate::manifest::Manifest;

/// Most entries an add-on archive may hold.
pub const MAX_XPI_ENTRIES: usize = 10_000;
/// Most bytes extracted from one add-on archive (zip-bomb cap).
pub const MAX_XPI_BYTES: u64 = 256 * 1024 * 1024;

pub fn read_manifest_from_path(path: &Path) -> Result<Manifest> {
    if path.is_dir() {
        let text = fs::read_to_string(path.join("manifest.json"))?;
        return Manifest::from_json(&text);
    }
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xpi") || e.eq_ignore_ascii_case("zip"))
    {
        return read_manifest_from_xpi(path);
    }
    Err(FrihartError::config(format!(
        "not an unpacked add-on or .xpi: {}",
        path.display()
    )))
}

fn read_manifest_from_xpi(path: &Path) -> Result<Manifest> {
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| FrihartError::config(format!("xpi zip: {e}")))?;
    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|e| FrihartError::config(format!("xpi missing manifest.json: {e}")))?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Manifest::from_json(&text)
}

/// Copy an unpacked dir or extract an XPI into `dest`.
///
/// Extraction is done by hand rather than with `ZipArchive::extract`:
/// entries whose names escape `dest` are skipped, symlink entries are
/// skipped (never created), Unix permission bits from the archive are never
/// applied (files get the process umask, nothing is executable by design),
/// and entry count and total size are capped.
pub fn materialize(source: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    if source.is_dir() {
        copy_dir(source, dest)?;
        return Ok(());
    }
    let file = File::open(source)?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| FrihartError::config(format!("xpi zip: {e}")))?;
    if archive.len() > MAX_XPI_ENTRIES {
        return Err(FrihartError::config("xpi: too many entries"));
    }
    let mut budget = MAX_XPI_BYTES;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| FrihartError::config(format!("xpi entry: {e}")))?;
        if entry.is_symlink() {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        // Never write through a path that already exists as a symlink.
        if fs::symlink_metadata(&out).is_ok_and(|m| m.file_type().is_symlink()) {
            continue;
        }
        let mut dest_file = File::create(&out)?;
        // Read one byte past the budget so an over-size entry is detected
        // from the real stream, not the size the archive claims.
        let copied = std::io::copy(&mut entry.take(budget + 1), &mut dest_file)?;
        if copied > budget {
            drop(dest_file);
            let _ = fs::remove_dir_all(dest);
            return Err(FrihartError::config(
                "xpi: archive too large when extracted",
            ));
        }
        budget -= copied;
        dest_file.flush()?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn scratch(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "frihart-xpi-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_xpi(
        dir: &Path,
        build: impl FnOnce(&mut ZipWriter<Cursor<Vec<u8>>>),
    ) -> std::path::PathBuf {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        build(&mut w);
        let bytes = w.finish().unwrap().into_inner();
        let p = dir.join("addon.xpi");
        fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn extracts_files_but_skips_symlinks_and_escapes() {
        let dir = scratch("sym");
        let opts = SimpleFileOptions::default().unix_permissions(0o4755);
        let xpi = write_xpi(&dir, |w| {
            w.start_file("manifest.json", opts).unwrap();
            w.write_all(br#"{"manifest_version":2,"name":"t","version":"1"}"#)
                .unwrap();
            w.add_symlink("link", "/etc/passwd", opts).unwrap();
            w.start_file("../escape.txt", opts).unwrap();
            w.write_all(b"x").unwrap();
            w.start_file("js/a.js", opts).unwrap();
            w.write_all(b"1;").unwrap();
        });
        let dest = dir.join("out");
        materialize(&xpi, &dest).unwrap();
        assert!(dest.join("manifest.json").is_file());
        assert_eq!(fs::read(dest.join("js/a.js")).unwrap(), b"1;");
        assert!(fs::symlink_metadata(dest.join("link")).is_err());
        assert!(!dir.join("escape.txt").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dest.join("js/a.js"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o7000, 0, "setuid/setgid/sticky never applied");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_reads_from_xpi() {
        let dir = scratch("manifest");
        let xpi = write_xpi(&dir, |w| {
            w.start_file("manifest.json", SimpleFileOptions::default())
                .unwrap();
            w.write_all(br#"{"manifest_version":2,"name":"t","version":"1"}"#)
                .unwrap();
        });
        assert!(read_manifest_from_path(&xpi).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }
}
