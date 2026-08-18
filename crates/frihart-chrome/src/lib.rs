//! Linux browser chrome: windows, tabs, toolbar, viewport.
//!
//! This crate owns pixels and input. It does not parse HTML and it does
//! not speak TLS.

#![forbid(unsafe_code)]

mod app;
mod paint;
mod raster;
mod state;
mod text;
mod theme;
mod worker;

pub use app::run;
