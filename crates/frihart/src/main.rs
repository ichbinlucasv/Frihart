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

    let profile = if args.private {
        Profile::ephemeral()?
    } else if let Some(path) = args.profile {
        Profile::open_dir(path)?
    } else {
        Profile::open_default()?
    };

    frihart_chrome::run(profile, args.url, args.tor)
}
