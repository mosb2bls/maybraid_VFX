//! Urbanization kinds and per-kind development recipes.

/// Development kind selected for a guillotine leaf (mirrors Richmond recipes).
///
/// Kept local to this crate so urbanization does not depend on
/// `richmond-development-models`. Convert at the models boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UrbanDevelopmentKind {
	Empty,
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
}

/// Well-known urbanization layer for a 1600 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UrbanizationKind {
	None,
	MixedAgeCity,
	ModernCity,
	RuralLife,
	Townships,
	Frontier,
	Colony,
}

/// One development bucket: an [`UrbanDevelopmentKind`] plus throw weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedDevelopment {
	pub kind: UrbanDevelopmentKind,
	pub weight: f32,
}

/// Bucket-Throw recipe for one urbanization kind.
#[derive(Debug, Clone, PartialEq)]
pub struct UrbanizationRecipe {
	pub kind: UrbanizationKind,
	pub developments: Vec<WeightedDevelopment>,
}

impl UrbanizationKind {
	pub const ALL: &[Self] = &[
		Self::None,
		Self::MixedAgeCity,
		Self::ModernCity,
		Self::RuralLife,
		Self::Townships,
		Self::Frontier,
		Self::Colony,
	];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::MixedAgeCity => "mixed-age-city",
			Self::ModernCity => "modern-city",
			Self::RuralLife => "rural-life",
			Self::Townships => "townships",
			Self::Frontier => "frontier",
			Self::Colony => "colony",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}

	/// Relative-weight recipe used when filling guillotine leaves.
	///
	/// [`UrbanizationKind::None`] returns an empty recipe (selection short-circuits
	/// before guillotine; the recipe is unused).
	pub fn recipe(self) -> UrbanizationRecipe {
		use UrbanDevelopmentKind::*;
		let developments = match self {
			Self::None => Vec::new(),
			Self::RuralLife => vec![
				w(Empty, 0.78),
				w(ShepherdsVillage, 0.10),
				w(ShepherdsCommune, 0.08),
				w(WizardsTower, 0.04),
			],
			Self::Townships => vec![
				w(Empty, 0.35),
				w(ShepherdsCommune, 0.22),
				w(SuburbanHomes, 0.18),
				w(OldCityMarket, 0.15),
				w(ShepherdsVillage, 0.07),
				w(LesHalles, 0.03),
			],
			Self::Frontier => vec![
				w(Empty, 0.45),
				w(ShepherdsVillage, 0.18),
				w(ShepherdsCommune, 0.12),
				w(RingFort, 0.20),
				w(LesHalles, 0.03),
			],
			Self::Colony => vec![
				w(Empty, 0.28),
				w(OldCityMarket, 0.18),
				w(TempleComplex, 0.14),
				w(SingleHighrise, 0.12),
				w(RingFort, 0.12),
				w(SuburbanHomes, 0.08),
				w(ShepherdsCommune, 0.05),
				w(LesHalles, 0.03),
			],
			Self::MixedAgeCity => vec![
				w(Empty, 0.18),
				w(OldCityMarket, 0.16),
				w(SuburbanHomes, 0.14),
				w(SingleHighrise, 0.14),
				w(TempleComplex, 0.12),
				w(LesHalles, 0.08),
				w(ShepherdsCommune, 0.08),
				w(SkybridgeBazaar, 0.06),
				w(RingFort, 0.04),
			],
			Self::ModernCity => vec![
				w(Empty, 0.22),
				w(SingleHighrise, 0.28),
				w(SuburbanHomes, 0.20),
				w(SkybridgeBazaar, 0.18),
				w(LesHalles, 0.08),
				w(TempleComplex, 0.04),
			],
		};
		UrbanizationRecipe { kind: self, developments }
	}
}

fn w(kind: UrbanDevelopmentKind, weight: f32) -> WeightedDevelopment {
	WeightedDevelopment { kind, weight }
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn kebab_round_trips_every_kind() -> Result<()> {
		assert_eq!(UrbanizationKind::ALL.len(), 7);
		for kind in UrbanizationKind::ALL {
			let name = kind.as_kebab();
			assert_eq!(UrbanizationKind::from_kebab(name), Some(*kind));
		}
		assert!(UrbanizationKind::from_kebab("not-a-kind").is_none());
		Ok(())
	}

	#[test]
	fn none_recipe_is_empty() -> Result<()> {
		assert!(UrbanizationKind::None.recipe().developments.is_empty());
		Ok(())
	}

	#[test]
	fn frontier_has_no_wizard() -> Result<()> {
		let recipe = UrbanizationKind::Frontier.recipe();
		assert!(recipe.developments.iter().all(|d| d.kind != UrbanDevelopmentKind::WizardsTower));
		Ok(())
	}
}
