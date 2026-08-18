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
    #[arg(long, value_name = "PATH")]
    install_addon: Option<PathBuf>,
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

    let profile = if frihart_platform::should_open_ephemeral(args.private, args.profile.is_some()) {
        Profile::ephemeral()?
    } else if let Some(path) = args.profile {
        Profile::open_dir(path)?
    } else {
        Profile::open_default()?
    };

    frihart_chrome::run(profile, args.url, args.tor)
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
        out.sandboxed = report.no_new_privs || report.landlock || report.seccomp;
        out.detail = report.detail.clone();
        let bytes = serde_json::to_string(&out)
            .map_err(|e| frihart_core::FrihartError::Message(e.to_string()))?;
        writeln!(stdout, "{bytes}")?;
        stdout.flush()?;
    }
    Ok(())
}
