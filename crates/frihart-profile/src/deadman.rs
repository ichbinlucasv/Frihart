//! Dead-man switch: shred the profile if it was not opened for N days.
//!
//! The setting and the time of the last open live in a small plain file,
//! `deadman`, in the profile directory. It holds a day count and a timestamp,
//! nothing else. Same format and rules as HashChat's switch, with a different
//! magic so the two files are never confused.
//!
//! Limits: the check only runs when Frihart opens the profile. If the browser
//! is never started, nothing happens, and a copy of the profile made earlier is
//! not affected. Until the profile is encrypted and has an unlock prompt, any
//! start resets the timer, including one by someone else at the keyboard. A
//! clock set far forward triggers the shred early; a clock set back only
//! delays it.

use std::fs;
use std::io::Read;
use std::path::Path;

use frihart_core::{Result, write_private_str};

pub(crate) const DEADMAN_FILE: &str = "deadman";
const MAGIC: &str = "FHDM1";
const MAX_FILE_BYTES: u64 = 128;

pub const MIN_DEADMAN_DAYS: u32 = 1;
pub const MAX_DEADMAN_DAYS: u32 = 365;
const SECS_PER_DAY: u64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeadmanConfig {
    pub days: u32,
    pub last_open_unix: u64,
}

impl DeadmanConfig {
    /// Seconds left before the shred fires, 0 if already due.
    pub fn remaining_secs(&self, now_unix: u64) -> u64 {
        let deadline = self
            .last_open_unix
            .saturating_add(u64::from(self.days) * SECS_PER_DAY);
        deadline.saturating_sub(now_unix)
    }

    /// Due once `days` full days have passed since the last open. A clock
    /// that reads earlier than the recorded time never counts as due.
    pub fn is_due(&self, now_unix: u64) -> bool {
        now_unix >= self.last_open_unix && self.remaining_secs(now_unix) == 0
    }
}

fn encode(cfg: &DeadmanConfig) -> String {
    format!("{MAGIC} {} {}\n", cfg.days, cfg.last_open_unix)
}

fn decode(raw: &[u8]) -> Option<DeadmanConfig> {
    let text = std::str::from_utf8(raw).ok()?;
    let mut parts = text.trim_end().split(' ');
    if parts.next()? != MAGIC {
        return None;
    }
    let days: u32 = parts.next()?.parse().ok()?;
    let last_open_unix: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(MIN_DEADMAN_DAYS..=MAX_DEADMAN_DAYS).contains(&days) {
        return None;
    }
    Some(DeadmanConfig {
        days,
        last_open_unix,
    })
}

/// Reads the file only if it is a regular file of sane size. A symlink in its
/// place is not followed.
fn read_file(root: &Path) -> Option<Vec<u8>> {
    let path = root.join(DEADMAN_FILE);
    let meta = fs::symlink_metadata(&path).ok()?;
    if !meta.file_type().is_file() || meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut raw = Vec::new();
    fs::File::open(&path)
        .ok()?
        .take(MAX_FILE_BYTES)
        .read_to_end(&mut raw)
        .ok()?;
    Some(raw)
}

/// Current setting, or `None` if unset or unreadable.
pub fn deadman_config(root: &Path) -> Option<DeadmanConfig> {
    decode(&read_file(root)?)
}

/// True if something sits at the dead-man path at all, even a damaged file.
pub fn deadman_present(root: &Path) -> bool {
    fs::symlink_metadata(root.join(DEADMAN_FILE)).is_ok()
}

/// Turn the switch on for `days` days, counting from `now_unix`.
pub fn set_deadman(root: &Path, days: u32, now_unix: u64) -> Result<()> {
    if !(MIN_DEADMAN_DAYS..=MAX_DEADMAN_DAYS).contains(&days) {
        return Err(frihart_core::FrihartError::Message(format!(
            "dead-man days must be between {MIN_DEADMAN_DAYS} and {MAX_DEADMAN_DAYS}"
        )));
    }
    let cfg = DeadmanConfig {
        days,
        last_open_unix: now_unix,
    };
    write_private_str(&root.join(DEADMAN_FILE), &encode(&cfg))
}

/// Turn the switch off. A missing file is fine.
pub fn clear_deadman(root: &Path) -> Result<()> {
    match fs::remove_file(root.join(DEADMAN_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Startup check. Returns true if the switch is set and due, or if the file is
/// damaged: an attacker could otherwise defeat the switch by corrupting it.
/// When not due, records this open as the new starting point.
pub(crate) fn check_on_open(root: &Path, now_unix: u64) -> bool {
    if !deadman_present(root) {
        return false;
    }
    match deadman_config(root) {
        Some(cfg) if !cfg.is_due(now_unix) => {
            // Never move the starting point back when the clock reads early.
            let start = now_unix.max(cfg.last_open_unix);
            let _ = set_deadman(root, cfg.days, start);
            false
        }
        _ => true,
    }
}

/// Seconds since the Unix epoch; 0 if the clock is before it.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const DAY: u64 = SECS_PER_DAY;

    fn tmp_dir(tag: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("frihart-deadman-{tag}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn set_read_and_clear() {
        let dir = tmp_dir("roundtrip");
        assert!(deadman_config(&dir).is_none());
        set_deadman(&dir, 14, 1_000).unwrap();
        assert_eq!(
            deadman_config(&dir),
            Some(DeadmanConfig {
                days: 14,
                last_open_unix: 1_000
            })
        );
        clear_deadman(&dir).unwrap();
        clear_deadman(&dir).unwrap();
        assert!(!deadman_present(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_out_of_range_days() {
        let dir = tmp_dir("range");
        assert!(set_deadman(&dir, 0, 1).is_err());
        assert!(set_deadman(&dir, 366, 1).is_err());
        assert!(!deadman_present(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn due_after_full_days_only() {
        let cfg = DeadmanConfig {
            days: 2,
            last_open_unix: 10 * DAY,
        };
        assert!(!cfg.is_due(11 * DAY));
        assert!(!cfg.is_due(12 * DAY - 1));
        assert!(cfg.is_due(12 * DAY));
        assert!(!cfg.is_due(5 * DAY), "clock set back is never due");
        assert_eq!(cfg.remaining_secs(11 * DAY), DAY);
    }

    #[test]
    fn open_before_deadline_moves_it() {
        let dir = tmp_dir("touch");
        set_deadman(&dir, 3, 0).unwrap();
        assert!(!check_on_open(&dir, 2 * DAY));
        assert_eq!(deadman_config(&dir).unwrap().last_open_unix, 2 * DAY);
        assert!(!check_on_open(&dir, 4 * DAY));
        assert!(check_on_open(&dir, 8 * DAY));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn early_clock_does_not_move_start_back() {
        let dir = tmp_dir("early");
        set_deadman(&dir, 3, 10 * DAY).unwrap();
        assert!(!check_on_open(&dir, DAY));
        assert_eq!(deadman_config(&dir).unwrap().last_open_unix, 10 * DAY);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_or_replaced_file_counts_as_due() {
        let dir = tmp_dir("damaged");
        assert!(!check_on_open(&dir, 1));
        fs::write(dir.join(DEADMAN_FILE), "FHDM1 seven 0\n").unwrap();
        assert!(check_on_open(&dir, 1));
        fs::write(dir.join(DEADMAN_FILE), "HCDM1 7 0\n").unwrap();
        assert!(check_on_open(&dir, 1), "HashChat's file is not ours");
        fs::remove_file(dir.join(DEADMAN_FILE)).unwrap();
        #[cfg(unix)]
        {
            let target = dir.join("elsewhere");
            fs::write(&target, "FHDM1 7 0\n").unwrap();
            std::os::unix::fs::symlink(&target, dir.join(DEADMAN_FILE)).unwrap();
            assert!(check_on_open(&dir, 1), "a symlink is not followed");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
