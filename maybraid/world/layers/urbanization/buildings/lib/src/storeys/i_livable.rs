//! Generic livable I-frame aliases.
//!
//! The original I-frame implementation was introduced for apartment storeys,
//! but its shell + progressive [`crate::LivableApartments`] fill is also the
//! reusable storey primitive for small houses.

pub use super::i_apartment::parameterized::MIN_POCKET_SHAFT_SIDE;
pub use super::i_apartment::{
	IApartmentFloorPlan as ILivableFloorPlan, IApartmentFullStorey as ILivableStorey,
	IApartmentParameterized as ILivableParameterized,
};
