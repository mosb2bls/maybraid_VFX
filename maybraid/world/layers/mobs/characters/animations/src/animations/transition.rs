use character_rigs::authoring::{ArmatureOffset, PoseBuffer};

/// Remaps linear transition progress into blend weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionCurve {
	Linear,
	#[default]
	SmoothStep,
	EaseIn,
	EaseOut,
	EaseInOut,
}

impl TransitionCurve {
	pub fn sample(self, t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);

		match self {
			Self::Linear => t,
			Self::SmoothStep => t * t * (3.0 - 2.0 * t),
			Self::EaseIn => t * t,
			Self::EaseOut => {
				let inv = 1.0 - t;
				1.0 - inv * inv
			}
			Self::EaseInOut => {
				if t < 0.5 {
					2.0 * t * t
				} else {
					1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
				}
			}
		}
	}
}

/// Alias kept for earlier docs referring to [`BlendCurve`].
pub type BlendCurve = TransitionCurve;

/// Transition into an animation from a captured source pose.
///
/// Unlike [`Mix`](super::Mix) or [`Smooth`](super::Smooth), the source pose is fixed at
/// construction time rather than re-sampled each frame. Call [`Transition::apply`] with
/// separate animation and transition progress values.
#[derive(Debug, Clone)]
pub struct Transition<A> {
	/// Animation being transitioned into.
	pub animation: A,
	/// Pose captured when the transition began.
	pub from_pose: PoseBuffer,
	/// Armature offset visible when the transition began. Identity fades in from rest.
	pub from_offset: ArmatureOffset,
	/// Curve used to remap transition progress into blend weight.
	pub curve: TransitionCurve,
}

impl<A> Transition<A> {
	/// Creates a transition from an explicit captured pose. Offset starts at identity.
	pub fn from_pose(animation: A, from_pose: PoseBuffer) -> Self {
		Self::from_visible(animation, from_pose, ArmatureOffset::IDENTITY)
	}

	/// Latest-wins capture: blend from the last visible pose and armature offset.
	pub fn from_visible(animation: A, from_pose: PoseBuffer, from_offset: ArmatureOffset) -> Self {
		Self { animation, from_pose, from_offset, curve: TransitionCurve::default() }
	}

	pub fn with_curve(mut self, curve: TransitionCurve) -> Self {
		self.curve = curve;
		self
	}

	pub fn weight(&self, transition_progress: f32) -> f32 {
		self.curve.sample(transition_progress.clamp(0.0, 1.0))
	}

	pub fn is_complete(&self, transition_progress: f32) -> bool {
		transition_progress >= 1.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn transition_curve_endpoints() -> anyhow::Result<()> {
		for curve in [
			TransitionCurve::Linear,
			TransitionCurve::SmoothStep,
			TransitionCurve::EaseIn,
			TransitionCurve::EaseOut,
			TransitionCurve::EaseInOut,
		] {
			assert!(curve.sample(0.0).abs() < 1e-5);
			assert!((curve.sample(1.0) - 1.0).abs() < 1e-5);
		}
		Ok(())
	}
}
