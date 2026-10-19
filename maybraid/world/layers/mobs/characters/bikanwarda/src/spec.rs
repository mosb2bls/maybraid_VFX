//! Form presentation and shared assembly spec.

use bevy::prelude::*;
use characters::species::epiphant::EpiphantConfig;

use crate::measure::encounter_uniform_scale;
use crate::presentation::BikanwardaPresentation;

/// Encounter layout vs runtime summon vs enraged presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BikanwardaForm {
	#[default]
	Encounter,
	/// Smaller summon footprint (provisional — see PR open questions).
	Ward,
	Enraged,
}

impl BikanwardaForm {
	pub const VALUES: &'static [Self] = &[Self::Encounter, Self::Ward, Self::Enraged];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Encounter => "encounter",
			Self::Ward => "ward",
			Self::Enraged => "enraged",
		}
	}

	/// Multiplier on encounter uniform scale (Ward summon lane).
	pub fn summon_scale_factor(self) -> f32 {
		match self {
			Self::Ward => 0.85,
			Self::Encounter | Self::Enraged => 1.0,
		}
	}
}

/// Authored actor recipe shared by Encounter, Ward, and Enraged lanes.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct BikanwardaSpec {
	pub epiphant: EpiphantConfig,
	pub form: BikanwardaForm,
	pub presentation: BikanwardaPresentation,
}

impl Default for BikanwardaSpec {
	fn default() -> Self {
		Self::encounter()
	}
}

impl BikanwardaSpec {
	pub fn encounter() -> Self {
		Self {
			epiphant: EpiphantConfig::default_preview(),
			form: BikanwardaForm::Encounter,
			presentation: BikanwardaPresentation::encounter(),
		}
	}

	pub fn ward() -> Self {
		Self {
			form: BikanwardaForm::Ward,
			presentation: BikanwardaPresentation::ward(),
			..Self::encounter()
		}
	}

	pub fn enraged() -> Self {
		Self {
			form: BikanwardaForm::Enraged,
			presentation: BikanwardaPresentation::enraged(),
			..Self::encounter()
		}
	}

	/// Uniform assembly scale for this form (Epiphant preview hull → boss height).
	pub fn uniform_scale(&self) -> f32 {
		encounter_uniform_scale() * self.form.summon_scale_factor()
	}

	pub fn root_transform(&self, translation: Vec3, facing_y: f32) -> Transform {
		let scale = self.uniform_scale();
		Transform::from_translation(translation)
			.with_rotation(Quat::from_rotation_y(facing_y))
			.with_scale(Vec3::splat(scale))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_forms_share_the_same_epiphant_recipe_skeleton() {
		for form in BikanwardaForm::VALUES {
			let spec = BikanwardaSpec { form: *form, ..BikanwardaSpec::encounter() };
			assert_eq!(spec.epiphant.body, EpiphantConfig::default_preview().body);
		}
	}

	#[test]
	fn ward_is_smaller_than_encounter() {
		let encounter = BikanwardaSpec::encounter().uniform_scale();
		let ward = BikanwardaSpec::ward().uniform_scale();
		assert!(ward < encounter);
		assert!((ward / encounter - 0.85).abs() < 1e-4);
	}
}
