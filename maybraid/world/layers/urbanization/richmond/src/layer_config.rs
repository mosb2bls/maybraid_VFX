//! Per-mode knobs for [`crate::Richmond`].

use bevy::prelude::*;
use procedural_common::NoiseParams;
use urbanization_cells::{
	UrbanizationKind, UrbanizationSelection, DEFAULT_URBANIZATION_EXTENT_XZ,
	DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

use crate::config::{DevelopmentConfig, DevelopmentSites};

/// Occupancy fill used by both the world and the developments playground.
pub const PLAYGROUND_LIKELIHOOD: f32 = 0.9;

/// Default present ring multiplier (`1` → 1 km present / 3 km generate).
pub const DEFAULT_URBANIZATION_STREAM_RADIUS: u32 = 1;

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_URBANIZATION_EXTENT_XZ, DEFAULT_URBANIZATION_EXTENT_XZ * 2.0);
	}
	let present = DEVELOPMENT_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (DEVELOPMENT_GENERATE_RADIUS_M - DEVELOPMENT_PRESENT_RADIUS_M))
}

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_URBANIZATION_NOISE: &str = "1337,0.0005,1,1";

/// Exclusive development-archetype focus (playground `/focus-development`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentFocus {
	All,
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

impl DevelopmentFocus {
	pub fn apply(self, config: &mut DevelopmentConfig) {
		let selected = if self == Self::All { None } else { Some(self) };
		config.les_halles_weight = weight(selected, Self::LesHalles);
		config.shepherds_village_weight = weight(selected, Self::ShepherdsVillage);
		config.shepherds_commune_weight = weight(selected, Self::ShepherdsCommune);
		config.ring_fort_weight = weight(selected, Self::RingFort);
		config.temple_complex_weight = weight(selected, Self::TempleComplex);
		config.single_highrise_weight = weight(selected, Self::SingleHighrise);
		config.suburban_homes_weight = weight(selected, Self::SuburbanHomes);
		config.wizards_tower_weight = weight(selected, Self::WizardsTower);
		config.skybridge_bazaar_weight = weight(selected, Self::SkybridgeBazaar);
		config.old_city_market_weight = weight(selected, Self::OldCityMarket);
	}

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::All => "all",
			Self::LesHalles => "les-halles",
			Self::ShepherdsVillage => "shepherds-village",
			Self::ShepherdsCommune => "shepherds-commune",
			Self::RingFort => "ring-fort",
			Self::TempleComplex => "temple-complex",
			Self::SingleHighrise => "single-highrise",
			Self::SuburbanHomes => "suburban-homes",
			Self::WizardsTower => "wizards-tower",
			Self::SkybridgeBazaar => "skybridge-bazaar",
			Self::OldCityMarket => "old-city-market",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		Some(match name.trim().to_ascii_lowercase().as_str() {
			"all" => Self::All,
			"les-halles" => Self::LesHalles,
			"shepherds-village" => Self::ShepherdsVillage,
			"shepherds-commune" => Self::ShepherdsCommune,
			"ring-fort" => Self::RingFort,
			"temple-complex" => Self::TempleComplex,
			"single-highrise" => Self::SingleHighrise,
			"suburban-homes" => Self::SuburbanHomes,
			"wizards-tower" => Self::WizardsTower,
			"skybridge-bazaar" => Self::SkybridgeBazaar,
			"old-city-market" => Self::OldCityMarket,
			_ => return None,
		})
	}
}

impl std::fmt::Display for DevelopmentFocus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_kebab())
	}
}

fn weight(selected: Option<DevelopmentFocus>, kind: DevelopmentFocus) -> f32 {
	match selected {
		None => 1.0,
		Some(selected) if selected == kind => 1.0,
		Some(_) => 0.0,
	}
}

/// Live urbanization-stream knobs (noise / ring / pinned kind).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UrbanizationStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub kind: Option<UrbanizationKind>,
}

impl Default for UrbanizationStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_URBANIZATION_STREAM_RADIUS,
			kind: None,
		}
	}
}

impl UrbanizationStreamSpec {
	pub fn key(self) -> String {
		let kind_key = self.kind.map(UrbanizationKind::as_kebab).unwrap_or("hopscotch");
		format!("urbanization:{kind_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// Stream spec, focus pins, and the urbanization generate budget.
///
/// `world_defaults()` is 16 (assembled world). [`Default`] is 8 (standalone
/// playground), matching the table in [#883](https://github.com/ramate-io/maybraid/issues/883).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct RichmondConfig {
	pub urbanization: Option<UrbanizationStreamSpec>,
	pub focus_urbanization: Option<UrbanizationKind>,
	pub focus_development: Option<DevelopmentFocus>,
	pub generate_budget: u32,
}

impl Default for RichmondConfig {
	fn default() -> Self {
		Self {
			urbanization: None,
			focus_urbanization: None,
			focus_development: None,
			generate_budget: 8,
		}
	}
}

impl RichmondConfig {
	/// Hopscotch at 1 km / 3 km rings, generate budget 16.
	pub fn world_defaults() -> Self {
		Self {
			urbanization: Some(UrbanizationStreamSpec::default()),
			focus_urbanization: None,
			focus_development: None,
			generate_budget: 16,
		}
	}

	pub fn development_config(&self) -> DevelopmentConfig {
		// Urbanization leaves under a stream or pinned urbanization; a
		// development focus then only changes kind weights. The 300 m lattice is
		// the no-stream catalog path. Without either the world is hopscotch-off
		// and only authored developments stand.
		let sites = if self.urbanization.is_some() || self.focus_urbanization.is_some() {
			DevelopmentSites::Urbanization
		} else if self.focus_development.is_some() {
			DevelopmentSites::Lattice
		} else {
			DevelopmentSites::Authored
		};
		let mut development = DevelopmentConfig {
			likelihood: PLAYGROUND_LIKELIHOOD,
			sites,
			..DevelopmentConfig::from_world_seed(42)
		};
		if let Some(focus) = self.focus_development {
			focus.apply(&mut development);
		}
		if let Some(spec) = self.urbanization {
			development.seed = spec.noise.seed.max(0) as u32;
		}
		development
	}

	/// Urbanization the developments sit on: the development seed's noise,
	/// pinned to the focused kind.
	pub fn urbanization_selection(&self) -> UrbanizationSelection {
		UrbanizationSelection {
			noise: self.development_config().urbanization_noise(),
			kind: focused_spec(self).and_then(|spec| spec.kind).or(self.focus_urbanization),
		}
	}
}

/// Spec the stream and pin write: `focus_urbanization` fills an open kind.
pub fn focused_spec(config: &RichmondConfig) -> Option<UrbanizationStreamSpec> {
	config.urbanization.map(|mut spec| {
		spec.kind = spec.kind.or(config.focus_urbanization);
		spec
	})
}
