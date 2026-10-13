//! Braidman preset notes.
//!
//! The first implementation reuses the shared [`crate::GenderPreset`] and
//! [`crate::BuildPreset`] enums directly. Braidman-specific preset behavior lives
//! in `pose`, where those preset IDs become small bone-scale refinement layers.
//!
//! When the full resolution pipeline lands, this module should own:
//! - which shared preset IDs Braidman exposes in the creator
//! - per-preset slider percent offsets (see spec Braidman gender/build tables)
//! - any Braidman-only default feature or asset choices
//!
//! `pose` would then consume resolved slider values instead of duplicating preset
//! effects as parallel bone-scale match arms.

// Module slot reserved for species-owned preset tables; lean pass keeps behavior
// in `pose` so the playground can preview silhouettes before Stage-1 marshaling.
