use crate::Effects;

/// Animations that hold the same pose at every progress sample.
///
/// Implement [`Animation`] by delegating to [`Self::apply_fixed`]. Use with
/// [`Transition`](super::Transition) to blend linearly into or out of a known target pose.
pub trait FixedPosition<Rig> {
	/// Apply the held pose; progress is intentionally ignored.
	fn apply_fixed(&self, rig: &mut Rig) -> Effects;
}
