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

/// Overwrite a regular file, then unlink it. A symlink is unlinked without
/// touching its target, so a link planted in a profile cannot redirect the wipe.
pub fn shred_file(path: &Path) -> Result<()> {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if meta.file_type().is_symlink() {
        fs::remove_file(path)?;
        return Ok(());
    }
    if !meta.is_file() {
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

/// Like [`shred_file`] for a directory tree. Symlinks are unlinked, never followed.
pub fn shred_tree(path: &Path) -> Result<()> {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if !meta.is_dir() {
        return shred_file(path);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if entry.file_type()?.is_dir() {
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

#[cfg(test)]
mod shred_tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("frihart-shred-{tag}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn shred_file_removes_a_regular_file() {
        let dir = tmp("file");
        let f = dir.join("a.txt");
        fs::write(&f, b"secret").unwrap();
        shred_file(&f).unwrap();
        assert!(!f.exists());
        // Missing path is fine.
        shred_file(&f).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn shred_never_follows_symlinks() {
        let dir = tmp("link");
        let outside = tmp("outside");
        let target_file = outside.join("keep.txt");
        fs::write(&target_file, b"keep me").unwrap();
        let tree = dir.join("tree");
        fs::create_dir_all(&tree).unwrap();
        std::os::unix::fs::symlink(&target_file, tree.join("file-link")).unwrap();
        std::os::unix::fs::symlink(&outside, tree.join("dir-link")).unwrap();

        shred_tree(&tree).unwrap();

        assert!(!tree.exists(), "tree and its links are gone");
        assert_eq!(fs::read(&target_file).unwrap(), b"keep me");
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&outside);
    }
}
