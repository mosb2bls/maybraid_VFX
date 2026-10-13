//! I-Apartment storey.
//!
//! Fit an [`crate::IFloor`] I / T / U / L / Z envelope to confines from seed, then
//! expose the shell’s natural **1–3 primary rectangles** (stem + optional flanges),
//! cut exterior apertures, and remap inbound shafts onto 9-pocket centroids.
//!
//! Pipeline:
//! - [`IApartmentParameterized::sample`] — I-frame layout + window catalog knobs
//! - [`IApartmentFloorPlan::from_parameterized`] — shell + openings + primary rects
//! - [`IApartmentFullStorey::from_floor_plan`] — [`crate::LivableApartments`] per primary rect
//!   (HallsToShafts → group → [`crate::LivableApartment`])

pub mod floor_plan;
pub mod full_storey;
pub mod parameterized;

pub use floor_plan::IApartmentFloorPlan;
pub use full_storey::IApartmentFullStorey;
pub use parameterized::IApartmentParameterized;

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "i_apartment";
