//! Save bytes to a user directory. Never execute.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use frihart_core::{FrihartError, Result};

use crate::Download;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadLog {
    #[serde(default)]
    pub items: Vec<DownloadRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub url: String,
    pub dest: String,
    pub bytes: u64,
}

impl DownloadLog {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(|e| FrihartError::profile(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| FrihartError::profile(e.to_string()))?;
        frihart_core::write_private_str(path, &text)
    }

    pub fn push(&mut self, rec: DownloadRecord) {
        self.items.insert(0, rec);
        self.items.truncate(200);
    }
}

pub fn should_save(content_type: &str, headers: &[(String, String)]) -> bool {
    if disposition_attachment(headers) {
        return true;
    }
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("html") || ct.contains("xhtml") {
        return false;
    }
    if ct.starts_with("text/") {
        return false;
    }
    if ct.is_empty() {
        return false;
    }
    ct.starts_with("image/")
        || ct.starts_with("audio/")
        || ct.starts_with("video/")
        || ct.starts_with("application/")
        || ct.contains("octet-stream")
}

fn disposition_attachment(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(n, v)| {
        n.eq_ignore_ascii_case("content-disposition")
            && v.to_ascii_lowercase().contains("attachment")
    })
}

pub fn filename_for(url: &Url, content_disposition: Option<&str>) -> String {
    if let Some(disp) = content_disposition {
        if let Some(name) = filename_from_disposition(disp) {
            return sanitize_filename(&name);
        }
    }
    let last = url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or("download");
    sanitize_filename(last)
}

fn filename_from_disposition(disp: &str) -> Option<String> {
    let lower = disp.to_ascii_lowercase();
    let idx = lower.find("filename=")?;
    let rest = disp[idx + "filename=".len()..].trim();
    let rest = rest.trim_matches('"').trim_matches('\'');
    let rest = rest.split(';').next().unwrap_or(rest).trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("download").trim();
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." || cleaned.starts_with('.') {
        "download".into()
    } else {
        cleaned
    }
}

pub fn unique_dest(dir: &Path, name: &str) -> PathBuf {
    let dest = dir.join(name);
    if !dest.exists() {
        return dest;
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    for i in 1..1000 {
        let candidate = if ext.is_empty() {
            dir.join(format!("{stem}-{i}"))
        } else {
            dir.join(format!("{stem}-{i}.{ext}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-overflow"))
}

pub fn save_download(
    dir: &Path,
    url: &Url,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<Download> {
    if dir.as_os_str().is_empty() {
        return Err(FrihartError::network("download dir missing"));
    }
    std::fs::create_dir_all(dir)?;
    let disp = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-disposition"))
        .map(|(_, v)| v.as_str());
    let name = filename_for(url, disp);
    let dest = unique_dest(dir, &name);
    write_file_0600(&dest, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&dest)?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o111 != 0 {
            return Err(FrihartError::network("download execute bit refused"));
        }
    }
    Ok(Download {
        url: url.clone(),
        dest,
        bytes: body.len() as u64,
    })
}

fn write_file_0600(path: &Path, data: &[u8]) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let tmp = path.with_extension("part");
    {
        let mut opts = OpenOptions::new();
        opts.create(true).write(true).truncate(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut file = opts.open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn html_is_not_a_download() {
        assert!(!should_save("text/html", &[]));
        assert!(should_save(
            "application/pdf",
            &[(
                "content-disposition".into(),
                "attachment; filename=a.pdf".into()
            )]
        ));
        assert_eq!(
            filename_for(
                &Url::parse("https://ex.test/files/report.pdf").unwrap(),
                None
            ),
            "report.pdf"
        );
    }

    #[test]
    fn writes_0600_and_never_executes() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("frihart-dl-{stamp}"));
        let url = Url::parse("https://ex.test/a.bin").unwrap();
        let d = save_download(&dir, &url, &[], b"hello").unwrap();
        assert!(!d.may_execute());
        assert_eq!(std::fs::read(&d.dest).unwrap(), b"hello");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&d.dest).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
