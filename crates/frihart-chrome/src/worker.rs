//! Long-lived content workers, one process per isolation key.
//! Chrome never applies the sandbox itself.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use frihart_core::IsolationKey;
use frihart_pipeline::{LayoutJob, LayoutOut, execute};

const WORKER_FLAG: &str = "--content-worker";

#[derive(Default)]
pub struct WorkerPool {
    by_key: HashMap<IsolationKey, PersistentWorker>,
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.drop_all();
    }
}

impl WorkerPool {
    pub fn layout(&mut self, key: IsolationKey, job: &LayoutJob) -> LayoutOut {
        if let Some(w) = self.by_key.get_mut(&key) {
            if let Ok(out) = w.layout(job) {
                return out;
            }
            self.by_key.remove(&key);
        }
        match PersistentWorker::start().and_then(|mut w| {
            let out = w.layout(job)?;
            Ok((w, out))
        }) {
            Ok((w, out)) => {
                self.by_key.insert(key, w);
                out
            }
            Err(_) => {
                let mut out = execute(job);
                out.detail = "worker failed; in-process fallback".into();
                out.sandboxed = false;
                out
            }
        }
    }

    pub fn drop_all(&mut self) {
        for (_, mut w) in self.by_key.drain() {
            w.kill();
        }
    }

    pub fn retain_keys(&mut self, keep: &[IsolationKey]) {
        self.by_key.retain(|k, w| {
            if keep.iter().any(|live| live == k) {
                true
            } else {
                w.kill();
                false
            }
        });
    }
}

struct PersistentWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl PersistentWorker {
    fn start() -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if exe.file_name().and_then(|s| s.to_str()) != Some("frihart") {
            return Err("not the frihart binary".into());
        }
        let mut child = Command::new(exe)
            .arg(WORKER_FLAG)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().ok_or("stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("stdout")?);
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    fn layout(&mut self, job: &LayoutJob) -> Result<LayoutOut, String> {
        if let Ok(Some(_)) = self.child.try_wait() {
            return Err("dead".into());
        }
        let line = serde_json::to_string(job).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{line}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut reply = String::new();
        self.stdout
            .read_line(&mut reply)
            .map_err(|e| e.to_string())?;
        if reply.is_empty() {
            return Err("eof".into());
        }
        serde_json::from_str(&reply).map_err(|e| e.to_string())
    }

    fn kill(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
fn layout_isolated(job: &LayoutJob) -> LayoutOut {
    let mut pool = WorkerPool::default();
    pool.layout(
        IsolationKey::new("about", "test", frihart_core::ContainerId::PERSONAL),
        job,
    )
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
