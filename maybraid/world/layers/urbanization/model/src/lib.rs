//! Urbanization as a layer over ground model `U`.
//!
//! [`Urbanization<U>`] is `U`'s ground with pads composed in.

mod model;
mod pads;
mod region;
mod setting;

pub use model::Urbanization;
pub use pads::PadOps;
pub use region::{urbanization_host_region, urbanization_visual_region};
pub use setting::UrbanSetting;
