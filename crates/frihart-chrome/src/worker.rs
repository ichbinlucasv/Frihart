//! One-shot content worker. Chrome never applies the sandbox itself.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use frihart_pipeline::{LayoutJob, LayoutOut, execute};

const WORKER_FLAG: &str = "--content-worker";

pub fn layout_isolated(job: &LayoutJob) -> LayoutOut {
    match spawn_worker(job) {
        Ok(out) => out,
        Err(_) => {
            let mut out = execute(job);
            out.detail = "worker failed; in-process fallback".into();
            out.sandboxed = false;
            out
        }
    }
}

fn spawn_worker(job: &LayoutJob) -> Result<LayoutOut, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if exe.file_name().and_then(|s| s.to_str()) != Some("frihart") {
        return Err("not the frihart binary".into());
    }
    let payload = serde_json::to_vec(job).map_err(|e| e.to_string())?;
    let mut child = Command::new(exe)
        .arg(WORKER_FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    {
        let mut stdin = child.stdin.take().ok_or("stdin")?;
        stdin.write_all(&payload).map_err(|e| e.to_string())?;
    }
    let timeout = Duration::from_secs(8);
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            let _ = child.kill();
            return Err("timeout".into());
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err("worker exit".into()),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut stdout = child.stdout.take().ok_or("stdout")?;
    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_lays_out() {
        let out = layout_isolated(&LayoutJob {
            html: "<p>hello worker</p>".into(),
            extra_css: String::new(),
            viewport_w: 320.0,
        });
        assert!(out.display.find("hello").is_some());
    }
}
