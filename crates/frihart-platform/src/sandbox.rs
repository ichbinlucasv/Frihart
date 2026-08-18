//! Content-process sandbox. Do not apply this in the chrome process:
//! landlock would lock the profile out.

#![allow(unsafe_code)]

use frihart_core::Result;

#[derive(Clone, Debug, Default)]
pub struct SandboxSpec {
    pub enabled: bool,
    pub seccomp: bool,
    pub landlock: bool,
    pub no_new_privs: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SandboxReport {
    pub no_new_privs: bool,
    pub landlock: bool,
    pub detail: String,
}

impl SandboxSpec {
    pub fn content_default() -> Self {
        Self {
            enabled: true,
            seccomp: true,
            landlock: true,
            no_new_privs: true,
        }
    }

    /// Apply restrictions to *this* process. Call from a content child
    /// (`pre_exec`), never from chrome.
    pub fn apply(&self) -> Result<SandboxReport> {
        if !self.enabled {
            return Ok(SandboxReport {
                detail: "disabled".into(),
                ..SandboxReport::default()
            });
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (self.seccomp, self.landlock, self.no_new_privs);
            return Ok(SandboxReport {
                detail: "not linux".into(),
                ..SandboxReport::default()
            });
        }
        #[cfg(target_os = "linux")]
        {
            linux_apply(self)
        }
    }
}

/// Kernel Landlock ABI, if present.
pub fn landlock_abi() -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        linux_landlock_abi()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_apply(spec: &SandboxSpec) -> Result<SandboxReport> {
    let mut report = SandboxReport::default();
    if spec.no_new_privs {
        match set_no_new_privs() {
            Ok(()) => report.no_new_privs = true,
            Err(e) => report.detail = format!("no_new_privs: {e}"),
        }
    }
    if spec.landlock {
        match restrict_landlock() {
            Ok(()) => {
                report.landlock = true;
                if report.detail.is_empty() {
                    report.detail = "ok".into();
                }
            }
            Err(e) => {
                if report.detail.is_empty() {
                    report.detail = format!("landlock: {e}");
                } else {
                    report.detail = format!("{}; landlock: {e}", report.detail);
                }
            }
        }
    }
    let _ = spec.seccomp;
    Ok(report)
}

#[cfg(target_os = "linux")]
fn set_no_new_privs() -> std::io::Result<()> {
    // SAFETY: PR_SET_NO_NEW_PRIVS with arg 1 is a documented no-op on
    // credentials other than "cannot gain privileges later."
    let rc = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1 << 0;
#[cfg(target_os = "linux")]
const LANDLOCK_ACCESS_FS_EXECUTE: u64 = 1 << 0;
#[cfg(target_os = "linux")]
const LANDLOCK_ACCESS_FS_READ_FILE: u64 = 1 << 2;
#[cfg(target_os = "linux")]
const LANDLOCK_ACCESS_FS_READ_DIR: u64 = 1 << 3;
#[cfg(target_os = "linux")]
const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

#[cfg(target_os = "linux")]
#[repr(C)]
struct LandlockRulesetAttr {
    handled_access_fs: u64,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct LandlockPathBeneath {
    allowed_access: u64,
    parent_fd: i32,
}

#[cfg(target_os = "linux")]
fn linux_landlock_abi() -> Option<u32> {
    // SAFETY: version probe; null attr + VERSION flag is the documented query.
    let rc = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<LandlockRulesetAttr>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    if rc < 0 { None } else { Some(rc as u32) }
}

#[cfg(target_os = "linux")]
fn restrict_landlock() -> std::io::Result<()> {
    let handled =
        LANDLOCK_ACCESS_FS_EXECUTE | LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_READ_DIR;
    let attr = LandlockRulesetAttr {
        handled_access_fs: handled,
    };
    // SAFETY: attr is a valid landlock_ruleset_attr of that size.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &attr,
            std::mem::size_of::<LandlockRulesetAttr>(),
            0u32,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let fd = fd as i32;
    let allowed = handled;
    for path in ["/usr", "/lib", "/lib64", "/bin", "/etc"] {
        if let Err(e) = add_path(fd, path, allowed) {
            let _ = e;
        }
    }
    // SAFETY: fd is a ruleset we created; flags 0.
    let rc = unsafe { libc::syscall(libc::SYS_landlock_restrict_self, fd, 0u32) };
    // SAFETY: close the ruleset fd.
    unsafe {
        libc::close(fd);
    }
    if rc < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn add_path(ruleset: i32, path: &str, access: u64) -> std::io::Result<()> {
    let c = std::ffi::CString::new(path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path"))?;
    // SAFETY: path is a C string; O_PATH|O_CLOEXEC is valid.
    let pfd = unsafe { libc::open(c.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
    if pfd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let beneath = LandlockPathBeneath {
        allowed_access: access,
        parent_fd: pfd,
    };
    // SAFETY: ruleset + path_beneath attr as documented.
    let rc = unsafe {
        libc::syscall(
            libc::SYS_landlock_add_rule,
            ruleset,
            LANDLOCK_RULE_PATH_BENEATH,
            &beneath,
            0u32,
        )
    };
    // SAFETY: close the O_PATH fd.
    unsafe {
        libc::close(pfd);
    }
    if rc < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_default_is_on() {
        let s = SandboxSpec::content_default();
        assert!(s.enabled && s.landlock && s.no_new_privs);
    }

    #[test]
    fn apply_disabled_is_noop() {
        let r = SandboxSpec::default().apply().unwrap();
        assert!(!r.landlock);
        assert!(!r.no_new_privs);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn child_survives_content_sandbox() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let mut cmd = Command::new("/bin/true");
        let status = unsafe {
            cmd.pre_exec(|| {
                SandboxSpec::content_default()
                    .apply()
                    .map(|_| ())
                    .map_err(|e| std::io::Error::other(e.to_string()))
            })
            .status()
        }
        .expect("spawn");
        assert!(status.success());
    }
}
