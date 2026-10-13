//! Well-known forest layerings ([RFC-183 §3.5.4]). Ground cover is omitted.
//! Missing groves (Conifer Lower Massives) are dropped, not aliased.

use super::{ForestGroveKind, ForestLayering, LayeringKind, WeightedGrove};

const fn w(kind: Option<ForestGroveKind>, weight: f32) -> WeightedGrove {
	WeightedGrove { kind, weight }
}

const fn grove(kind: ForestGroveKind, weight: f32) -> WeightedGrove {
	w(Some(kind), weight)
}

const fn none(weight: f32) -> WeightedGrove {
	w(None, weight)
}

/// Lush Jungle ([RFC-183 §3.5.4.1]).
pub fn lush_jungle() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::LushJungle,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::TallGrass, 1.0),
			grove(ForestGroveKind::WildGrass, 1.0),
			grove(ForestGroveKind::TropicalTufts, 1.0),
		],
		understory: vec![
			none(1.0),
			grove(ForestGroveKind::BraidGrass, 0.5),
			grove(ForestGroveKind::MonsterGrass, 0.1),
			grove(ForestGroveKind::TropicalUndergrowth, 1.0),
			grove(ForestGroveKind::TropicalThicket, 1.0),
			grove(ForestGroveKind::SpottyBushes, 1.0),
		],
		lower_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::UnendingJungle, 2.0),
			grove(ForestGroveKind::Shamanhome, 0.5),
			grove(ForestGroveKind::JungleLowerMassives, 0.2),
		],
		upper_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::TradeWinds, 4.0),
			grove(ForestGroveKind::PalmShade, 2.0),
			grove(ForestGroveKind::RiparianGeneral, 2.0),
			grove(ForestGroveKind::Leeward, 1.0),
			grove(ForestGroveKind::JungleMassives, 0.2),
		],
	}
}

/// Riparian ([RFC-183 §3.5.4.2]).
pub fn riparian() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Riparian,
		tufts: vec![
			none(1.5),
			grove(ForestGroveKind::TallGrass, 1.0),
			grove(ForestGroveKind::WildGrass, 1.0),
			grove(ForestGroveKind::CommonTufts, 0.5),
		],
		understory: vec![
			none(1.5),
			grove(ForestGroveKind::RiverineGreen, 2.0),
			grove(ForestGroveKind::LowBush, 0.75),
			grove(ForestGroveKind::HighBush, 0.5),
			grove(ForestGroveKind::SpottyBushes, 0.5),
		],
		lower_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::GoettingenFollow, 1.0),
			grove(ForestGroveKind::UnendingJungle, 0.35),
			grove(ForestGroveKind::StrangeOasis, 0.25),
		],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::RiparianGeneral, 2.0),
			grove(ForestGroveKind::RiparianMix, 1.5),
			grove(ForestGroveKind::PalmShade, 0.35),
			grove(ForestGroveKind::RollingOaks, 0.35),
		],
	}
}

/// Taiga ([RFC-183 §3.5.4.3]).
pub fn taiga() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Taiga,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::CommonTufts, 1.0),
			grove(ForestGroveKind::TallGrass, 0.75),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![
			none(5.0),
			grove(ForestGroveKind::JerrysChaparral, 0.5),
			grove(ForestGroveKind::SpottyBushes, 0.5),
			grove(ForestGroveKind::LowBush, 0.35),
		],
		lower_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::ConiferSapling, 1.5),
			grove(ForestGroveKind::AridConiferSapling, 0.75),
		],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::ChristmasTaiga, 2.0),
			grove(ForestGroveKind::Alpine, 1.5),
			grove(ForestGroveKind::ConiferMassives, 0.01),
			grove(ForestGroveKind::Dryland, 0.25),
		],
	}
}

/// Liam's Summer ([RFC-183 §3.5.4.4]).
pub fn liams_summer() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::LiamsSummer,
		tufts: vec![
			none(1.0),
			grove(ForestGroveKind::WildGrass, 1.5),
			grove(ForestGroveKind::TropicalTufts, 1.5),
			grove(ForestGroveKind::TallGrass, 0.75),
		],
		understory: vec![
			none(0.75),
			grove(ForestGroveKind::MonsterGrass, 5.0),
			grove(ForestGroveKind::BraidGrass, 1.5),
		],
		lower_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::UnendingJungle, 0.75),
			grove(ForestGroveKind::StrangeOasis, 0.5),
		],
		upper_canopy: vec![
			none(3.0),
			grove(ForestGroveKind::PalmShade, 1.5),
			grove(ForestGroveKind::ForlornSavanna, 0.5),
			grove(ForestGroveKind::WanderingAcacia, 0.5),
			grove(ForestGroveKind::JungleMassives, 0.1),
			grove(ForestGroveKind::TemperateMassives, 0.1),
		],
	}
}

/// Owl's Desert ([RFC-183 §3.5.4.5]).
pub fn owls_desert() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::OwlsDesert,
		tufts: vec![
			none(16.0),
			grove(ForestGroveKind::BushScrub, 1.0),
			grove(ForestGroveKind::WildGrass, 0.35),
			grove(ForestGroveKind::CommonTufts, 0.25),
		],
		understory: vec![
			none(16.0),
			grove(ForestGroveKind::LevantineScrub, 1.0),
			grove(ForestGroveKind::SpottyBushes, 0.5),
			grove(ForestGroveKind::LowBush, 0.25),
		],
		lower_canopy: vec![
			none(32.0),
			grove(ForestGroveKind::StrangeOasis, 1.0),
			grove(ForestGroveKind::AridConiferSapling, 0.35),
		],
		upper_canopy: vec![
			none(8.0),
			grove(ForestGroveKind::WanderingAcacia, 1.0),
			grove(ForestGroveKind::Dryland, 0.5),
			grove(ForestGroveKind::PalmShade, 0.25),
		],
	}
}

/// Mi Robles ([RFC-183 §3.5.4.6]).
pub fn mi_robles() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::MiRobles,
		tufts: vec![
			none(3.0),
			grove(ForestGroveKind::CommonTufts, 0.75),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![
			none(5.0),
			grove(ForestGroveKind::LowBush, 0.5),
			grove(ForestGroveKind::SpottyBushes, 0.5),
			grove(ForestGroveKind::RiverineGreen, 0.5),
		],
		lower_canopy: vec![none(5.0), grove(ForestGroveKind::GoettingenFollow, 0.5)],
		upper_canopy: vec![none(1.5), grove(ForestGroveKind::RollingOaks, 3.0)],
	}
}

/// Seceda ([RFC-183 §3.5.4.7]).
pub fn seceda() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Seceda,
		tufts: vec![
			none(2.5),
			grove(ForestGroveKind::CommonTufts, 1.0),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![
			none(1.5),
			grove(ForestGroveKind::JerrysChaparral, 2.0),
			grove(ForestGroveKind::HighBush, 2.0),
		],
		lower_canopy: vec![none(1.5), grove(ForestGroveKind::AridConiferSapling, 2.0)],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::Alpine, 2.0),
			grove(ForestGroveKind::Dryland, 0.35),
			grove(ForestGroveKind::ChristmasTaiga, 0.35),
			grove(ForestGroveKind::ConiferMassives, 0.25),
		],
	}
}

/// Kumulipo ([RFC-183 §3.5.4.8]).
pub fn kumulipo() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Kumulipo,
		tufts: vec![
			none(5.0),
			grove(ForestGroveKind::TropicalTufts, 1.0),
			grove(ForestGroveKind::WildGrass, 1.0),
			grove(ForestGroveKind::CommonTufts, 0.5),
		],
		understory: vec![
			none(5.0),
			grove(ForestGroveKind::TropicalUndergrowth, 0.5),
			grove(ForestGroveKind::LowBush, 0.25),
		],
		lower_canopy: vec![none(1.0), grove(ForestGroveKind::Shamanhome, 2.0)],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::PalmShade, 2.0),
			grove(ForestGroveKind::TradeWinds, 0.35),
			grove(ForestGroveKind::Leeward, 0.35),
			grove(ForestGroveKind::WanderingAcacia, 0.25),
			grove(ForestGroveKind::JungleMassives, 0.20),
		],
	}
}

/// Waiguo ([RFC-183 §3.5.4.9]).
pub fn waiguo() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Waiguo,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::CommonTufts, 1.0),
			grove(ForestGroveKind::TallGrass, 0.5),
		],
		understory: vec![
			none(1.5),
			grove(ForestGroveKind::BraidGrass, 2.0),
			grove(ForestGroveKind::TropicalThicket, 0.75),
		],
		lower_canopy: vec![none(4.0), grove(ForestGroveKind::GoettingenFollow, 0.5)],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::Orchard, 2.0),
			grove(ForestGroveKind::Vineyard, 2.0),
			grove(ForestGroveKind::DateGrove, 2.0),
			grove(ForestGroveKind::TemperateMassives, 0.20),
		],
	}
}

/// Ag Town ([RFC-183 §3.5.4.10]).
pub fn ag_town() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::AgTown,
		tufts: vec![none(8.0), grove(ForestGroveKind::CommonTufts, 0.5)],
		understory: vec![
			none(8.0),
			grove(ForestGroveKind::LowBush, 0.35),
			grove(ForestGroveKind::BraidGrass, 0.25),
		],
		lower_canopy: vec![none(8.0)],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::Orchard, 2.0),
			grove(ForestGroveKind::Vineyard, 2.0),
			grove(ForestGroveKind::DateGrove, 2.0),
		],
	}
}

/// Sun's Barren ([RFC-183 §3.5.4.11]).
pub fn suns_barren() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::SunsBarren,
		tufts: vec![none(64.0), grove(ForestGroveKind::CommonTufts, 0.25)],
		understory: vec![none(12.0)],
		lower_canopy: vec![none(12.0)],
		upper_canopy: vec![none(12.0)],
	}
}

/// Temperate Holy ([RFC-183 §3.5.4.12]).
pub fn temperate_holy() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::TemperateHoly,
		tufts: vec![none(8.0), grove(ForestGroveKind::CommonTufts, 0.25)],
		understory: vec![none(8.0), grove(ForestGroveKind::LowBush, 0.25)],
		lower_canopy: vec![none(1.0), grove(ForestGroveKind::TemperateLowerMassives, 2.0)],
		upper_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::TemperateMassives, 1.0),
			grove(ForestGroveKind::ConiferMassives, 0.25),
		],
	}
}

/// Old Steppe ([RFC-183 §3.5.4.13]).
pub fn old_steppe() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::OldSteppe,
		tufts: vec![
			none(4.0),
			grove(ForestGroveKind::TallGrass, 0.75),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![none(8.0)],
		lower_canopy: vec![none(8.0), grove(ForestGroveKind::ConiferSapling, 0.35)],
		upper_canopy: vec![none(10.0)],
	}
}

/// Trap Thicket ([RFC-183 §3.5.4.14]).
pub fn trap_thicket() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::TrapThicket,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::TropicalTufts, 1.0),
			grove(ForestGroveKind::WildGrass, 0.75),
		],
		understory: vec![
			none(0.75),
			grove(ForestGroveKind::MonsterGrass, 1.5),
			grove(ForestGroveKind::BraidGrass, 1.5),
			grove(ForestGroveKind::TropicalThicket, 1.5),
			grove(ForestGroveKind::TropicalUndergrowth, 1.5),
		],
		lower_canopy: vec![none(1.0), grove(ForestGroveKind::UnendingJungle, 2.5)],
		upper_canopy: vec![
			none(2.0),
			grove(ForestGroveKind::TradeWinds, 0.75),
			grove(ForestGroveKind::JungleMassives, 0.20),
		],
	}
}

/// Bush ([RFC-183 §3.5.4.15]).
pub fn bush() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Bush,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::BushScrub, 1.0),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![
			none(1.0),
			grove(ForestGroveKind::LowBush, 2.0),
			grove(ForestGroveKind::HighBush, 2.0),
			grove(ForestGroveKind::BraidGrass, 0.75),
			grove(ForestGroveKind::MonsterGrass, 0.5),
		],
		lower_canopy: vec![none(2.5), grove(ForestGroveKind::GoettingenFollow, 0.75)],
		upper_canopy: vec![
			none(1.5),
			grove(ForestGroveKind::ForlornSavanna, 1.5),
			grove(ForestGroveKind::WanderingAcacia, 1.5),
			grove(ForestGroveKind::Storytellers, 0.20),
		],
	}
}

/// Old Nevada ([RFC-183 §3.5.4.16]).
pub fn old_nevada() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::OldNevada,
		tufts: vec![
			none(5.0),
			grove(ForestGroveKind::BushScrub, 0.5),
			grove(ForestGroveKind::WildGrass, 0.35),
		],
		understory: vec![none(8.0), grove(ForestGroveKind::SpottyBushes, 0.35)],
		lower_canopy: vec![none(1.5), grove(ForestGroveKind::AridConiferSapling, 2.0)],
		upper_canopy: vec![none(7.0), grove(ForestGroveKind::Dryland, 0.35)],
	}
}

/// Storybook ([RFC-183 §3.5.4.17]).
pub fn storybook() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Storybook,
		tufts: vec![
			none(2.0),
			grove(ForestGroveKind::WildGrass, 1.0),
			grove(ForestGroveKind::CommonTufts, 0.75),
		],
		understory: vec![
			none(2.0),
			grove(ForestGroveKind::RiverineGreen, 0.75),
			grove(ForestGroveKind::LowBush, 0.25),
			grove(ForestGroveKind::HighBush, 0.25),
		],
		lower_canopy: vec![
			none(3.0),
			grove(ForestGroveKind::GoettingenFollow, 0.5),
			grove(ForestGroveKind::UnendingJungle, 0.25),
		],
		upper_canopy: vec![
			none(1.0),
			grove(ForestGroveKind::RiparianMix, 2.0),
			grove(ForestGroveKind::Storytellers, 0.75),
			grove(ForestGroveKind::Leeward, 0.75),
			grove(ForestGroveKind::TradeWinds, 0.75),
		],
	}
}

/// Meadowland ([RFC-183 §3.5.4.18]).
pub fn meadowland() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::Meadowland,
		tufts: vec![
			none(3.0),
			grove(ForestGroveKind::WildGrass, 1.0),
			grove(ForestGroveKind::TallGrass, 0.75),
		],
		understory: vec![none(8.0), grove(ForestGroveKind::LowBush, 0.25)],
		lower_canopy: vec![none(9.0)],
		upper_canopy: vec![
			none(8.0),
			grove(ForestGroveKind::RiparianMix, 0.5),
			grove(ForestGroveKind::RollingOaks, 0.5),
			grove(ForestGroveKind::TemperateMassives, 0.15),
			grove(ForestGroveKind::ConiferMassives, 0.10),
		],
	}
}

/// Fruit Plains ([RFC-183 §3.5.4.19]).
pub fn fruit_plains() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::FruitPlains,
		tufts: vec![
			none(4.0),
			grove(ForestGroveKind::CommonTufts, 0.75),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![
			none(6.0),
			grove(ForestGroveKind::BraidGrass, 0.5),
			grove(ForestGroveKind::LowBush, 0.25),
		],
		lower_canopy: vec![none(8.0)],
		upper_canopy: vec![
			none(3.0),
			grove(ForestGroveKind::RollingOaks, 0.75),
			grove(ForestGroveKind::Orchard, 0.75),
			grove(ForestGroveKind::Vineyard, 0.75),
			grove(ForestGroveKind::DateGrove, 0.75),
		],
	}
}

/// Damas Edge ([RFC-183 §3.5.4.20]).
pub fn damas_edge() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::DamasEdge,
		tufts: vec![
			none(5.0),
			grove(ForestGroveKind::BushScrub, 0.75),
			grove(ForestGroveKind::TropicalTufts, 0.35),
		],
		understory: vec![
			none(4.0),
			grove(ForestGroveKind::LevantineScrub, 1.0),
			grove(ForestGroveKind::TropicalUndergrowth, 0.35),
			grove(ForestGroveKind::TropicalThicket, 0.35),
		],
		lower_canopy: vec![
			none(5.0),
			grove(ForestGroveKind::StrangeOasis, 0.35),
			grove(ForestGroveKind::UnendingJungle, 0.35),
		],
		upper_canopy: vec![
			none(4.0),
			grove(ForestGroveKind::Orchard, 0.5),
			grove(ForestGroveKind::DateGrove, 1.0),
			grove(ForestGroveKind::PalmShade, 0.35),
		],
	}
}

/// Open Tropics ([RFC-183 §3.5.4.21]).
pub fn open_tropics() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::OpenTropics,
		tufts: vec![
			none(4.0),
			grove(ForestGroveKind::TropicalTufts, 0.75),
			grove(ForestGroveKind::WildGrass, 0.5),
		],
		understory: vec![none(6.0), grove(ForestGroveKind::TropicalUndergrowth, 0.5)],
		lower_canopy: vec![none(7.0), grove(ForestGroveKind::UnendingJungle, 0.35)],
		upper_canopy: vec![none(3.0), grove(ForestGroveKind::TradeWinds, 1.0)],
	}
}

/// West Maui ([RFC-183 §3.5.4.22]).
pub fn west_maui() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::WestMaui,
		tufts: vec![
			none(1.0),
			grove(ForestGroveKind::WildGrass, 2.0),
			grove(ForestGroveKind::BushScrub, 2.0),
			grove(ForestGroveKind::TropicalTufts, 2.0),
		],
		understory: vec![none(5.0)],
		lower_canopy: vec![none(8.0)],
		upper_canopy: vec![none(3.0), grove(ForestGroveKind::WanderingAcacia, 1.0)],
	}
}

/// Upper Park ([RFC-183 §3.5.4.23]).
pub fn upper_park() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::UpperPark,
		tufts: vec![
			none(1.0),
			grove(ForestGroveKind::WildGrass, 2.0),
			grove(ForestGroveKind::BushScrub, 2.0),
			grove(ForestGroveKind::CommonTufts, 0.5),
		],
		understory: vec![
			none(6.0),
			grove(ForestGroveKind::LowBush, 0.35),
			grove(ForestGroveKind::SpottyBushes, 0.35),
		],
		lower_canopy: vec![none(8.0)],
		upper_canopy: vec![none(3.0), grove(ForestGroveKind::RollingOaks, 1.0)],
	}
}

/// Steppe Down ([RFC-183 §3.5.4.24]).
pub fn steppe_down() -> ForestLayering {
	ForestLayering {
		kind: LayeringKind::SteppeDown,
		tufts: vec![
			none(1.0),
			grove(ForestGroveKind::WildGrass, 2.0),
			grove(ForestGroveKind::BushScrub, 2.0),
		],
		understory: vec![none(8.0)],
		lower_canopy: vec![none(10.0)],
		upper_canopy: vec![none(10.0)],
	}
}

impl LayeringKind {
	pub fn layering(self) -> ForestLayering {
		match self {
			Self::LushJungle => lush_jungle(),
			Self::Riparian => riparian(),
			Self::Taiga => taiga(),
			Self::LiamsSummer => liams_summer(),
			Self::OwlsDesert => owls_desert(),
			Self::MiRobles => mi_robles(),
			Self::Seceda => seceda(),
			Self::Kumulipo => kumulipo(),
			Self::Waiguo => waiguo(),
			Self::AgTown => ag_town(),
			Self::SunsBarren => suns_barren(),
			Self::TemperateHoly => temperate_holy(),
			Self::OldSteppe => old_steppe(),
			Self::TrapThicket => trap_thicket(),
			Self::Bush => bush(),
			Self::OldNevada => old_nevada(),
			Self::Storybook => storybook(),
			Self::Meadowland => meadowland(),
			Self::FruitPlains => fruit_plains(),
			Self::DamasEdge => damas_edge(),
			Self::OpenTropics => open_tropics(),
			Self::WestMaui => west_maui(),
			Self::UpperPark => upper_park(),
			Self::SteppeDown => steppe_down(),
		}
	}
}
