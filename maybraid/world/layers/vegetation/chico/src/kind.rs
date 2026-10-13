//! Grove and layering identities that exist in `chico-groves`.
//!
//! Ground-cover groves and Conifer Lower Massives are not listed — they are not
//! implemented and must not be aliased onto another grove.

/// Existing well-known grove a forest layer may select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForestGroveKind {
	Alpine,
	AridConiferSapling,
	BraidGrass,
	BushScrub,
	ChristmasTaiga,
	CommonTufts,
	ConiferMassives,
	ConiferSapling,
	DateGrove,
	Dryland,
	ForlornSavanna,
	GoettingenFollow,
	HighBush,
	JerrysChaparral,
	JungleLowerMassives,
	JungleMassives,
	Leeward,
	LevantineScrub,
	LowBush,
	MonsterGrass,
	Orchard,
	PalmShade,
	RiparianGeneral,
	RiparianMix,
	RiverineGreen,
	RollingOaks,
	Shamanhome,
	SpottyBushes,
	Storytellers,
	StrangeOasis,
	TallGrass,
	TemperateLowerMassives,
	TemperateMassives,
	TradeWinds,
	TropicalThicket,
	TropicalTufts,
	TropicalUndergrowth,
	UnendingJungle,
	Vineyard,
	WanderingAcacia,
	WildGrass,
}

/// Well-known forest layering ([RFC-183 §3.5.4]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayeringKind {
	LushJungle,
	Riparian,
	Taiga,
	LiamsSummer,
	OwlsDesert,
	MiRobles,
	Seceda,
	Kumulipo,
	Waiguo,
	AgTown,
	SunsBarren,
	TemperateHoly,
	OldSteppe,
	TrapThicket,
	Bush,
	OldNevada,
	Storybook,
	Meadowland,
	FruitPlains,
	DamasEdge,
	OpenTropics,
	WestMaui,
	UpperPark,
	SteppeDown,
}

/// One layer bucket: a grove or explicit `None`, plus throw weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedGrove {
	pub kind: Option<ForestGroveKind>,
	pub weight: f32,
}

/// Four forest layers (no ground cover). Each is a Bucket Throw including `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ForestLayering {
	pub kind: LayeringKind,
	pub tufts: Vec<WeightedGrove>,
	pub understory: Vec<WeightedGrove>,
	pub lower_canopy: Vec<WeightedGrove>,
	pub upper_canopy: Vec<WeightedGrove>,
}

/// Groves selected for one forest cell (one per layer, or `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedLayers {
	pub layering: LayeringKind,
	pub tufts: Option<ForestGroveKind>,
	pub understory: Option<ForestGroveKind>,
	pub lower_canopy: Option<ForestGroveKind>,
	pub upper_canopy: Option<ForestGroveKind>,
}

/// One forest vegetation layer. A 100 m tile may store one [`crate::ChicoGrove`] per layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForestLayer {
	Tufts,
	Understory,
	LowerCanopy,
	UpperCanopy,
}

impl ForestLayer {
	pub const ALL: [Self; 4] =
		[Self::Tufts, Self::Understory, Self::LowerCanopy, Self::UpperCanopy];

	pub fn kind(self, layers: SelectedLayers) -> Option<ForestGroveKind> {
		match self {
			Self::Tufts => layers.tufts,
			Self::Understory => layers.understory,
			Self::LowerCanopy => layers.lower_canopy,
			Self::UpperCanopy => layers.upper_canopy,
		}
	}

	/// Origin-cell Y used to distinguish stacked grove ids on the same 100 m tile.
	pub fn id_y(self) -> f32 {
		match self {
			Self::Tufts => 0.0,
			Self::Understory => 1.0,
			Self::LowerCanopy => 2.0,
			Self::UpperCanopy => 3.0,
		}
	}

	pub fn from_id_y(y: f32) -> Option<Self> {
		match y.floor() as i32 {
			0 => Some(Self::Tufts),
			1 => Some(Self::Understory),
			2 => Some(Self::LowerCanopy),
			3 => Some(Self::UpperCanopy),
			_ => None,
		}
	}

	/// Present-layer dropout for the tufts bucket ([#652](https://github.com/ramate-io/maybraid/issues/652)).
	///
	/// Blade groves on other layers use [`LayerDropOut::for_stacked`].
	pub fn drop_out(self) -> LayerDropOut {
		LayerDropOut::for_stacked(self, false)
	}
}

/// World-metre height below which tuft plants are omitted on High.
pub const TUFT_DROP_MIN_HEIGHT_M: f32 = 0.5;

/// When a forest-layer host stops emitting kits, and a High size floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerDropOut {
	/// This band and farther (`<=` in [`lod::LodSceneLevel`] order) emit nothing.
	pub empty_from: Option<lod::LodSceneLevel>,
	/// Drop High plants shorter than this (0 = keep all).
	pub min_height_m: f32,
}

impl LayerDropOut {
	pub const fn none() -> Self {
		Self { empty_from: None, min_height_m: 0.0 }
	}

	/// Tufts layer, or a tuft-typed tile on any layer (Monster / Braid understory).
	pub fn for_stacked(layer: ForestLayer, tuft_tile: bool) -> Self {
		if layer == ForestLayer::Tufts || tuft_tile {
			Self {
				empty_from: Some(lod::LodSceneLevel::Medium),
				min_height_m: TUFT_DROP_MIN_HEIGHT_M,
			}
		} else {
			Self::none()
		}
	}

	pub fn omits(self, level: lod::LodSceneLevel) -> bool {
		self.empty_from.is_some_and(|from| level <= from)
	}
}

impl LayeringKind {
	pub const ALL: &[Self] = &[
		Self::LushJungle,
		Self::Riparian,
		Self::Taiga,
		Self::LiamsSummer,
		Self::OwlsDesert,
		Self::MiRobles,
		Self::Seceda,
		Self::Kumulipo,
		Self::Waiguo,
		Self::AgTown,
		Self::SunsBarren,
		Self::TemperateHoly,
		Self::OldSteppe,
		Self::TrapThicket,
		Self::Bush,
		Self::OldNevada,
		Self::Storybook,
		Self::Meadowland,
		Self::FruitPlains,
		Self::DamasEdge,
		Self::OpenTropics,
		Self::WestMaui,
		Self::UpperPark,
		Self::SteppeDown,
	];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::LushJungle => "lush-jungle",
			Self::Riparian => "riparian",
			Self::Taiga => "taiga",
			Self::LiamsSummer => "liams-summer",
			Self::OwlsDesert => "owls-desert",
			Self::MiRobles => "mi-robles",
			Self::Seceda => "seceda",
			Self::Kumulipo => "kumulipo",
			Self::Waiguo => "waiguo",
			Self::AgTown => "ag-town",
			Self::SunsBarren => "suns-barren",
			Self::TemperateHoly => "temperate-holy",
			Self::OldSteppe => "old-steppe",
			Self::TrapThicket => "trap-thicket",
			Self::Bush => "bush",
			Self::OldNevada => "old-nevada",
			Self::Storybook => "storybook",
			Self::Meadowland => "meadowland",
			Self::FruitPlains => "fruit-plains",
			Self::DamasEdge => "damas-edge",
			Self::OpenTropics => "open-tropics",
			Self::WestMaui => "west-maui",
			Self::UpperPark => "upper-park",
			Self::SteppeDown => "steppe-down",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}
}

impl ForestLayering {
	/// Highest-weight non-`None` grove on each layer (review / pinned cells).
	pub fn typical_layers(&self) -> SelectedLayers {
		SelectedLayers {
			layering: self.kind,
			tufts: typical_grove(&self.tufts),
			understory: typical_grove(&self.understory),
			lower_canopy: typical_grove(&self.lower_canopy),
			upper_canopy: typical_grove(&self.upper_canopy),
		}
	}
}

fn typical_grove(buckets: &[WeightedGrove]) -> Option<ForestGroveKind> {
	buckets
		.iter()
		.filter_map(|bucket| Some((bucket.kind?, bucket.weight)))
		.max_by(|a, b| a.1.total_cmp(&b.1))
		.map(|(kind, _)| kind)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn kebab_round_trips_every_layering() -> Result<()> {
		assert_eq!(LayeringKind::ALL.len(), 24);
		for kind in LayeringKind::ALL {
			let name = kind.as_kebab();
			assert_eq!(LayeringKind::from_kebab(name), Some(*kind));
		}
		assert!(LayeringKind::from_kebab("not-a-layering").is_none());
		Ok(())
	}

	#[test]
	fn typical_lush_jungle_keeps_canopy() -> Result<()> {
		let layers = LayeringKind::LushJungle.layering().typical_layers();
		assert_eq!(layers.upper_canopy, Some(ForestGroveKind::TradeWinds));
		assert!(layers.tufts.is_some());
		Ok(())
	}

	#[test]
	fn stacked_dropout_covers_understory_tuft_tiles() -> Result<()> {
		assert!(LayerDropOut::for_stacked(ForestLayer::Understory, true)
			.omits(lod::LodSceneLevel::Medium));
		assert!(!LayerDropOut::for_stacked(ForestLayer::Understory, false)
			.omits(lod::LodSceneLevel::Medium));
		assert!(
			!LayerDropOut::for_stacked(ForestLayer::Tufts, false).omits(lod::LodSceneLevel::High)
		);
		Ok(())
	}
}
