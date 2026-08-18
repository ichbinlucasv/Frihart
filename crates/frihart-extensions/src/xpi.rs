//! Read an `.xpi` (ZIP) or an unpacked extension directory.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use zip::ZipArchive;

use frihart_core::{FrihartError, Result};

use crate::manifest::Manifest;

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
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| FrihartError::config(format!("xpi entry: {e}")))?;
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
        let mut dest_file = File::create(&out)?;
        std::io::copy(&mut entry, &mut dest_file)?;
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
