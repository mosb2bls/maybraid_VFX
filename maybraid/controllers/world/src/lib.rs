//! Compatibility re-export of [`maybraid_session`].
//!
//! Session capabilities live in `gameplay/session`. This crate keeps the
//! existing package name.

pub use maybraid_session::SessionPlugin as WorldControllerPlugin;
pub use maybraid_session::*;
