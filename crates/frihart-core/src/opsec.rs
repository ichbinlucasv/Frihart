//! Local file permissions. No world-readable profile data.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::Result;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const FILE_MODE: u32 = 0o600;
const DIR_MODE: u32 = 0o700;

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    lockdown(path, DIR_MODE)?;
    Ok(())
}

pub fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut opts = OpenOptions::new();
        opts.create(true).write(true).truncate(true);
        #[cfg(unix)]
        opts.mode(FILE_MODE);
        let mut file = opts.open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    lockdown(path, FILE_MODE)?;
    Ok(())
}

pub fn write_private_str(path: &Path, text: &str) -> Result<()> {
    write_private(path, text.as_bytes())
}

fn lockdown(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms)?;
    }
    let _ = mode;
    let _ = path;
    Ok(())
}

/// Host only. Never userinfo, path, or query.
pub fn safe_host(url: &url::Url) -> String {
    url.host_str().unwrap_or("-").to_string()
}

pub fn shred_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let len = fs::metadata(path)?.len();
    let mut file = OpenOptions::new().write(true).open(path)?;
    let mut chunk = vec![0u8; 65_536];
    for _ in 0..3 {
        fill_random(&mut chunk)?;
        file.seek(SeekFrom::Start(0))?;
        let mut left = len;
        while left > 0 {
            let n = chunk.len().min(left as usize);
            file.write_all(&chunk[..n])?;
            left -= n as u64;
        }
        file.sync_all()?;
    }
    chunk.fill(0);
    file.seek(SeekFrom::Start(0))?;
    let mut left = len;
    while left > 0 {
        let n = chunk.len().min(left as usize);
        file.write_all(&chunk[..n])?;
        left -= n as u64;
    }
    file.sync_all()?;
    drop(file);
    fs::remove_file(path)?;
    Ok(())
}

pub fn shred_tree(path: &Path) -> Result<()> {
    if path.is_file() {
        return shred_file(path);
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            shred_tree(&p)?;
        } else {
            shred_file(&p)?;
        }
    }
    let _ = fs::remove_dir(path);
    Ok(())
}

fn fill_random(buf: &mut [u8]) -> Result<()> {
    let mut src = File::open("/dev/urandom")?;
    src.read_exact(buf)?;
    Ok(())
}

pub fn sanitize_error(msg: &str) -> String {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("key") || lower.contains("password") || lower.contains("authorization") {
        return "request failed".into();
    }
    msg.lines()
        .next()
        .unwrap_or("error")
        .chars()
        .take(120)
        .collect()
}
