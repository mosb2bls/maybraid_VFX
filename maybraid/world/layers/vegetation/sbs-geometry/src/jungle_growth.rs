//! Jungle-growth cluster shape ([RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).

/// Per-instance configuration for a jungle-growth cluster at one anchor.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct JungleGrowthShape {
	/// Inner noisy-ball radius multiplier relative to spawn transform uniform scale (node radius).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.72))]
	pub inner_ball_scale: f32,
	/// Frond crown scale relative to the assembly root (anchor spawn uniform scale).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub foliage_scale: f32,
	/// Buddha's-hand tuft scale relative to [`Self::foliage_scale`].
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.8))]
	pub buddha_hand_scale: f32,
	/// Deterministic seed for body and foliage noise at this anchor.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for JungleGrowthShape {
	fn default() -> Self {
		Self { inner_ball_scale: 0.72, foliage_scale: 1.0, buddha_hand_scale: 0.8, seed: 0 }
	}
}

impl JungleGrowthShape {
	pub fn ball_radius(&self, node_radius: f32) -> f32 {
		node_radius * self.inner_ball_scale
	}
}
