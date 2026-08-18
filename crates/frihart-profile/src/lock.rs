use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use frihart_core::{FrihartError, Result};

/// Best-effort pid lock. Enough to stop two chrome processes writing
/// the same profile. Not a distributed lock.
pub struct ProfileLock {
    path: PathBuf,
    _file: File,
}

impl ProfileLock {
    pub fn acquire(path: PathBuf) -> Result<Self> {
        if path.exists() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(pid) = text.trim().parse::<u32>() {
                    if pid_is_alive(pid) {
                        return Err(FrihartError::ProfileLocked { pid });
                    }
                }
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        write!(file, "{}", std::process::id())?;
        file.flush()?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for ProfileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn pid_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stale_lock_is_replaced() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("frihart-lock-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lock");
        // A pid that will not exist on this machine.
        fs::write(&path, "4294967294\n").unwrap();
        let lock = ProfileLock::acquire(path.clone()).unwrap();
        drop(lock);
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
