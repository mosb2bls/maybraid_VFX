pub mod animations;
pub mod quantize;
pub mod rigs;

pub use character_rigs::authoring::ArmatureOffset;
pub use quantize::{
	finite_parameter_bits, interval_seconds, normalize_signed_zero, ClipTimePolicy, SampleAddress,
	DEFAULT_SAMPLE_INTERVAL_US,
};

/// Armature offset returned by [`Animation::effects_for`].
///
/// Identity is the absent effect. Translation, rotation, and scale all blend.
pub type Effects = ArmatureOffset;

/// Normalized animation sampling coordinate.
///
/// Cyclic animations wrap via [`Self::cycle`]; one-shot animations clamp via [`Self::clamp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress(pub f32);

impl Progress {
	pub fn new(value: f32) -> Self {
		Self(value)
	}

	/// Cyclic progress in `[0.0, 1.0)`.
	pub fn cycle(self) -> f32 {
		self.0.rem_euclid(1.0)
	}

	/// Clamped one-shot progress in `[0.0, 1.0]`.
	pub fn clamp(self) -> f32 {
		self.0.clamp(0.0, 1.0)
	}

	/// Whole completed cycles.
	pub fn cycles(self) -> f32 {
		self.0.floor()
	}

	/// True once a one-shot animation has reached or passed its end.
	pub fn is_complete(self) -> bool {
		self.0 >= 1.0
	}
}

/// Applies an animation to a rig at a specific progress value.
///
/// For cyclic animations, progress wraps by taking `progress.fract()`. Values above `1.0`
/// are valid (e.g. `1.5` samples halfway through the next cycle). One-shot animations
/// clamp progress to `[0.0, 1.0]`.
///
/// Split so a mailbox can write bones and root-motion independently (LOD markers):
///
/// - [`Self::apply_for`] mutates bone pose only.
/// - [`Self::effects_for`] is read-only (rest lengths + time) and returns armature
///   [`Effects`].
/// - [`Self::apply`] is the compatibility wrapper: `apply_for` then `effects_for`.
///
/// Composites ([`crate::animations::Mix`], [`crate::animations::Transition`]) must
/// call the split on children — do not go through [`Self::apply`] and discard half.
pub trait Animation<Rig> {
	/// Write bone pose. Do not apply armature root-motion here.
	fn apply_for(&self, rig: &mut Rig, progress: f32);

	/// Armature side effects from rest lengths and `progress`. Must not write bones.
	fn effects_for(&self, _rig: &Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}

	/// Compatibility: [`Self::apply_for`] then [`Self::effects_for`].
	fn apply(&self, rig: &mut Rig, progress: f32) -> Effects {
		self.apply_for(rig, progress);
		self.effects_for(rig, progress)
	}
}

/// Animation playback state owned by the controller.
#[derive(Debug, Clone)]
pub struct Playing<A> {
	pub animation: A,
	pub progress: f32,
	pub speed: f32,
}

impl<A> Playing<A> {
	pub fn new(animation: A, speed: f32) -> Self {
		Self { animation, progress: 0.0, speed }
	}

	pub fn advance(&mut self, delta_seconds: f32) {
		self.progress += delta_seconds * self.speed;
	}

	pub fn apply<R>(&self, rig: &mut R) -> Effects
	where
		A: Animation<R>,
	{
		self.animation.apply(rig, self.progress)
	}
}
