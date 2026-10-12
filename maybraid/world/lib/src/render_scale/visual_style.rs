//! Visual style presentation modes with artistic resolution targets and effects.

use bevy::prelude::*;

/// Named visual styles with distinct resolution and presentation characteristics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VisualStyle {
	#[default]
	Impressionist,
	Sharp,
	Retro,
	Antediluvian,
	PrePostModern,
}

impl VisualStyle {
	pub const ALL: [Self; 5] = [
		Self::Impressionist,
		Self::Sharp,
		Self::Retro,
		Self::Antediluvian,
		Self::PrePostModern,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::Impressionist => "Impressionist",
			Self::Sharp => "Sharp",
			Self::Retro => "Retro",
			Self::Antediluvian => "Antediluvian",
			Self::PrePostModern => "Pre-post-modern",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Impressionist => {
				"Painterly softening at ~700×394 (16:9). Inexpensive linear upscale with artistic blur. Default."
			}
			Self::Sharp => "Native physical resolution, untreated. Primarily useful for testing.",
			Self::Retro => "Nearest-neighbor upscale from ~540×304 (16:9). Performance mode.",
			Self::Antediluvian => {
				"Nearest-neighbor upscale from ~400×225 (16:9). Extreme performance mode."
			}
			Self::PrePostModern => {
				"Misregistered color-print effect at ~1280×720 (16:9). Slight color layer displacement and stable dithering."
			}
		}
	}

	/// Reference height at 16:9 aspect ratio for this style.
	pub fn reference_height(self) -> u32 {
		match self {
			Self::Impressionist => 394,
			Self::Sharp => 0,
			Self::Retro => 304,
			Self::Antediluvian => 225,
			Self::PrePostModern => 720,
		}
	}

	/// Parse a stable identifier from the agent scripting API.
	pub fn from_script_id(id: &str) -> Option<Self> {
		match id {
			"impressionist" => Some(Self::Impressionist),
			"sharp" => Some(Self::Sharp),
			"retro" => Some(Self::Retro),
			"antediluvian" => Some(Self::Antediluvian),
			"pre_post_modern" => Some(Self::PrePostModern),
			_ => None,
		}
	}

	/// Stable identifier for the agent scripting API.
	pub fn script_id(self) -> &'static str {
		match self {
			Self::Impressionist => "impressionist",
			Self::Sharp => "sharp",
			Self::Retro => "retro",
			Self::Antediluvian => "antediluvian",
			Self::PrePostModern => "pre_post_modern",
		}
	}

	/// Migrate a legacy InGameRenderScale value to a VisualStyle.
	pub fn from_legacy_render_scale(scale: LegacyRenderScale) -> Self {
		match scale {
			LegacyRenderScale::Auto | LegacyRenderScale::Half => Self::Impressionist,
			LegacyRenderScale::Native => Self::Sharp,
			LegacyRenderScale::ThreeQuarter => Self::PrePostModern,
		}
	}

	/// Plan resolution and presentation parameters for this style.
	pub fn plan(
		self,
		window_physical: UVec2,
		window_scale_factor: f32,
	) -> VisualStylePlan {
		if window_physical.x == 0 || window_physical.y == 0 {
			return VisualStylePlan {
				style: self,
				target_size: UVec2::ZERO,
				window_logical: Vec2::ZERO,
				image_scale_factor: 1.0,
				treatment: PresentationTreatment::None,
			};
		}

		let window_logical = window_logical_size(window_physical, window_scale_factor);
		let reference_height = self.reference_height();

		let (target_size, treatment) = if reference_height == 0 {
			(window_physical, PresentationTreatment::None)
		} else {
			let aspect = window_physical.x as f32 / window_physical.y as f32;
			let target_height = reference_height.min(window_physical.y);
			let target_width = ((target_height as f32) * aspect).round() as u32;
			let target_size = UVec2::new(target_width.max(1), target_height.max(1));

			let treatment = match self {
				Self::Impressionist => PresentationTreatment::Impressionist,
				Self::Retro | Self::Antediluvian => PresentationTreatment::NearestNeighbor,
				Self::PrePostModern => PresentationTreatment::PrePostModern,
				Self::Sharp => PresentationTreatment::None,
			};

			(target_size, treatment)
		};

		let image_scale_factor = if target_size.x > 0 && window_logical.x > 0.0 {
			target_size.x as f32 / window_logical.x
		} else {
			1.0
		};

		VisualStylePlan {
			style: self,
			target_size,
			window_logical,
			image_scale_factor,
			treatment,
		}
	}
}

/// Legacy render scale enum values for settings migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyRenderScale {
	Auto,
	Native,
	ThreeQuarter,
	Half,
}

/// Resolved sizes and presentation parameters for one window + visual style pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualStylePlan {
	pub style: VisualStyle,
	pub target_size: UVec2,
	pub window_logical: Vec2,
	pub image_scale_factor: f32,
	pub treatment: PresentationTreatment,
}

impl VisualStylePlan {
	/// Whether this plan requires an offscreen render target and presentation pass.
	pub fn requires_offscreen(&self) -> bool {
		self.treatment != PresentationTreatment::None
	}
}

/// Presentation treatment applied during upscale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresentationTreatment {
	None,
	NearestNeighbor,
	Impressionist,
	PrePostModern,
}

fn window_logical_size(window_physical: UVec2, window_scale_factor: f32) -> Vec2 {
	let factor = if window_scale_factor > 0.0 && window_scale_factor.is_finite() {
		window_scale_factor
	} else {
		1.0
	};
	Vec2::new(window_physical.x as f32 / factor, window_physical.y as f32 / factor)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_styles_have_nonempty_labels_and_descriptions() {
		for style in VisualStyle::ALL {
			assert!(!style.label().is_empty());
			assert!(!style.description().is_empty());
		}
	}

	#[test]
	fn script_ids_round_trip() {
		for style in VisualStyle::ALL {
			let id = style.script_id();
			assert_eq!(VisualStyle::from_script_id(id), Some(style));
		}
		assert_eq!(VisualStyle::from_script_id("unknown"), None);
	}

	#[test]
	fn reference_heights_match_spec() {
		assert_eq!(VisualStyle::Impressionist.reference_height(), 394);
		assert_eq!(VisualStyle::Sharp.reference_height(), 0);
		assert_eq!(VisualStyle::Retro.reference_height(), 304);
		assert_eq!(VisualStyle::Antediluvian.reference_height(), 225);
		assert_eq!(VisualStyle::PrePostModern.reference_height(), 720);
	}

	#[test]
	fn sharp_uses_native_resolution() {
		let plan = VisualStyle::Sharp.plan(UVec2::new(1920, 1080), 1.0);
		assert_eq!(plan.target_size, UVec2::new(1920, 1080));
		assert_eq!(plan.treatment, PresentationTreatment::None);
		assert!(!plan.requires_offscreen());
	}

	#[test]
	fn impressionist_scales_to_reference_height() {
		let plan = VisualStyle::Impressionist.plan(UVec2::new(1920, 1080), 1.0);
		assert_eq!(plan.target_size.y, 394);
		let expected_width = (394.0_f32 * 1920.0 / 1080.0).round() as u32;
		assert_eq!(plan.target_size.x, expected_width);
		assert_eq!(plan.treatment, PresentationTreatment::Impressionist);
		assert!(plan.requires_offscreen());
	}

	#[test]
	fn retro_and_antediluvian_use_nearest_neighbor() {
		let retro = VisualStyle::Retro.plan(UVec2::new(1920, 1080), 1.0);
		assert_eq!(retro.treatment, PresentationTreatment::NearestNeighbor);

		let antediluvian = VisualStyle::Antediluvian.plan(UVec2::new(1920, 1080), 1.0);
		assert_eq!(antediluvian.treatment, PresentationTreatment::NearestNeighbor);
		assert!(antediluvian.target_size.y < retro.target_size.y);
	}

	#[test]
	fn pre_post_modern_targets_720p() {
		let plan = VisualStyle::PrePostModern.plan(UVec2::new(1920, 1080), 1.0);
		assert_eq!(plan.target_size.y, 720);
		assert_eq!(plan.treatment, PresentationTreatment::PrePostModern);
	}

	#[test]
	fn target_clamped_to_native_when_viewport_is_smaller() {
		let plan = VisualStyle::Impressionist.plan(UVec2::new(640, 360), 1.0);
		assert_eq!(plan.target_size, UVec2::new(640, 360));
	}

	#[test]
	fn aspect_ratio_preserved() {
		let window = UVec2::new(2560, 1440);
		let aspect = window.x as f32 / window.y as f32;
		let plan = VisualStyle::Impressionist.plan(window, 1.0);
		let target_aspect = plan.target_size.x as f32 / plan.target_size.y as f32;
		assert!((target_aspect - aspect).abs() < 0.01);
	}

	#[test]
	fn legacy_migration_matches_spec() {
		assert_eq!(
			VisualStyle::from_legacy_render_scale(LegacyRenderScale::Auto),
			VisualStyle::Impressionist
		);
		assert_eq!(
			VisualStyle::from_legacy_render_scale(LegacyRenderScale::Native),
			VisualStyle::Sharp
		);
		assert_eq!(
			VisualStyle::from_legacy_render_scale(LegacyRenderScale::ThreeQuarter),
			VisualStyle::PrePostModern
		);
		assert_eq!(
			VisualStyle::from_legacy_render_scale(LegacyRenderScale::Half),
			VisualStyle::Impressionist
		);
	}
}
