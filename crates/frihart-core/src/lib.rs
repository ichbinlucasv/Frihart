//! Shared types, errors, and URL helpers.
//!
//! Every crate may depend on `frihart-core`. This crate depends on
//! almost nothing else.

#![forbid(unsafe_code)]

mod error;
mod ids;
mod license;
mod opsec;
mod urls;

pub use error::{FrihartError, Result};
pub use ids::{ContainerId, DocumentId, IsolationKey, TabId, WindowId};
pub use license::{licensed_locally, linux_is_free, list_price_eur, price_label};
pub use opsec::{
    ensure_private_dir, safe_host, sanitize_error, shred_file, shred_tree, write_private,
    write_private_str,
};
pub use urls::{
    UrlKind, about_page, classify_url, display_url, is_script_scheme, looks_like_destination,
    parse_user_input, try_parse_user_input,
};

/// Product name shown in chrome and logs.
pub const APP_NAME: &str = "Frihart";

/// Reverse-DNS identifier for the desktop file and sandbox.
pub const APP_ID: &str = "org.frihart.Frihart";

/// Crate version, injected by Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Frozen user-agent. No OS micro-version, no engine salad.
///
/// Sites that require a Chrome impersonation will fail until the user
/// sets `network.user_agent` themselves. That is a feature.
pub const FROZEN_USER_AGENT: &str = "Mozilla/5.0 (compatible; Frihart/0.1)";

/// Default profile name on disk.
pub const DEFAULT_PROFILE_NAME: &str = "default";
