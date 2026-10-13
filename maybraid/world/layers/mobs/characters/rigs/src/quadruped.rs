/// Rest-pose upper and lower leg lengths used for analytic drop.
///
/// Sampling derives these from the effective rest. A translation whose length is
/// near zero keeps the default 0.5 m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSegmentLengths {
	pub upper: f32,
	pub lower: f32,
}

impl Default for LegSegmentLengths {
	fn default() -> Self {
		Self { upper: 0.5, lower: 0.5 }
	}
}
