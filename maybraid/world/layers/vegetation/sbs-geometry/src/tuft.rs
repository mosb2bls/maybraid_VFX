//! Tuft shape IR for Chico vegetation ([RFC-183 §3.1.2.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md)).

mod blade;
mod directions;
mod spear;
mod sway;

pub use blade::{BladeFrondSegment, BladeStrand, BladeTuftShape};
pub use spear::SpearTuftShape;
