//! Frihart process entry. Phase 6 will reuse this binary as a supervisor.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use frihart_core::{APP_NAME, VERSION};
use frihart_profile::Profile;

#[derive(Debug, Parser)]
#[command(
    name = "frihart",
    version = VERSION,
    about = "A sovereign, privacy-first web browser"
)]
struct Args {
    /// URL or about: page to open. Defaults to the homepage preference.
    url: Option<String>,

    /// Profile directory. Defaults to $XDG_DATA_HOME/frihart/profiles/default.
    #[arg(long)]
    profile: Option<PathBuf>,

    /// Private window: memory only, no history, no cookie writes.
    #[arg(long)]
    private: bool,
}

fn main() -> ExitCode {
    if let Err(err) = try_main() {
        eprintln!("{APP_NAME}: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn try_main() -> frihart_core::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("frihart=info")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    let args = Args::parse();

    let profile = if args.private {
        tracing::info!("private window; profile stays in memory");
        Profile::ephemeral()?
    } else if let Some(path) = args.profile {
        tracing::info!(path = %path.display(), "opening profile");
        Profile::open_dir(path)?
    } else {
        Profile::open_default()?
    };

    tracing::info!(
        version = VERSION,
        profile = profile.name(),
        "starting Frihart"
    );

    frihart_chrome::run(profile, args.url)
}
