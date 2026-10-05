#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use frihart_core::{APP_NAME, VERSION};
use frihart_profile::Profile;

#[derive(Debug, Parser)]
#[command(name = "frihart", version = VERSION, about = "Frihart")]
struct Args {
    url: Option<String>,
    #[arg(long)]
    profile: Option<PathBuf>,
    #[arg(long)]
    private: bool,
    #[arg(long)]
    tor: bool,
    #[arg(long)]
    i2p: bool,
    #[arg(long, value_name = "PATH")]
    install_addon: Option<PathBuf>,
    /// Shred the profile and exit, without opening a window.
    #[arg(long)]
    wipe: bool,
    /// Shred the profile if it is not opened for DAYS days (1-365), or `off`.
    #[arg(long, value_name = "DAYS|off")]
    deadman: Option<String>,
    /// Hidden: chrome spawns this to layout HTML under the content sandbox.
    #[arg(long, hide = true)]
    content_worker: bool,
}

fn main() -> ExitCode {
    if try_main().is_err() {
        eprintln!("{APP_NAME}: err");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn try_main() -> frihart_core::Result<()> {
    let args = Args::parse();

    if args.content_worker {
        return run_content_worker();
    }

    if let Some(addon) = &args.install_addon {
        let mut profile = if let Some(path) = &args.profile {
            Profile::open_dir(path)?
        } else {
            Profile::open_default()?
        };
        let installed = profile.install_addon(addon)?;
        println!("{}", installed.id);
        return Ok(());
    }

    if args.wipe {
        let mut profile = if let Some(path) = &args.profile {
            Profile::open_dir(path)?
        } else {
            Profile::open_default()?
        };
        return profile.shred();
    }

    if let Some(value) = &args.deadman {
        let profile = if let Some(path) = &args.profile {
            Profile::open_dir(path)?
        } else {
            Profile::open_default()?
        };
        return set_deadman(&profile, value);
    }

    let profile = if frihart_platform::should_open_ephemeral(args.private, args.profile.is_some()) {
        Profile::ephemeral()?
    } else if let Some(path) = args.profile {
        Profile::open_dir(path)?
    } else {
        Profile::open_default()?
    };

    if args.tor && args.i2p {
        return Err(frihart_core::FrihartError::Message(
            "pick one circuit: --tor or --i2p".into(),
        ));
    }

    frihart_chrome::run(profile, args.url, args.tor, args.i2p)
}

fn set_deadman(profile: &Profile, value: &str) -> frihart_core::Result<()> {
    if value == "off" {
        frihart_profile::clear_deadman(profile.root())?;
        println!("dead-man switch off");
        return Ok(());
    }
    let days: u32 = value.parse().map_err(|_| {
        frihart_core::FrihartError::Message("--deadman takes a number of days or `off`".into())
    })?;
    frihart_profile::set_deadman(profile.root(), days, frihart_profile::unix_now())?;
    println!("dead-man switch on: shred after {days} days without a start");
    Ok(())
}

fn run_content_worker() -> frihart_core::Result<()> {
    use std::io::{BufRead, Write};

    let report = frihart_platform::SandboxSpec::content_default().apply()?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let lines = stdin.lock().lines();
    for line in lines {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "quit" {
            break;
        }
        let job: frihart_pipeline::LayoutJob = serde_json::from_str(line)
            .map_err(|e| frihart_core::FrihartError::Message(e.to_string()))?;
        let mut out = frihart_pipeline::execute(&job);
        out.sandboxed = report.no_new_privs || report.landlock || report.seccomp || report.rlimits;
        out.detail = report.detail.clone();
        let bytes = serde_json::to_string(&out)
            .map_err(|e| frihart_core::FrihartError::Message(e.to_string()))?;
        writeln!(stdout, "{bytes}")?;
        stdout.flush()?;
    }
    Ok(())
}
