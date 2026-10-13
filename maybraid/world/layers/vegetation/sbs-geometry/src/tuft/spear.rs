//! **Spear tuft** — thin flat grass-like spears (authoring shape; VC approximates as blades).

/// CLI / noise-driven shape parameters for a spear tuft.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SpearTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12))]
	pub spear_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.9))]
	pub spear_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.008))]
	pub base_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.022))]
	pub belly_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.25))]
	pub max_tilt_radians: f32,
	/// Along-strand segment count (`1` = one straight section base→tip; higher = more kinks).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 2))]
	pub bend_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub noise_amplitude: f32,
	/// Sway cycles **per bend segment**; near `1.0` each segment kinks independently, lower
	/// keeps neighbouring segments correlated (smoother bow).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for SpearTuftShape {
	fn default() -> Self {
		Self {
			spear_count: 12,
			spear_length: 0.9,
			base_half_width: 0.008,
			belly_half_width: 0.022,
			max_tilt_radians: 0.25,
			bend_segments: 2,
			noise_amplitude: 0.08,
			noise_frequency: 1.0,
			seed: 0,
		}
	}
}
