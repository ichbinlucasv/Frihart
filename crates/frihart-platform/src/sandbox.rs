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
    pub rlimits: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SandboxReport {
    pub no_new_privs: bool,
    pub landlock: bool,
    pub seccomp: bool,
    pub rlimits: bool,
    pub detail: String,
}

impl SandboxSpec {
    pub fn content_default() -> Self {
        Self {
            enabled: true,
            seccomp: true,
            landlock: true,
            no_new_privs: true,
            rlimits: true,
        }
    }

    /// Apply restrictions to *this* process. Call from a content child
    /// (`frihart --content-worker` after start), never from chrome.
    pub fn apply(&self) -> Result<SandboxReport> {
        if !self.enabled {
            return Ok(SandboxReport {
                detail: "disabled".into(),
                ..SandboxReport::default()
            });
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (self.seccomp, self.landlock, self.no_new_privs, self.rlimits);
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

/// Syscalls the content filter returns EPERM for. Names, not numbers.
pub fn seccomp_denies() -> &'static [&'static str] {
    &[
        "socket", "connect", "accept", "bind", "listen", "clone", "fork", "exec", "ptrace", "mount",
    ]
}

/// Resource caps the content worker applies to itself.
pub fn rlimit_names() -> &'static [&'static str] {
    &["as=256M", "nofile=128", "nproc=0", "core=0"]
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
            }
            Err(e) => append_detail(&mut report.detail, &format!("landlock: {e}")),
        }
    }
    if spec.rlimits {
        match restrict_rlimits() {
            Ok(()) => report.rlimits = true,
            Err(e) => append_detail(&mut report.detail, &format!("rlimit: {e}")),
        }
    }
    if spec.seccomp {
        match restrict_seccomp() {
            Ok(()) => {
                report.seccomp = true;
            }
            Err(e) => append_detail(&mut report.detail, &format!("seccomp: {e}")),
        }
    }
    if report.detail.is_empty() {
        let mut parts = Vec::new();
        if report.no_new_privs {
            parts.push("nnp");
        }
        if report.landlock {
            parts.push("landlock");
        }
        if report.seccomp {
            parts.push("seccomp");
        }
        if report.rlimits {
            parts.push("rlimit");
        }
        report.detail = if parts.is_empty() {
            "none".into()
        } else {
            parts.join("+")
        };
    }
    Ok(report)
}

fn append_detail(detail: &mut String, extra: &str) {
    if detail.is_empty() {
        *detail = extra.into();
    } else {
        detail.push_str("; ");
        detail.push_str(extra);
    }
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

#[cfg(target_os = "linux")]
const RLIMIT_AS_BYTES: u64 = 256 * 1024 * 1024;
#[cfg(target_os = "linux")]
const RLIMIT_NOFILE_N: u64 = 128;

#[cfg(target_os = "linux")]
fn restrict_rlimits() -> std::io::Result<()> {
    set_one(libc::RLIMIT_AS, RLIMIT_AS_BYTES)?;
    set_one(libc::RLIMIT_NOFILE, RLIMIT_NOFILE_N)?;
    set_one(libc::RLIMIT_NPROC, 0)?;
    set_one(libc::RLIMIT_CORE, 0)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_one(resource: libc::__rlimit_resource_t, soft: libc::rlim_t) -> std::io::Result<()> {
    let lim = libc::rlimit {
        rlim_cur: soft,
        rlim_max: soft,
    };
    // SAFETY: lim is a valid rlimit for this resource.
    let rc = unsafe { libc::setrlimit(resource, &lim) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// Classic BPF / seccomp-bpf constants (linux/filter.h, linux/seccomp.h).
#[cfg(target_os = "linux")]
const BPF_LD_W_ABS: u16 = 0x20;
#[cfg(target_os = "linux")]
const BPF_JMP_JEQ_K: u16 = 0x15;
#[cfg(target_os = "linux")]
const BPF_RET_K: u16 = 0x06;
#[cfg(target_os = "linux")]
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
#[cfg(target_os = "linux")]
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
#[cfg(target_os = "linux")]
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
#[cfg(target_os = "linux")]
const SECCOMP_DATA_NR: u32 = 0;
#[cfg(target_os = "linux")]
const SECCOMP_DATA_ARCH: u32 = 4;

#[cfg(target_os = "linux")]
fn native_audit_arch() -> Option<u32> {
    #[cfg(target_arch = "x86_64")]
    {
        Some(0xc000_003e)
    }
    #[cfg(target_arch = "aarch64")]
    {
        Some(0xc000_00b7)
    }
    #[cfg(target_arch = "riscv64")]
    {
        Some(0xc000_00f3)
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn denied_syscall_nrs() -> Vec<u32> {
    let mut nrs = vec![
        libc::SYS_socket as u32,
        libc::SYS_connect as u32,
        libc::SYS_accept as u32,
        libc::SYS_bind as u32,
        libc::SYS_listen as u32,
        libc::SYS_clone as u32,
        libc::SYS_execve as u32,
        libc::SYS_ptrace as u32,
        libc::SYS_mount as u32,
        libc::SYS_umount2 as u32,
        libc::SYS_unshare as u32,
        libc::SYS_setns as u32,
        libc::SYS_reboot as u32,
        libc::SYS_swapon as u32,
        libc::SYS_swapoff as u32,
        libc::SYS_syslog as u32,
        libc::SYS_init_module as u32,
        libc::SYS_delete_module as u32,
        libc::SYS_pivot_root as u32,
        libc::SYS_process_vm_readv as u32,
        libc::SYS_process_vm_writev as u32,
        libc::SYS_accept4 as u32,
        libc::SYS_socketpair as u32,
        libc::SYS_execveat as u32,
        libc::SYS_bpf as u32,
        libc::SYS_userfaultfd as u32,
        libc::SYS_perf_event_open as u32,
        libc::SYS_kexec_load as u32,
        libc::SYS_finit_module as u32,
        435, // clone3 (linux 5.3+)
    ];
    #[cfg(not(target_arch = "aarch64"))]
    {
        nrs.push(libc::SYS_fork as u32);
        nrs.push(libc::SYS_vfork as u32);
    }
    #[cfg(target_arch = "x86")]
    {
        nrs.push(libc::SYS_socketcall as u32);
    }
    nrs.sort_unstable();
    nrs.dedup();
    nrs
}

#[cfg(target_os = "linux")]
fn restrict_seccomp() -> std::io::Result<()> {
    let Some(arch) = native_audit_arch() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "seccomp arch",
        ));
    };
    let denied = denied_syscall_nrs();
    let mut filter: Vec<libc::sock_filter> = Vec::with_capacity(4 + denied.len() * 2);
    // Load arch. Wrong ABI → kill (x32 / compat would skip the deny list).
    filter.push(libc::sock_filter {
        code: BPF_LD_W_ABS,
        jt: 0,
        jf: 0,
        k: SECCOMP_DATA_ARCH,
    });
    filter.push(libc::sock_filter {
        code: BPF_JMP_JEQ_K,
        jt: 1,
        jf: 0,
        k: arch,
    });
    filter.push(libc::sock_filter {
        code: BPF_RET_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_KILL_PROCESS,
    });
    filter.push(libc::sock_filter {
        code: BPF_LD_W_ABS,
        jt: 0,
        jf: 0,
        k: SECCOMP_DATA_NR,
    });
    let eperm = SECCOMP_RET_ERRNO | (libc::EPERM as u32);
    for nr in denied {
        filter.push(libc::sock_filter {
            code: BPF_JMP_JEQ_K,
            jt: 0,
            jf: 1,
            k: nr,
        });
        filter.push(libc::sock_filter {
            code: BPF_RET_K,
            jt: 0,
            jf: 0,
            k: eperm,
        });
    }
    filter.push(libc::sock_filter {
        code: BPF_RET_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_ALLOW,
    });
    let prog = libc::sock_fprog {
        len: u16::try_from(filter.len())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "seccomp filter"))?,
        filter: filter.as_mut_ptr(),
    };
    // SAFETY: prog points at a live sock_filter array for the duration
    // of the syscall. no_new_privs must already be set.
    let rc = unsafe { libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &prog) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_default_is_on() {
        let s = SandboxSpec::content_default();
        assert!(s.enabled && s.landlock && s.no_new_privs && s.seccomp && s.rlimits);
        assert!(seccomp_denies().contains(&"socket"));
        assert!(seccomp_denies().contains(&"exec"));
        assert!(rlimit_names().iter().any(|n| n.starts_with("as=")));
    }

    #[test]
    fn apply_disabled_is_noop() {
        let r = SandboxSpec::default().apply().unwrap();
        assert!(!r.landlock);
        assert!(!r.no_new_privs);
        assert!(!r.seccomp);
        assert!(!r.rlimits);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn child_survives_fs_sandbox() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        // landlock + nnp only: seccomp denies execve, so /bin/true cannot
        // be the proof. The worker applies the full spec after start.
        let spec = SandboxSpec {
            enabled: true,
            seccomp: false,
            landlock: true,
            no_new_privs: true,
            rlimits: false,
        };
        let mut cmd = Command::new("/bin/true");
        let status = unsafe {
            cmd.pre_exec(move || {
                spec.apply()
                    .map(|_| ())
                    .map_err(|e| std::io::Error::other(e.to_string()))
            })
            .status()
        }
        .expect("spawn");
        assert!(status.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn seccomp_denies_socket() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let spec = SandboxSpec {
            enabled: true,
            seccomp: true,
            landlock: false,
            no_new_privs: true,
            rlimits: false,
        };
        let mut cmd = Command::new("/bin/true");
        let status = unsafe {
            cmd.pre_exec(move || {
                spec.apply()
                    .map(|_| ())
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
                if fd < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EPERM)
                        || err.raw_os_error() == Some(libc::EACCES)
                    {
                        libc::_exit(0);
                    }
                } else {
                    libc::close(fd);
                }
                libc::_exit(2);
            })
            .status()
        }
        .expect("spawn");
        assert!(status.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rlimits_cap_nofile() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let spec = SandboxSpec {
            enabled: true,
            seccomp: false,
            landlock: false,
            no_new_privs: true,
            rlimits: true,
        };
        let mut cmd = Command::new("/bin/true");
        let status = unsafe {
            cmd.pre_exec(move || {
                spec.apply()
                    .map(|_| ())
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let mut lim = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) == 0
                    && lim.rlim_cur == RLIMIT_NOFILE_N
                {
                    libc::_exit(0);
                }
                libc::_exit(2);
            })
            .status()
        }
        .expect("spawn");
        assert!(status.success());
    }
}
